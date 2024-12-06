use log::{info, error, LevelFilter};
use std::thread;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

mod consts;
mod socks;
use crate::socks::{SocksHandler, SocksRequest, Socks5Command};
use socks::methods::MethodHandler;
use anyhow::{anyhow, Result};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let listener = TcpListener::bind("0.0.0.0:1080").await?;
    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            socks_connection_handle(socket).await.unwrap();
        });
    }
}

async fn socks_connection_handle(mut socket: TcpStream) -> Result<()> {
    let mut buf = [0; 1024];
    let mut first: bool = true;
    // In a loop, read data from the socket and write the data back.
    loop {
        let n = socket.read(&mut buf).await.expect("Socket read error.") as u32;
        if n == 0 {
            info!("end the connection");
            return Ok(())
        }
        let client_ip_info = socket.peer_addr();
        info!("{:?}", client_ip_info);
        if first == true {
            first = false;
            let reply_message = MethodHandler::get_reply_message(&buf);
            socket.write(&reply_message).await.expect("reply for auth method request");
        } else {
            let mut socks = SocksHandler::new(&mut socket, &buf.to_vec());
            if let Err(e) = socks.execute_command().await {
                error!("Socks error: {}", e);
                return Ok(());
            }
        }
    }
}