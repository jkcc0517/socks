pub mod replies;
pub mod requests;
pub mod methods;
pub mod client;
pub mod udp;
use tokio::time::{sleep, Duration};
use serde::ser::{Serialize, Serializer};
// use serde::Serialize;
use log::{debug, error, info};
use std::net::SocketAddr;
use tokio::net::{TcpStream, ToSocketAddrs, UdpSocket, lookup_host};
use tokio::sync::mpsc;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use std::sync::Arc;
use udp::UdpMessage;
use crate::consts;
use std::array::TryFromSliceError;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use replies::SocksReply;
use requests::SocksRequest;
use anyhow::{Result, anyhow};

pub trait SocksMessage {
    fn deserialize_from_bytes(bytes: &[u8]) -> Self;
    fn serialize_to_bytes(&self) -> Vec<u8>;
}

#[derive(Debug, Clone)]
pub enum Socks5Command {
    TCPConnect,
    TCPBind,
    UDPAssociate,
}

#[allow(dead_code)]
impl Socks5Command {
    #[inline]
    #[rustfmt::skip]
    fn as_u8(&self) -> u8 {
        match self {
            Socks5Command::TCPConnect   => consts::SOCKS5_CMD_TCP_CONNECT,
            Socks5Command::TCPBind      => consts::SOCKS5_CMD_TCP_BIND,
            Socks5Command::UDPAssociate => consts::SOCKS5_CMD_UDP_ASSOCIATE,
        }
    }
}

impl From<u8> for Socks5Command {
    fn from(number: u8) -> Socks5Command {
        match number {
            consts::SOCKS5_CMD_TCP_CONNECT      => Socks5Command::TCPConnect,
            consts::SOCKS5_CMD_TCP_BIND         => Socks5Command::TCPBind,
            consts::SOCKS5_CMD_UDP_ASSOCIATE    => Socks5Command::UDPAssociate,
            _ => {
                panic!("run socks5 command");
            },
        }
    }
}

// 想要讓 u16 deserialize to bytes 的時候值是 Big Endian，並且在 display 的時候顯示正確的值
#[derive(Debug, Clone, Copy)]
struct SocksPort(u16);

impl SocksPort {
    pub fn new(port: u16) -> Self {
        SocksPort(port)
    }
    pub fn serialize_to_bytes(&self) -> Vec<u8> {
        self.0.to_be_bytes().to_vec()
    }
}
impl From<SocksPort> for u16 {
    fn from(val: SocksPort) -> Self {
        val.0
    }
}
#[derive(Debug, Clone)]
// #[serde(untagged)]
pub enum SocksAddress {
    IP(IpAddr),
    Domain(String),
}

impl SocksAddress {
    async fn get_ip_addr(&self) -> IpAddr {
        match self {
            SocksAddress::IP(ip_addr) => {
                *ip_addr
            },
            SocksAddress::Domain(domain) => {
                let mut ip_addr: Option<IpAddr> = None;
                let address = format!("{}:666", domain);
                for addr in lookup_host(address).await.unwrap() {
                    println!("socket address is {}", addr);
                    ip_addr = Some(addr.ip());
                    break;
                }
                match ip_addr {
                    Some(addr) => {
                        addr
                    },
                    _ => { panic!("can not resolve domain name."); }
                }
            }
        }
    }
    pub fn parse_destination_address(atyp: u8, data: &mut Vec<u8>) -> SocksAddress {
        match atyp {
            consts::SOCKS5_ADDR_TYPE_IPV4 => {
                let address: Vec<u8> = (1..=4).map(|_|
                    data.remove(0)
                ).collect();
                let address: Result<[u8; 4], TryFromSliceError> = address.as_slice().try_into() as Result<[u8; 4], TryFromSliceError>;
                SocksAddress::IP(IpAddr::V4(Ipv4Addr::from(address.unwrap())))
            },
            consts::SOCKS5_ADDR_TYPE_DOMAIN_NAME => {
                let number_of_name = data.remove(0);
                let domain: String = (1..=number_of_name).map(|_|
                    data.remove(0) as char
                ).collect();
                SocksAddress::Domain(domain)
            },
            consts::SOCKS5_ADDR_TYPE_IPV6 => {
                let address: Vec<u8> = (1..=16).map(|_|
                    data.remove(0)
                ).collect();
                let address: Result<[u8; 16], TryFromSliceError> = address.as_slice().try_into() as Result<[u8; 16], TryFromSliceError>;
                SocksAddress::IP(IpAddr::V6(Ipv6Addr::from(address.unwrap())))
            },
            _ => {
                debug!("{:?}", data);
                panic!("atyp {:?} parsed error!!", atyp);
            },
        }
    }
    pub fn serialize_to_bytes(&self) -> Vec<u8> {
        let s: Vec<u8> = match self {
            SocksAddress::IP(ip) => {
                match ip {
                    IpAddr::V4(ipv4) => {
                        ipv4.octets().to_vec()
                    },
                    IpAddr::V6(ipv6) => {
                        ipv6.octets().to_vec()
                    },
                }
            },
            SocksAddress::Domain(domain) => {
                todo!();
            }
        };
        s
    }
}

