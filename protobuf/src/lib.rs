use bytes::{Bytes, BytesMut};
pub use tonic;

use prost::{EncodeError, Message};
use std::io::Cursor;

pub mod proto;
// pub mod proto {
// include!(concat!(env!("OUT_DIR"), "/featureprobe.link.service.rs"));
// }

pub fn encode(packet: proto::packet::Packet) -> Result<Bytes, EncodeError> {
    let p = proto::Packet {
        packet: Some(packet),
    };
    let mut buf = BytesMut::with_capacity(p.encoded_len());
    p.encode(&mut buf)?;
    Ok(buf.freeze())
}

pub fn decode(buf: &[u8]) -> Result<Option<proto::packet::Packet>, prost::DecodeError> {
    Ok(proto::Packet::decode(&mut Cursor::new(buf))?.packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    pub fn build_packet(namespace: String) -> proto::packet::Packet {
        let message: proto::Message = proto::Message {
            namespace,
            path: "path".to_owned(),
            metadata: Default::default(),
            body: vec![1, 2, 3, 4],
        };
        proto::packet::Packet::Message(message)
    }

    #[test]
    fn test() -> Result<(), prost::DecodeError> {
        let request = String::from("Hello, World!");

        let request = build_packet(request);
        let request_vector = encode(request).unwrap();

        let request_deserialized_result = match decode(&request_vector) {
            Ok(request_deserialized_result) => request_deserialized_result,
            Err(e) => return Err(e),
        };
        println!("{:#?}", request_deserialized_result);
        Ok(())
    }
}
