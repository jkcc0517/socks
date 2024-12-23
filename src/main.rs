use log::{debug, info, error};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
mod consts;
mod socks;

use socks::handlers::{SocksHandler, MethodHandler};
use anyhow::Result;

#[tokio::main(flavor = "multi_thread", worker_threads = 100)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let listener = TcpListener::bind("0.0.0.0:1080").await?;
    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            process_socks_connection(socket).await.unwrap();
        });
    }
}

async fn process_socks_connection(mut socket: TcpStream) -> Result<()> {
    let mut buf = [0; 1024];
    let mut is_authenticated: bool = false;
    // In a loop, read data from the socket and write the data back.

    loop {
        let n = match socket.read(&mut buf).await {
            Ok(n) => {
                if n == 0 {
                    info!("end the connection.");
                    return Ok(())
                }
                n
            },
            Err(e) => {
                debug!("socket connection disconnect. Reason: {}", e);
                return Ok(())
            }
        };

        let client_ip_info = socket.peer_addr();
        info!("{:?}", client_ip_info);
        let buf = &buf[..n];
        if is_authenticated == false {
            let mut method_handler = MethodHandler::new(&mut socket, &buf);
            method_handler.reply().await?;
            is_authenticated = true;
        } else {
            let server_ip_port: SocketAddr = socket.local_addr().unwrap().clone();
            let client_ip_port: SocketAddr = socket.peer_addr().unwrap().clone();
            let mut socks_handler = SocksHandler::new(
                &mut socket,
                &buf.to_vec(),
                server_ip_port,
                client_ip_port,
            );
            if let Err(e) = socks_handler.execute_command().await {
                error!("Socks error: {}", e);
                return Ok(());
            }
        }
    }
}