impl Serialize for SocksAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            SocksAddress::IP(ip) => {
                match ip {
                    IpAddr::V4(ipv4) => {
                        let u32_ip: u32 = u32::from_le_bytes(ipv4.octets());
                        serializer.serialize_u32(u32_ip)
                    },
                    IpAddr::V6(ipv6) => {
                        let u128_ip = u128::from_le_bytes(ipv6.octets());
                        serializer.serialize_u128(u128_ip)
                    },
                }
            },
            SocksAddress::Domain(domain) => {
                serializer.serialize_bytes(domain.as_bytes())
            }
        };
        s
    }
}

fn calculate_port_number(first: u8, second: u8) -> Option<u16> {
    let o: Vec<u8> = vec![first, second];
    if o.len() == 2 {
        // 將 Vec<u8> 的前兩個元素組合成一個 16 位的整數
        let result: u16 = ((o[0] as u16) << 8) | (o[1] as u16);
        return Some(result);
    }
    None
}

pub struct SocksHandler<T: AsyncRead + AsyncWrite + Unpin> {
    socket: T,
    socks_request: SocksRequest,
    server_ip_port: SocketAddr,
    client_ip_port: SocketAddr,
    // socks_reply: SocksReply,
}

impl<T: AsyncRead + AsyncWrite + Unpin> SocksHandler<T> {
    pub fn new(socket: T, data: &Vec<u8>, server_ip_port: SocketAddr, client_ip_port: SocketAddr) -> SocksHandler<T> {
        let data = data.clone();
        let socks_request = SocksRequest::deserialize_from_bytes(&data);
        SocksHandler {
            socket: socket,
            socks_request: socks_request,
            server_ip_port: server_ip_port,
            client_ip_port: client_ip_port,
        }
    }

