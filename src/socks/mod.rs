pub mod replies;
pub mod methods;
pub mod client;
pub mod utils;
pub mod udp;

use serde::ser::{Serialize, SerializeStruct, Serializer};
// use serde::Serialize;
use log::{debug, error, info};
use std::net::{SocketAddr};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket, lookup_host};
use tokio::sync::mpsc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::sync::Arc;
use udp::requests::UDPRequest;
use crate::consts;
use std::array::TryFromSliceError;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use replies::SocksReply;
use anyhow::{Result, anyhow};

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

#[derive(Debug, Clone)]
// #[serde(untagged)]
pub enum DestinationAddress {
    IP(IpAddr),
    Domain(String),
}

impl DestinationAddress {
    async fn get_ip_addr(&self) -> IpAddr {
        match self {
            DestinationAddress::IP(ip_addr) => {
                *ip_addr
            },
            DestinationAddress::Domain(domain) => {
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
    pub fn parse_destination_address(atyp: u8, data: &mut Vec<u8>) -> DestinationAddress {
        match atyp {
            consts::SOCKS5_ADDR_TYPE_IPV4 => {
                let address: Vec<u8> = (1..=4).map(|_|
                    data.remove(0)
                ).collect();
                let address: Result<[u8; 4], TryFromSliceError> = address.as_slice().try_into() as Result<[u8; 4], TryFromSliceError>;
                DestinationAddress::IP(IpAddr::V4(Ipv4Addr::from(address.unwrap())))
            },
            consts::SOCKS5_ADDR_TYPE_DOMAIN_NAME => {
                let number_of_name = data.remove(0);
                let domain: String = (1..=number_of_name).map(|_|
                    data.remove(0) as char
                ).collect();
                DestinationAddress::Domain(domain)
            },
            consts::SOCKS5_ADDR_TYPE_IPV6 => {
                let address: Vec<u8> = (1..=16).map(|_|
                    data.remove(0)
                ).collect();
                let address: Result<[u8; 16], TryFromSliceError> = address.as_slice().try_into() as Result<[u8; 16], TryFromSliceError>;
                DestinationAddress::IP(IpAddr::V6(Ipv6Addr::from(address.unwrap())))
            },
            _ => {
                debug!("{:?}", data);
                panic!("atyp {:?} parsed error!!", atyp);
            },
        }
    }
}

impl Serialize for DestinationAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            DestinationAddress::IP(ip) => {
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
            DestinationAddress::Domain(domain) => {
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
        println!("計算結果: {}", result);  // 輸出 258
        return Some(result);
    } else {
        println!("Vec 長度不正確");
    }
    None
}

#[derive(Debug)]
pub struct SocksRequest {
    version: u8,
    command: Socks5Command,
    atyp: u8,
    destination_address: DestinationAddress,
    destination_port: u16,
}

impl SocksRequest {
    pub fn deserialize_from_bytes(bytes: &[u8]) -> SocksRequest {
        debug!("socks request content: {:?}", bytes);
        let mut data = bytes.to_vec();
        let version = data.remove(0);
        let command = Socks5Command::from(data.remove(0));
        let _rsv: u8 = data.remove(0);
        let atyp: u8 = data.remove(0);
        //let destination_address: DestinationAddress = match atyp {
        let dest_address = DestinationAddress::parse_destination_address(atyp, &mut data);
        let port = calculate_port_number(data.remove(0), data.remove(0)).unwrap();
        let socks_request = SocksRequest {
            version: version,
            command: command,
            atyp: atyp,
            destination_address: dest_address,
            destination_port: port,
        };
        debug!("{:?}", socks_request);
        socks_request
    }
    pub async fn get_dst_addr(&self) -> IpAddr {
        self.destination_address.get_ip_addr().await
    }
    pub fn get_dst_port(&self) -> u16 {
        self.destination_port
    }
    pub fn get_command(&self) -> Socks5Command {
        self.command.clone()
    }
    pub fn get_version(&self) -> u8 {
        self.version
    }
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
                    eprintln!("failed to write to socket; err = {:?}", e);
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
                // let lcts = Arc::new(listening_client_to_socks);
                // let lcts2 = lcts.clone();
                // let lstt = Arc::new(listening_socks_to_target);
                // if let Err(e) = self.socket.write(&resp).await {
                //     eprintln!("failed to write to socket; err = {:?}", e);
                // }

                // let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(50);
                // tokio::spawn(async move {
                //     while let Some((bytes, addr)) = rx.recv().await {
                //         let udp_request = UDPRequest::deserialize_from_bytes(&bytes);
                //         let send_data = udp_request.get_udp_data();
                //         let send_to_addr = udp_request.get_dst_socket_addr();
                //         let _len_2 = lstt.send_to(&send_data, send_to_addr).await;
                //         let len = lcts2.send_to(&bytes, &addr).await.unwrap();
                //         let (len_3, _socket_addr) = lstt.recv_from(&mut b).await.unwrap();
                //         let udp_response = &b[..len_3];
                //         let reply_message = udp_request.reply(udp_response.to_vec());
                //         lcts2.send_to(&reply_message, addr).await.unwrap();
                //         println!("{:?} bytes sent", len);
                //     }
                // });
                // let mut udp_buf = [0; 1024];
                // loop {
                //     let (len, addr) = lcts.recv_from(&mut udp_buf).await?;
                //     println!("{:?} bytes received from {:?}", len, addr);
                //     tx.send((udp_buf[..len].to_vec(), addr)).await.unwrap();
                // }
                println!("try read");

                loop {
                    info!("start udp listen {:?}", listening_client_to_socks);
                    let (len, socks_listening_socks_to_target) = listening_client_to_socks.recv_from(&mut b).await?;
                    debug!("length: {:?}", len);
                    let udp_request = UDPRequest::deserialize_from_bytes(&b[..len]);
                    debug!("{:?}", udp_request);
                    let send_data = udp_request.get_udp_data();
                    let send_to_addr = udp_request.get_dst_socket_addr();
                    let _len_2 = listening_socks_to_target.send_to(&send_data, send_to_addr).await;
                    let (len_3, _socket_addr) = listening_socks_to_target.recv_from(&mut b).await?;
                    let udp_response = &b[..len_3];
                    let reply_message = udp_request.reply(udp_response.to_vec());
                    listening_client_to_socks.send_to(&reply_message, socks_listening_socks_to_target).await?;
                }
                println!("udp finsh!");
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