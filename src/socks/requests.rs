use crate::socks::{Socks5Command, DestinationAddress, calculate_port_number};
use log::{debug, info};
use std::net::IpAddr;

#[derive(Debug)]
pub struct SocksRequest {
    version: u8,
    command: Socks5Command,
    atyp: u8,
    destination_address: DestinationAddress,
    destination_port: u16,
}

impl SocksRequest {
    pub fn deserialize_from_bytes(bytes: &[u8]) -> SocksRequest {
        debug!("socks request content: {:?}", bytes);
        let mut data = bytes.to_vec();
        let version = data.remove(0);
        let command = Socks5Command::from(data.remove(0));
        let _rsv: u8 = data.remove(0);
        let atyp: u8 = data.remove(0);
        //let destination_address: DestinationAddress = match atyp {
        let dest_address = DestinationAddress::parse_destination_address(atyp, &mut data);
        let port = calculate_port_number(data.remove(0), data.remove(0)).unwrap();
        let socks_request = SocksRequest {
            version: version,
            command: command,
            atyp: atyp,
            destination_address: dest_address,
            destination_port: port,
        };
        debug!("{:?}", socks_request);
        socks_request
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
