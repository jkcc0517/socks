pub trait SocksSerializable {
    fn deserialize_from_bytes(bytes: &[u8]) -> Self {
        panic!("deserialize_from_bytes not implemented for this type")
    }
    fn serialize_to_bytes(&self) -> Vec<u8>;
}