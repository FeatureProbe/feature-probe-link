use anyhow::Error;
use bytes::{BufMut, BytesMut};
use server_base::{codec, proto::packet::Packet};
use tokio_util::codec::{Decoder as TokioDecoder, Encoder as TokioEncoder};

#[derive(Default)]
pub struct Codec {
    codec: codec::Codec,
}

impl TokioEncoder<Packet> for Codec {
    type Error = Error;

    fn encode(&mut self, item: Packet, dst: &mut BytesMut) -> Result<(), Self::Error> {
        log::trace!("encocde {:?}", item);
        let bytes = self.codec.encode(item)?;
        log::trace!("encocded {:?}", bytes);
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

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }
        let m = self.codec.decode(src).map_err(|e| e.into());
        log::debug!("decode {:?}", m);
        m
    }
}
