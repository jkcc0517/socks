use super::methods::{MethodRequest, MethodReply};
use log::debug;
pub struct MethodHandler {}

impl MethodHandler {
    pub fn get_reply_message(request: &[u8]) -> Vec<u8> {
        let m_request = MethodRequest::deserialize_from_bytes(request);
        debug!("{:?}", m_request);
        let allow_method = match m_request.method_exists(0) {
            true => crate::consts::SOCKS5_AUTH_METHOD_NONE,
            false => crate::consts::SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE,
        };
        let m_reply = MethodReply::new(allow_method);
        m_reply.serialize_to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts;

    #[test]
    fn test_method_handler_valid_method() {
        let request = vec![5, 1, consts::SOCKS5_AUTH_METHOD_NONE]; // Version 5, 1 method, method=NO_AUTH
        let reply = MethodHandler::get_reply_message(&request);
        assert_eq!(reply, vec![5, consts::SOCKS5_AUTH_METHOD_NONE]);
    }

    #[test]
    fn test_method_handler_invalid_method() {
        let request = vec![5, 1, 0xff]; // Version 5, 1 method, unsupported method
        let reply = MethodHandler::get_reply_message(&request);
        assert_eq!(reply, vec![5, consts::SOCKS5_AUTH_METHOD_NOT_ACCEPTABLE]);
    }
}