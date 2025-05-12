use log::{debug, info, error};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
use clap::Parser;
mod consts;
mod socks;

use socks::handlers::{SocksHandler, MethodHandler};
use anyhow::Result;

/// A SOCKS5 proxy server
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Host address to bind
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port number to listen on
    #[arg(long, default_value_t = 1080)]
    port: u16,

    /// Enable verbose mode
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 100)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // 設置日誌級別
    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", if args.verbose { "debug" } else { "info" });
    env_logger::Builder::from_env(env).init();

    let addr = format!("{}:{}", args.host, args.port);
    info!("Starting SOCKS5 server on {}", addr);

    let listener = TcpListener::bind(&addr).await?;
    info!("SOCKS5 server listening on {}", addr);

    loop {
        let (socket, addr) = listener.accept().await?;
        info!("New connection from {}", addr);
        tokio::spawn(async move {
            if let Err(e) = process_socks_connection(socket).await {
                error!("Connection error: {}", e);
            }
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