use super::methods::{MethodRequest, MethodReply};
use super::{SocksCommand, SocksRequest};
use super::udp::UdpMessage;
use super::replies::SocksReply;
use super::consts;
use super::traits::*;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::time::{sleep, Duration};
use tokio::net::{TcpStream, ToSocketAddrs, UdpSocket};
use tokio::sync::mpsc;
use log::{debug, error, info};
use anyhow::{Result, anyhow};

pub struct MethodHandler<T: AsyncRead + AsyncWrite + Unpin> {
    socket: T,
    method_request: MethodRequest,
}

impl<T: AsyncRead + AsyncWrite + Unpin> MethodHandler<T> {
    pub fn new(socket: T, request: &[u8]) -> Self {
        let r: MethodRequest = MethodRequest::deserialize_from_bytes(request);
        debug!("{:?}", r);
        Self {
            socket: socket,
            method_request: r,
        }
    }
    pub async fn reply(&mut self) -> Result<()> {
        let allow_method = match self.method_request.method_exists(0) {
            true => crate::consts::SOCKS5_AUTH_METHOD_NONE,
            false => crate::consts::SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE,
        };
        let method_reply = MethodReply::new(allow_method);
        self.socket.write(&method_reply.serialize_to_bytes()).await?;
        Ok(())
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

    async fn tcp_bind(&mut self) -> Result<()> {
        let resp = SocksReply::new(consts::SOCKS5_REPLY_COMMAND_NOT_SUPPORTED, self.server_ip_port).serialize_to_bytes();
        // let resp = self.generate_reply(consts::SOCKS5_REPLY_COMMAND_NOT_SUPPORTED).serialize_to_bytes();
        if let Err(e) = self.socket.write(&resp).await {
            error!("failed to write to socket; err = {:?}", e);
        }
        Err(anyhow!("TCP Bind command not support"))
    }

    async fn tcp_connect(&mut self) -> Result<()> {
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
            error!("failed to write to socket; err = {:?}", e);
            return Err(anyhow!("{}", e));
        }

        if let Err(e) = self.socket.flush().await {
            return Err(anyhow!("{}", e));
        }

        transfer(&mut self.socket, outbound_socket).await.unwrap();
        Ok(())
    }
    
    async fn udp_associate(&mut self) -> Result<()> {
        // UDP socks request 會設定 type=domain domain=0 python client 是這樣實作的
        // 看起來這個 bound socks proxy -> target 是後面才做的
        // 感覺滿有問題好像可以不顧 TCP request 的 DST.addr 只要使用 UDP client 就可以決定送到哪裡
        let udp_for_client = UdpSocket::bind(format!("{}:0", self.server_ip_port.ip())).await?;
        let udp_for_target = UdpSocket::bind("0.0.0.0:0").await?;
        debug!("UDP listener bound: {:?}", udp_for_client);
        debug!("UDP listener bound: {:?}", udp_for_target);
        let mut b = [0; 1024];
        // debug!("{:?}", udp_for_client.local_addr().unwrap());
        let resp = SocksReply::new(
            consts::SOCKS5_REPLY_SUCCEEDED,
            udp_for_client.local_addr().unwrap()
        ).serialize_to_bytes();
        if let Err(e) = self.socket.write(&resp).await {
            error!("failed to write to socket; err = {:?}", e);
            return Err(anyhow!("{}", e));
        }
        let aufc = Arc::new(udp_for_client);
        let aufc2 = aufc.clone();
        let auft = Arc::new(udp_for_target);

        let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(50);
        let rx_handler = tokio::spawn(async move {
            while let Some((bytes, addr)) = rx.recv().await {
                let udp_request = UdpMessage::deserialize_from_bytes(&bytes);
                let send_data = udp_request.get_udp_data();
                let send_to_addr = udp_request.get_dst_socket_addr();
                let _ = auft.send_to(&send_data, send_to_addr).await.unwrap();
                let (resp_len, _socket_addr) = auft.recv_from(&mut b).await.unwrap();
                let udp_response = &b[..resp_len];
                let reply_message = udp_request.generate_reply_message(udp_response.to_vec());
                aufc2.send_to(&reply_message.serialize_to_bytes(), addr).await.unwrap();
            }
        });
        let mut udp_buf = [0; 1024];
        let tx_handler = tokio::spawn(async move {
            loop {
                let (len, addr) = aufc.recv_from(&mut udp_buf).await.unwrap();
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
    }

    pub async fn execute_command(&mut self) -> Result<()> {
        let cmd = self.socks_request.get_cmd();
        if self.socks_request.get_ver() != 5 {
            panic!("wrong socks version!");
        }
        match cmd {
            SocksCommand::TCPBind => {
                debug!("execute TCP bind command");
                self.tcp_bind().await?
            },
            SocksCommand::TCPConnect => {
                debug!("execute TCP connect command");
                self.tcp_connect().await?
            },
            SocksCommand::UDPAssociate => {
                debug!("execute UDP associate command");
                self.udp_associate().await?
            },
        };
        Ok(())
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
            info!("connect successful.");
            Ok(o)
        },
        Err(e) => match e.kind() {
            _ => Err(e.into()), // #[error("General failure")] ?
        },
    }
}