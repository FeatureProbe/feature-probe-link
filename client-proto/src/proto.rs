#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Packet {
    #[prost(oneof="packet::Packet", tags="1, 2, 3")]
    pub packet: ::core::option::Option<packet::Packet>,
}
/// Nested message and enum types in `Packet`.
pub mod packet {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Packet {
        #[prost(message, tag="1")]
        Ping(super::Ping),
        #[prost(message, tag="2")]
        Pong(super::Pong),
        #[prost(message, tag="3")]
        Message(super::Message),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ping {
    #[prost(uint64, tag="1")]
    pub timestamp: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Pong {
    #[prost(uint64, tag="1")]
    pub timestamp: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Message {
    #[prost(string, tag="1")]
    pub namespace: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub path: ::prost::alloc::string::String,
    #[prost(map="string, string", tag="3")]
    pub metadata: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    #[prost(bytes="vec", tag="4")]
    pub body: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, optional, tag="5")]
    pub expire_at: ::core::option::Option<u64>,
}
