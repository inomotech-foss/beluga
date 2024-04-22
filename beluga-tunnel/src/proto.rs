#[derive(Clone, PartialEq, prost::Message)]
pub(crate) struct Message {
    #[prost(enumeration = "Type", tag = "1")]
    pub msg_type: i32,
    #[prost(int32, tag = "2")]
    pub stream_id: i32,
    #[prost(bool, tag = "3")]
    pub ignorable: bool,
    #[prost(bytes = "vec", tag = "4")]
    pub payload: Vec<u8>,
    #[prost(string, tag = "5")]
    pub service_id: String,
    #[prost(string, repeated, tag = "6")]
    pub available_service_ids: Vec<String>,
    #[prost(uint32, tag = "7")]
    pub connection_id: u32,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration, enumn::N,
)]
#[repr(i32)]
pub(crate) enum Type {
    Unknown = 0,
    Data = 1,
    StreamStart = 2,
    StreamReset = 3,
    SessionReset = 4,
    ServiceIds = 5,
    ConnectionStart = 6,
    ConnectionReset = 7,
}