    pub async fn execute_command(&mut self) -> Result<()> {
        debug!("execute command");
        let cmd = self.socks_request.get_command();
        if self.socks_request.get_version() != 5 {
            panic!("wrong socks version!");
        }
        match cmd {
            Socks5Command::TCPBind => {
                let resp = SocksReply::new(consts::SOCKS5_REPLY_COMMAND_NOT_SUPPORTED, self.server_ip_port).serialize_to_bytes();
                // let resp = self.generate_reply(consts::SOCKS5_REPLY_COMMAND_NOT_SUPPORTED).serialize_to_bytes();
                if let Err(e) = self.socket.write(&resp).await {
                    error!("failed to write to socket; err = {:?}", e);
                }
                Err(anyhow!("TCP Bind command not support"))
            },
            Socks5Command::TCPConnect => {
                let socket_addr = SocketAddr::new(
                    self.socks_request.get_dst_addr().await,
                    self.socks_request.get_dst_port()
                );
                let outbound_socket = tcp_connect(socket_addr).await.unwrap();
                // 判斷是否連線成功回傳正確的 res value
                // log 輸出更多的資訊，來源 IP、DST、BND 等等
                let reply_message = SocksReply::new(consts::SOCKS5_REPLY_SUCCEEDED, self.server_ip_port).serialize_to_bytes();
                // let reply_message = self.generate_reply(consts::SOCKS5_REPLY_SUCCEEDED).serialize_to_bytes();
                if let Err(e) = self.socket.write(&reply_message).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                }
                match self.socket.flush().await {
                    Ok(_) => {
                        println!("ok");
                    }
                    Err(e) => {
                        println!("{:?}", e);
                    }
                }
        
                transfer(&mut self.socket, outbound_socket).await.unwrap();
                Ok(())
            },
            Socks5Command::UDPAssociate => {
                // UDP socks request 會設定 type=domain domain=0 python client 是這樣實作的
                // 看起來這個 bound socks proxy -> target 是後面才做的
                // 感覺滿有問題好像可以不顧 TCP request 的 DST.addr 只要使用 UDP client 就可以決定送到哪裡
                let listening_client_to_socks = UdpSocket::bind(format!("{}:0", self.server_ip_port.ip())).await?;
                let listening_socks_to_target = UdpSocket::bind("0.0.0.0:0").await?;
                debug!("UDP listener bound: {:?}", listening_client_to_socks);
                debug!("UDP listener bound: {:?}", listening_socks_to_target);
                let mut b = [0; 1024];
                // debug!("{:?}", listening_client_to_socks.local_addr().unwrap());
                let resp = SocksReply::new(consts::SOCKS5_REPLY_SUCCEEDED, listening_client_to_socks.local_addr().unwrap()).serialize_to_bytes();
                if let Err(e) = self.socket.write(&resp).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                }
                let lcts = Arc::new(listening_client_to_socks);
                let lcts2 = lcts.clone();
                let lstt = Arc::new(listening_socks_to_target);

                let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(50);
                let rx_handler = tokio::spawn(async move {
                    while let Some((bytes, addr)) = rx.recv().await {
                        let udp_request = UdpMessage::deserialize_from_bytes(&bytes);
                        let send_data = udp_request.get_udp_data();
                        let send_to_addr = udp_request.get_dst_socket_addr();
                        let _len_2 = lstt.send_to(&send_data, send_to_addr).await;
                        let _len = lcts2.send_to(&bytes, &addr).await.unwrap();
                        let (len_3, _socket_addr) = lstt.recv_from(&mut b).await.unwrap();
                        let udp_response = &b[..len_3];
                        let reply_message = udp_request.reply(udp_response.to_vec());
                        lcts2.send_to(&reply_message, addr).await.unwrap();
                    }
                });
                let mut udp_buf = [0; 1024];
                let tx_handler = tokio::spawn(async move {
                    loop {
                        let (len, addr) = lcts.recv_from(&mut udp_buf).await.unwrap();
                        debug!("{:?} bytes received from {:?}", len, addr);
                        tx.send((udp_buf[..len].to_vec(), addr)).await.unwrap();
                    }
                });

                loop {
                    sleep(Duration::from_secs(1)).await;
                    match self.socket.write_all(b"ping").await {
                        Ok(_) => {
                            debug!("ping");
                        },
                        Err(_) => {
                            debug!("Connection break");
                            rx_handler.abort();
                            tx_handler.abort();
                            break;
                        },
                    }
                }
                debug!("udp finsh!");
                Ok(())
            },
        }
    }

}

async fn transfer<I, O>(mut inbound: I, mut outbound: O) -> Result<()>
where
    I: AsyncRead + AsyncWrite + Unpin,
    O: AsyncRead + AsyncWrite + Unpin,
{
    match tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await {
        Ok(res) => info!("transfer closed ({}, {})", res.0, res.1),
        Err(err) => error!("transfer error: {:?}", err),
    };

    Ok(())
}

pub async fn tcp_connect<T>(addr: T) -> Result<TcpStream>
    where T: ToSocketAddrs,
{
    match TcpStream::connect(addr).await {
        Ok(o) => {
            println!("connect successful.");
            Ok(o)
        },
        Err(e) => match e.kind() {
            _ => Err(e.into()), // #[error("General failure")] ?
        },
    }
}