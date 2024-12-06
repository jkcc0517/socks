use std::net::{IpAddr, SocketAddr};
use crate::consts;
use log::debug;
use serde::{Serialize, Deserialize};
use std::net::{Ipv4Addr, Ipv6Addr};
use crate::socks::DestinationAddress;
use bincode::{options, serialize};
struct AuthReply {}

/*
o  REP    Reply field:
    o  X'00' succeeded
    o  X'01' general SOCKS server failure
    o  X'02' connection not allowed by ruleset
    o  X'03' Network unreachable
    o  X'04' Host unreachable
    o  X'05' Connection refused
    o  X'06' TTL expired
    o  X'07' Command not supported
    o  X'08' Address type not supported
    o  X'09' to X'FF' unassigned
*/

#[derive(Serialize, Debug)]
pub struct SocksReply {
    ver: u8,
    rep: u8, // Reply field
    rsv: u8,
    atyp: u8,
    bnd_addr: DestinationAddress,
    bnd_port: u16,
}

impl SocksReply {
    pub fn new(rep: u8, ip_addr: SocketAddr) -> SocksReply {
        let reply_message = match ip_addr.ip() {
            IpAddr::V4(ipv4) => {
                SocksReply {
                    ver: 5,
                    rep: rep,
                    rsv: 0,
                    atyp: 1,
                    bnd_addr: DestinationAddress::IP(IpAddr::V4(Ipv4Addr::from(ipv4))),
                    bnd_port: ip_addr.port(),
                }
            },
            IpAddr::V6(ipv6) => {
                SocksReply {
                    ver: 5,
                    rep: rep,
                    rsv: 0,
                    atyp: 4,
                    bnd_addr: DestinationAddress::IP(IpAddr::V6(Ipv6Addr::from(ipv6))),
                    bnd_port: ip_addr.port(),
                }
            },
        };
        debug!("{:?}", reply_message);
        reply_message
    }
    pub fn serialize_to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Serialization failed")
    }
}

// pub async fn socks_reply_data(sock_addr: SocketAddr, status: u8) -> Vec<u8> {
//     let (addr_type, mut ip_oct, mut port) = match sock_addr {
//         SocketAddr::V4(sock) => (
//             consts::SOCKS5_ADDR_TYPE_IPV4,
//             sock.ip().octets().to_vec(),
//             sock.port().to_be_bytes().to_vec(),
//         ),
//         SocketAddr::V6(sock) => (
//             consts::SOCKS5_ADDR_TYPE_IPV6,
//             sock.ip().octets().to_vec(),
//             sock.port().to_be_bytes().to_vec(),
//         ),
//     };

//     let mut reply = vec![0x05, status, 0x00, addr_type];
//     reply.append(&mut ip_oct);
//     reply.append(&mut port);
//     debug!("{:?}", reply);
//     reply
// }