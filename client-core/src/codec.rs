use anyhow::Error;
use bytes::{Buf, BufMut, BytesMut};
use client_proto as codec;
use client_proto::proto::packet::Packet;
use tokio_util::codec::{Decoder as TokioDecoder, Encoder as TokioEncoder};

pub struct Codec {}

impl Codec {
    pub fn new() -> Self {
        Self {}
    }
}

impl TokioEncoder<Packet> for Codec {
    type Error = Error;

    fn encode(&mut self, item: Packet, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes = codec::encode(item)?;
        if dst.remaining_mut() < bytes.len() {
            dst.reserve(bytes.len());
        }
        dst.put(bytes);
        Ok(())
    }
}

impl TokioDecoder for Codec {
    type Item = Packet;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.is_empty() {
            return Ok(None);
        }
        codec::decode(buf).map_err(|e| e.into())
    }
}

impl Default for Codec {
    fn default() -> Self {
        Codec::new()
    }
}
