pub use prost::{DecodeError, EncodeError};

use bytes::{Bytes, BytesMut};
use prost::Message;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/featureprobe.link.rs"));
}

pub fn encode(packet: proto::packet::Packet) -> Result<Bytes, EncodeError> {
    let p = proto::Packet {
        packet: Some(packet),
    };
    let mut buf = BytesMut::with_capacity(p.encoded_len());
    p.encode_length_delimited(&mut buf)?;
    Ok(buf.freeze())
}

pub fn decode(buf: &mut BytesMut) -> Result<Option<proto::packet::Packet>, DecodeError> {
    Ok(proto::Packet::decode_length_delimited(buf)?.packet)
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
            expire_at: None,
        };
        proto::packet::Packet::Message(message)
    }

    #[test]
    fn test() -> Result<(), prost::DecodeError> {
        let request = String::from("Hello, World!");
        let request = build_packet(request);
        let request_vector = encode(request).unwrap();
        let request_vector = [request_vector.clone(), request_vector].concat();
        let mut bm = BytesMut::from(request_vector.as_slice());

        let request_deserialized_result = match decode(&mut bm) {
            Ok(request_deserialized_result) => request_deserialized_result,
            Err(e) => return Err(e),
        };
        println!("1. {:#?}", request_deserialized_result);

        let request_deserialized_result = match decode(&mut bm) {
            Ok(request_deserialized_result) => request_deserialized_result,
            Err(e) => return Err(e),
        };
        println!("2. {:#?}", request_deserialized_result);
        Ok(())
    }
}
