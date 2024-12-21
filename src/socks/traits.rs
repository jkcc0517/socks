pub trait SocksDeserializeable {
    fn deserialize_from_bytes(bytes: &[u8]) -> Self;
}

pub trait SocksSerializable {
    fn serialize_to_bytes(&self) -> Vec<u8>;
}