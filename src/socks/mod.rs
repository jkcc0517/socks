pub mod server;
pub mod client;

use crate::consts;
use std::array::TryFromSliceError;
// use log::{info, error};
use tokio::net::lookup_host;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
pub type Result<T, E = std::io::Error> = core::result::Result<T, E>;

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

#[derive(Debug)]
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

pub struct SocksRequest {
    version: u8,
    command: Socks5Command,
    atyp: u8,
    destination_address: DestinationAddress,
    destination_port: u16,
}

impl SocksRequest {
    pub fn new(data: &Vec<u8>) -> SocksRequest {
        let mut data = data.clone();
        println!("data: {:?}", data);
        let version = data.remove(0);
        println!("version: {:?}", version);
        let command = Socks5Command::from(data.remove(0));
        println!("command: {:?}", command);
        let _rsv: u8 = data.remove(0);
        let atyp: u8 = data.remove(0);
        //let destination_address: DestinationAddress = match atyp {
        let dest_address: DestinationAddress = match atyp {
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
                panic!("parse address error!!d");
            },
        };
        let port = calculate_port_number(data.remove(0), data.remove(0)).unwrap();
        SocksRequest {
            version: version,
            command: command,
            atyp: atyp,
            destination_address: dest_address,
            destination_port: port,
        }
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