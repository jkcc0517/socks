pub trait SocksPacket {
    fn deserialize_from_bytes(bytes: &[u8]) -> Self;
    fn serialize_to_bytes(&self) -> Vec<u8>;
}