use anyhow::Error;
use bytes::{BufMut, BytesMut};
use server_base::{codec, packet::Packet};
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
        match codec::encode(item) {
            Ok(bytes) => {
                if dst.remaining_mut() < bytes.len() {
                    dst.reserve(bytes.len());
                }
                dst.put(bytes);
                Ok(())
            }
            Err(e) => Err(Error::from(e)),
        }
    }
}

impl TokioDecoder for Codec {
    type Item = Packet;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        codec::decode(src).map_err(Error::from)
    }
}

impl Default for Codec {
    fn default() -> Self {
        Codec::new()
    }
}
