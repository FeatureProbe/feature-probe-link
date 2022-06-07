use bytes::{Buf, Bytes, BytesMut};
use prost::Message;
pub use prost::{DecodeError, EncodeError};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/featureprobe.link.rs"));
}

#[derive(Default)]
pub struct Codec {
    decode_len: Option<usize>,
}

impl Codec {
    pub fn encode(&self, packet: proto::packet::Packet) -> Result<Bytes, EncodeError> {
        let p = proto::Packet {
            packet: Some(packet),
        };
        let mut buf = BytesMut::with_capacity(p.encoded_len());
        p.encode_length_delimited(&mut buf)?;
        Ok(buf.freeze())
    }

    pub fn decode(
        &mut self,
        buf: &mut BytesMut,
    ) -> Result<Option<proto::packet::Packet>, DecodeError> {
        let len = if let Some(len) = self.decode_len.take() {
            len
        } else {
            prost::decode_length_delimiter(buf as &mut dyn bytes::Buf)?
        };

        if len > buf.len() {
            self.decode_len = Some(len);
            return Ok(None);
        }
        let b = &buf[0..len];
        let p = proto::Packet::decode(b)?.packet;
        buf.advance(len);
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use bytes::BufMut;

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
    fn test_decode() -> Result<(), prost::DecodeError> {
        let mut codec = Codec::default();
        let request = String::from("Hello, World!");
        let request = build_packet(request);
        let request_vector = codec.encode(request).unwrap();
        let request_vector = [request_vector].concat();
        let mut bm = BytesMut::from(request_vector.as_slice());

        let request_deserialized_result = match codec.decode(&mut bm) {
            Ok(request_deserialized_result) => request_deserialized_result,
            Err(e) => return Err(e),
        };
        println!("1. {:#?}", request_deserialized_result);

        Ok(())
    }

    #[test]
    fn test_decode_multiple() -> Result<(), prost::DecodeError> {
        let mut codec = Codec::default();
        let request = String::from("Hello, World!");
        let request = build_packet(request);
        let request_vector = codec.encode(request).unwrap();
        let request_vector = [request_vector.clone(), request_vector].concat();
        let mut bm = BytesMut::from(request_vector.as_slice());

        let request_deserialized_result = match codec.decode(&mut bm) {
            Ok(request_deserialized_result) => request_deserialized_result,
            Err(e) => return Err(e),
        };
        println!("1. {:#?}", request_deserialized_result);

        let request_deserialized_result = match codec.decode(&mut bm) {
            Ok(request_deserialized_result) => request_deserialized_result,
            Err(e) => return Err(e),
        };
        println!("2. {:#?}", request_deserialized_result);
        Ok(())
    }

    #[test]
    fn test_decode_partial() -> Result<(), prost::DecodeError> {
        let mut codec = Codec::default();
        let request = String::from("Hello, World!");
        let request = build_packet(request);
        let request_vector = codec.encode(request).unwrap();
        let request_vector = [request_vector].concat();

        let len = request_vector.len();
        let mut bm = BytesMut::from(&request_vector[0..len / 2]);
        let request_deserialized_result = codec.decode(&mut bm)?;
        println!("1. {:#?}", request_deserialized_result);

        bm.put(&request_vector[len / 2..]);
        let request_deserialized_result = codec.decode(&mut bm)?;
        println!("1. {:#?}", request_deserialized_result);

        Ok(())
    }
}
