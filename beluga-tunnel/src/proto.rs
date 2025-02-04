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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub(crate) enum Type {
    Unknown = 0,
    Data = 1,
    StreamStart = 2,
    StreamReset = 3,
    SessionReset = 4,
}
