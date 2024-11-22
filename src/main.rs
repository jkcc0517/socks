use std::net::SocketAddr;
use log::{info, error};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
pub type Result<T, E = std::io::Error> = core::result::Result<T, E>;
mod consts;
mod socks;
use crate::socks::{SocksRequest, Socks5Command};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:1080").await?;
    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            socks_connection_handle(socket).await.unwrap();
        });
    }
}

// Now noly support no-auth
// TODO: other auths
async fn auth_method_handle(socket: &mut TcpStream) -> Result<()> {
    match socket.write(&[5, 0]).await {
        Ok(_) => {
            Ok(())
        }
        Err(e) => {
            eprintln!("failed to write to socket; err = {:?}", e);
            Err(e)
        }
    }
}

async fn socks_connection_handle(mut socket: TcpStream) -> Result<()> {
    let mut buf = [0; 1024];
    let mut first: bool = true;
    // In a loop, read data from the socket and write the data back.
    loop {
        let _n = match &socket.read(&mut buf).await {
            // socket closed
            Ok(n) if *n == 0 => {
                println!("null");
                return Ok(());
            },
            Ok(n) => {
                n
            },
            Err(e) => {
                eprintln!("failed to read from socket; err = {:?}", e);
                return Ok(());
            }
        };
        if first == true {
            first = false;
            auth_method_handle(&mut socket).await.unwrap();
        } else {
            let req = SocksRequest::new(&buf.to_vec());
            let cmd = req.get_command();
            if req.get_version() != 5 {
                panic!("wrong socks version!");
            }
            match cmd {
                Socks5Command::TCPBind => {
                    let resp = socks_v5_response_data(consts::BIND_IP_PORT, 0).await;
                    if let Err(e) = socket.write(&resp).await {
                        eprintln!("failed to write to socket; err = {:?}", e);
                        return Ok(());
                    }
                },
                Socks5Command::TCPConnect => {
                    let ip = req.get_dst_addr().await;
                    let socket_addr = SocketAddr::new(ip, req.get_dst_port());
                    let outbound_socket = tcp_connect(socket_addr).await.unwrap();
                    let resp = socks_v5_response_data(consts::BIND_IP_PORT, 0).await;
                    if let Err(e) = socket.write(&resp).await {
                        eprintln!("failed to write to socket; err = {:?}", e);
                        return Ok(());
                    }
                    match socket.flush().await {
                        Ok(_) => {
                            println!("ok");
                        }
                        Err(e) => {
                            println!("{:?}", e);
                        }
                    }
            
                    transfer(&mut socket, outbound_socket).await.unwrap();
                },
                Socks5Command::UDPAssociate => {
                    // TODO: this
                },
            }
        }
    }
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

async fn socks_v5_response_data(sock_addr: SocketAddr, status: u8) -> Vec<u8> {
    let (addr_type, mut ip_oct, mut port) = match sock_addr {
        SocketAddr::V4(sock) => (
            consts::SOCKS5_ADDR_TYPE_IPV4,
            sock.ip().octets().to_vec(),
            sock.port().to_be_bytes().to_vec(),
        ),
        SocketAddr::V6(sock) => (
            consts::SOCKS5_ADDR_TYPE_IPV6,
            sock.ip().octets().to_vec(),
            sock.port().to_be_bytes().to_vec(),
        ),
    };

    let mut reply = vec![0x05, status, 0x00, addr_type];
    reply.append(&mut ip_oct);
    reply.append(&mut port);
    reply
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