use std::io::Cursor;

use prost::Message;

pub mod message {
    include!(concat!(env!("OUT_DIR"), "/packet.rs"));
}

pub fn build_packet(namespace: String) -> message::Packet {
    let msg: message::Message = message::Message {
        namespace,
        path: "path".to_owned(),
        metadata: Default::default(),
        body: vec![1, 2, 3, 4],
    };
    let mut packet = message::Packet::default();
    packet.packet = Some(message::packet::Packet::Message(msg));
    packet
}

pub fn serialize(hello: &message::Packet) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.reserve(hello.encoded_len());

    hello.encode(&mut buf).unwrap();
    buf
}

pub fn deserialize(buf: &[u8]) -> Result<message::Packet, prost::DecodeError> {
    message::Packet::decode(&mut Cursor::new(buf))
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
