// @generated
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Message {
    #[prost(enumeration="message::Type", tag="1")]
    pub r#type: i32,
    #[prost(int32, tag="2")]
    pub stream_id: i32,
    #[prost(bool, tag="3")]
    pub ignorable: bool,
    #[prost(bytes="vec", tag="4")]
    pub payload: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="5")]
    pub service_id: ::prost::alloc::string::String,
    #[prost(string, repeated, tag="6")]
    pub available_service_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(uint32, tag="7")]
    pub connection_id: u32,
}
/// Nested message and enum types in `Message`.
pub mod message {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Type {
        Unknown = 0,
        Data = 1,
        StreamStart = 2,
        StreamReset = 3,
        SessionReset = 4,
        ServiceIds = 5,
        ConnectionStart = 6,
        ConnectionReset = 7,
    }
    impl Type {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Type::Unknown => "UNKNOWN",
                Type::Data => "DATA",
                Type::StreamStart => "STREAM_START",
                Type::StreamReset => "STREAM_RESET",
                Type::SessionReset => "SESSION_RESET",
                Type::ServiceIds => "SERVICE_IDS",
                Type::ConnectionStart => "CONNECTION_START",
                Type::ConnectionReset => "CONNECTION_RESET",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "UNKNOWN" => Some(Self::Unknown),
                "DATA" => Some(Self::Data),
                "STREAM_START" => Some(Self::StreamStart),
                "STREAM_RESET" => Some(Self::StreamReset),
                "SESSION_RESET" => Some(Self::SessionReset),
                "SERVICE_IDS" => Some(Self::ServiceIds),
                "CONNECTION_START" => Some(Self::ConnectionStart),
                "CONNECTION_RESET" => Some(Self::ConnectionReset),
                _ => None,
            }
        }
    }
}
// @@protoc_insertion_point(module)
