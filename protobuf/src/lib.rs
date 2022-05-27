pub use tonic;

use prost::Message;
use std::io::Cursor;

pub mod packet {
    include!(concat!(env!("OUT_DIR"), "/featureprobe.link.packet.rs"));
}

pub mod service {
    include!(concat!(env!("OUT_DIR"), "/featureprobe.link.service.rs"));
}

pub fn build_packet(namespace: String) -> packet::Packet {
    let msg: packet::Message = packet::Message {
        namespace,
        path: "path".to_owned(),
        metadata: Default::default(),
        body: vec![1, 2, 3, 4],
    };
    let mut packet = packet::Packet::default();
    packet.packet = Some(packet::packet::Packet::Message(msg));
    packet
}

pub fn serialize(hello: &packet::Packet) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.reserve(hello.encoded_len());

    hello.encode(&mut buf).unwrap();
    buf
}

pub fn deserialize(buf: &[u8]) -> Result<packet::Packet, prost::DecodeError> {
    packet::Packet::decode(&mut Cursor::new(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() -> Result<(), prost::DecodeError> {
        let request = String::from("Hello, World!");

        let request = build_packet(request);
        let request_vector = serialize(&request);

        let request_deserialized_result = match deserialize(&request_vector) {
            Ok(request_deserialized_result) => request_deserialized_result,
            Err(e) => return Err(e),
        };
        println!("{:#?}", request_deserialized_result);
        Ok(())
    }
}
