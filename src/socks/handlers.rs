use super::methods::{MethodRequest, MethodReply};
use super::{SocksCommand, SocksRequest};
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use std::sync::Arc;
use super::udp::UdpMessage;
use log::{debug, error, info};
use tokio::time::{sleep, Duration};
use tokio::net::{TcpStream, ToSocketAddrs, UdpSocket};
use super::replies::SocksReply;
use anyhow::{Result, anyhow};
use tokio::sync::mpsc;
use super::consts;
use super::traits::*;
pub struct MethodHandler {}

impl MethodHandler {
    pub fn get_reply_message(request: &[u8]) -> Vec<u8> {
        let m_request = MethodRequest::deserialize_from_bytes(request);
        debug!("{:?}", m_request);
        let allow_method = match m_request.method_exists(0) {
            true => crate::consts::SOCKS5_AUTH_METHOD_NONE,
            false => crate::consts::SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE,
        };
        let m_reply = MethodReply::new(allow_method);
        m_reply.serialize_to_bytes()
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
            SocksCommand::TCPBind => {
                let resp = SocksReply::new(consts::SOCKS5_REPLY_COMMAND_NOT_SUPPORTED, self.server_ip_port).serialize_to_bytes();
                // let resp = self.generate_reply(consts::SOCKS5_REPLY_COMMAND_NOT_SUPPORTED).serialize_to_bytes();
                if let Err(e) = self.socket.write(&resp).await {
                    error!("failed to write to socket; err = {:?}", e);
                }
                Err(anyhow!("TCP Bind command not support"))
            },
            SocksCommand::TCPConnect => {
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
            SocksCommand::UDPAssociate => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts;

    #[test]
    fn test_method_handler_valid_method() {
        let request = vec![5, 1, consts::SOCKS5_AUTH_METHOD_NONE]; // Version 5, 1 method, method=NO_AUTH
        let reply = MethodHandler::get_reply_message(&request);
        assert_eq!(reply, vec![5, consts::SOCKS5_AUTH_METHOD_NONE]);
    }

    #[test]
    fn test_method_handler_invalid_method() {
        let request = vec![5, 1, 0xff]; // Version 5, 1 method, unsupported method
        let reply = MethodHandler::get_reply_message(&request);
        assert_eq!(reply, vec![5, consts::SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE]);
    }
}