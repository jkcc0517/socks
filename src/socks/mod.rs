pub mod replies;
pub mod requests;
pub mod methods;
pub mod client;
pub mod udp;
pub mod traits;
pub mod handlers;

// use serde::Serialize;
use log::debug;
use tokio::net::lookup_host;
use traits::*;
use super::consts;
use std::array::TryFromSliceError;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use requests::SocksRequest;
use anyhow::Result;

#[derive(Debug, Clone)]
pub enum SocksCommand {
    TCPConnect,
    TCPBind,
    UDPAssociate,
}

#[allow(dead_code)]
impl SocksCommand {
    #[inline]
    #[rustfmt::skip]
    fn as_u8(&self) -> u8 {
        match self {
            SocksCommand::TCPConnect   => consts::SOCKS5_CMD_TCP_CONNECT,
            SocksCommand::TCPBind      => consts::SOCKS5_CMD_TCP_BIND,
            SocksCommand::UDPAssociate => consts::SOCKS5_CMD_UDP_ASSOCIATE,
        }
    }
}

impl From<u8> for SocksCommand {
    fn from(number: u8) -> SocksCommand {
        match number {
            consts::SOCKS5_CMD_TCP_CONNECT      => SocksCommand::TCPConnect,
            consts::SOCKS5_CMD_TCP_BIND         => SocksCommand::TCPBind,
            consts::SOCKS5_CMD_UDP_ASSOCIATE    => SocksCommand::UDPAssociate,
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

}
impl SocksPacket for SocksPort {
    fn serialize_to_bytes(&self) -> Vec<u8> {
        self.0.to_be_bytes().to_vec()
    }
    
    fn deserialize_from_bytes(bytes: &[u8]) -> Self {
        todo!()
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
    pub fn parse_dst_address(atyp: u8, data: &mut Vec<u8>) -> SocksAddress {
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

fn calculate_port_number(first: u8, second: u8) -> Option<u16> {
    let o: Vec<u8> = vec![first, second];
    if o.len() == 2 {
        // 將 Vec<u8> 的前兩個元素組合成一個 16 位的整數
        let result: u16 = ((o[0] as u16) << 8) | (o[1] as u16);
        return Some(result);
    }
    None
}