use super::{SocksAddress, SocksPort};
use super::calculate_port_number;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use log::{debug, info};

// +----+------+------+----------+----------+----------+
// |RSV | FRAG | ATYP | DST.ADDR | DST.PORT |   DATA   |
// +----+------+------+----------+----------+----------+
// | 2  |  1   |  1   | Variable |    2     | Variable |
// +----+------+------+----------+----------+----------+
// The fields in the UDP request header are:
//     o  RSV  Reserved X'0000'
//     o  FRAG    Current fragment number
//     o  ATYP    address type of following addresses:
//        o  IP V4 address: X'01'
//        o  DOMAINNAME: X'03'
//        o  IP V6 address: X'04'
//     o  DST.ADDR       desired destination address
//     o  DST.PORT       desired destination port
//     o  DATA     user data

#[derive(Debug, Clone)]
pub struct SocksUdpData {
    data: Vec<u8>,
}
impl SocksUdpData {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: data
        }
    }
}

#[derive(Debug)]
pub struct UdpMessage {
    rsv: u16,
    frag: u8,
    atyp: u8,
    dst_addr: SocksAddress,
    dst_port: SocksPort,
    data: SocksUdpData,
}

impl UdpMessage {
    pub fn deserialize_from_bytes(bytes: &[u8]) -> Self {
        let mut data = bytes.to_vec();
        let _rsv: Vec<u8> = data.drain(0..2).collect(); // 保留位元組，不處理
        let frag: u8 = data.remove(0);
        let atyp: u8 = data.remove(0);
        let dst_address = SocksAddress::parse_destination_address(atyp, &mut data);
        let port = calculate_port_number(data.remove(0), data.remove(0)).unwrap();
        let udp_data: &[u8] = &data[0..];
        Self {
            rsv: 0,
            frag: frag,
            atyp: atyp,
            dst_addr: dst_address,
            dst_port: SocksPort::new(port),
            data: SocksUdpData::new(udp_data.to_vec()),
        }
    }
    pub fn get_udp_data(&self) -> Vec<u8> {
        self.data.data.clone()
    }

    pub fn _get_dst_port(&self) -> u16 {
        self.dst_port.into()
    }

    pub fn get_dst_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53)
    }

    pub fn reply(&self, data: Vec<u8>) -> Vec<u8> {
        let mut message: Vec<u8> = vec![
            0,
            0,
            self.frag,
            self.atyp,
        ];
        message.extend(&self.dst_addr.serialize_to_bytes());
        message.extend(&self.dst_port.serialize_to_bytes());
        message.extend(&data);
        message
    }
}