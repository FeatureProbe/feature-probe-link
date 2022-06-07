use bytes::{Buf, Bytes, BytesMut};
pub use prost::{DecodeError, EncodeError, Message};
pub use tonic;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/featureprobe.link.rs"));
}

const VARINT_MAX_LEN: usize = 10;

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
        if buf.len() < VARINT_MAX_LEN {
            return self.decode_length_delimiter(buf);
        }

        self._decode(buf)
    }

    fn decode_length_delimiter(
        &mut self,
        buf: &mut BytesMut,
    ) -> Result<Option<proto::packet::Packet>, DecodeError> {
        match self.decode_len {
            Some(_) => self._decode(buf),
            None => {
                let mut b = buf.clone();
                let new_buf = &mut b;
                match prost::decode_length_delimiter(new_buf) {
                    Ok(_) => self._decode(buf),
                    Err(_) => Ok(None),
                }
            }
        }
    }

    fn _decode(
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
    pub fn build_packet(namespace: String, body_len: usize) -> proto::packet::Packet {
        let message: proto::Message = proto::Message {
            namespace,
            path: "path".to_owned(),
            metadata: Default::default(),
            body: vec![1; body_len],
            expire_at: None,
        };
        proto::packet::Packet::Message(message)
    }

    #[test]
    fn test_decode_empty() -> Result<(), prost::DecodeError> {
        let mut codec = Codec::default();
        let mut bm = BytesMut::new();
        let result = codec.decode(&mut bm);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        Ok(())
    }

    #[test]
    fn test_decode() -> Result<(), prost::DecodeError> {
        let mut codec = Codec::default();
        let request = String::from("Hello, World!");
        let request = build_packet(request, 4);
        let request_vector = codec.encode(request).unwrap();
        let request_vector = [request_vector].concat();
        let mut bm = BytesMut::from(request_vector.as_slice());

        let result = codec.decode(&mut bm);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_decode_multiple() -> Result<(), prost::DecodeError> {
        let mut codec = Codec::default();
        let request = String::from("Hello, World!");
        let request = build_packet(request, 4);
        let request_vector = codec.encode(request).unwrap();
        let request_vector = [request_vector.clone(), request_vector].concat();
        let mut bm = BytesMut::from(request_vector.as_slice());

        let result = codec.decode(&mut bm);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        let result = codec.decode(&mut bm);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
        Ok(())
    }

    #[test]
    fn test_decode_partial() -> Result<(), prost::DecodeError> {
        let mut codec = Codec::default();
        let request = String::from("Hello, World!");
        let request = build_packet(request, 4);
        let request_vector = codec.encode(request).unwrap();
        let request_vector = [request_vector].concat();

        let len = request_vector.len();
        let mut bm = BytesMut::from(&request_vector[0..len / 2]);
        let result = codec.decode(&mut bm)?;
        assert!(result.is_none());

        bm.put(&request_vector[len / 2..]);
        let result = codec.decode(&mut bm)?;
        assert!(result.is_some());
        Ok(())
    }

    #[test]
    fn test_decode_partial_varint() -> Result<(), prost::DecodeError> {
        let mut codec = Codec::default();
        let request = String::from("Hello, World!");
        let request = build_packet(request, 1000);
        let request_vector = codec.encode(request).unwrap();
        let request_vector = [request_vector].concat();

        let mut bm = BytesMut::from(&request_vector[0..1]);
        let result = codec.decode(&mut bm)?;
        assert!(result.is_none());

        bm.put(&request_vector[1..]);
        let result = codec.decode(&mut bm)?;
        assert!(result.is_some());

        let mut bm = BytesMut::from(&request_vector[0..2]);
        let result = codec.decode(&mut bm)?;
        assert!(result.is_none());

        bm.put(&request_vector[2..]);
        let result = codec.decode(&mut bm)?;
        assert!(result.is_some());

        let mut bm = BytesMut::from(&request_vector[0..3]);
        let result = codec.decode(&mut bm)?;
        assert!(result.is_none());

        bm.put(&request_vector[3..]);
        let result = codec.decode(&mut bm)?;
        assert!(result.is_some());

        let mut bm = BytesMut::from(&request_vector[0..4]);
        let result = codec.decode(&mut bm)?;
        assert!(result.is_none());

        bm.put(&request_vector[4..]);
        let result = codec.decode(&mut bm)?;
        assert!(result.is_some());
        Ok(())
    }
}
