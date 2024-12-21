use super::{SocksCommand, SocksAddress, SocksPort, calculate_port_number};
use log::{debug, info};
use std::net::IpAddr;
use super::traits::*;

#[derive(Debug)]
#[allow(dead_code)]
pub struct SocksRequest {
    ver: u8,
    cmd: SocksCommand,
    atyp: u8,
    dst_address: SocksAddress,
    dst_port: SocksPort,
}

impl SocksDeserializeable for SocksRequest {
    fn deserialize_from_bytes(bytes: &[u8]) -> SocksRequest {
        debug!("socks request content: {:?}", bytes);
        let mut data = bytes.to_vec();
        let ver = data.remove(0);
        let command = SocksCommand::from(data.remove(0));
        let _rsv: u8 = data.remove(0);
        let atyp: u8 = data.remove(0);
        //let dst_address: SocksAddress = match atyp {
        let dst_address = SocksAddress::parse_dst_address(atyp, &mut data);
        let port = calculate_port_number(data.remove(0), data.remove(0)).unwrap();
        let socks_request = SocksRequest {
            ver: ver,
            cmd: command,
            atyp: atyp,
            dst_address: dst_address,
            dst_port: SocksPort::new(port),
        };
        debug!("{:?}", socks_request);
        socks_request
    }
}

impl SocksSerializable for SocksRequest {
    fn serialize_to_bytes(&self) -> Vec<u8> {
        todo!()
    }
}

impl SocksRequest {
    pub async fn get_dst_addr(&self) -> IpAddr {
        self.dst_address.get_ip_addr().await
    }
    pub fn get_dst_port(&self) -> u16 {
        self.dst_port.into()
    }
    pub fn get_cmd(&self) -> SocksCommand {
        self.cmd.clone()
    }
    pub fn get_ver(&self) -> u8 {
        self.ver
    }
}