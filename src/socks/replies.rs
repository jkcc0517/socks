use std::net::{IpAddr, SocketAddr};
use crate::consts;
use log::debug;
use std::net::{Ipv4Addr, Ipv6Addr};
use crate::socks::{SocksAddress, SocksPort};
use super::traits::*;
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

#[derive(Debug)]
pub struct SocksReply {
    ver: u8,
    rep: u8, // Reply field
    rsv: u8,
    atyp: u8,
    bnd_addr: SocksAddress,
    bnd_port: SocksPort,
}

impl SocksDeserializeable for SocksReply {
    fn deserialize_from_bytes(bytes: &[u8]) -> Self {
        todo!()
    }
}

impl SocksSerializable for SocksReply {
    fn serialize_to_bytes(&self) -> Vec<u8> {
        let mut s = vec![self.ver, self.rep, self.rsv, self.atyp];
        s.extend(self.bnd_addr.serialize_to_bytes());
        s.extend(self.bnd_port.serialize_to_bytes());
        s
    }
}

impl SocksReply {
    pub fn new(rep: u8, socks_addr: SocketAddr) -> SocksReply {
        let reply_message = match socks_addr.ip() {
            IpAddr::V4(ipv4) => {
                SocksReply {
                    ver: consts::SOCKS5_VERSION,
                    rep: rep,
                    rsv: 0,
                    atyp: consts::SOCKS5_ADDR_TYPE_IPV4,
                    bnd_addr: SocksAddress::IP(IpAddr::V4(Ipv4Addr::from(ipv4))),
                    bnd_port: SocksPort::new(socks_addr.port()),
                }
            },
            IpAddr::V6(ipv6) => {
                SocksReply {
                    ver: consts::SOCKS5_VERSION,
                    rep: rep,
                    rsv: 0,
                    atyp: consts::SOCKS5_ADDR_TYPE_IPV6,
                    bnd_addr: SocksAddress::IP(IpAddr::V6(Ipv6Addr::from(ipv6))),
                    bnd_port: SocksPort::new(socks_addr.port()),
                }
            },
        };
        debug!("{:?}", reply_message);
        reply_message
    }
}