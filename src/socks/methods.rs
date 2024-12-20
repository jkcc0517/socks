use serde::{Serialize, Deserialize};
use bincode;
use log::{debug, info};

#[derive(Debug)]
#[allow(dead_code)]
pub struct MethodRequest {
    ver: u8,
    n_methods: u8,
    methods: Vec<u8>,
}

impl MethodRequest {
    pub fn deserialize_from_bytes(bytes: &[u8]) -> MethodRequest {
        let n_methods = bytes[1];
        let end: usize = n_methods as usize + 2;
        MethodRequest {
            ver: bytes[0],
            n_methods: n_methods,
            methods: bytes[2..end].to_vec(),
        }
    }
    pub fn method_exists(&self, method: u8) -> bool {
        if self.methods.contains(&method) {
            true
        } else {
            false
        }
    }
}
/*
o  X'00' NO AUTHENTICATION REQUIRED
o  X'01' GSSAPI
o  X'02' USERNAME/PASSWORD
o  X'03' to X'7F' IANA ASSIGNED
o  X'80' to X'FE' RESERVED FOR PRIVATE METHODS
o  X'FF' NO ACCEPTABLE METHODS
 */
#[derive(Serialize, Debug)]
pub struct MethodReply {
    ver: u8,
    method: u8,
}

impl MethodReply {
    pub fn new(method: u8) -> MethodReply {
        MethodReply {
            ver: 5,
            method,
        }
    }
    pub fn serialize_to_bytes(&self) -> Vec<u8> {
        debug!("{:?}", self);
        bincode::serialize(self).expect("Serialization failed")
    }
}