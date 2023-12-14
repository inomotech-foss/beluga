use std::ffi::c_void;

use strum::{AsRefStr, Display, EnumString};

use crate::Error;

macro_rules! enum_impl {
    ($visibility:vis enum $name:ident { $($variant:ident $(= $value:expr)*),+ $(,)* }) => {
        #[repr(C)]
        #[derive(Debug, Clone, Copy, Display, EnumString, AsRefStr)]
        $visibility enum $name {
            $($variant $(= $value)*,)+
        }

        impl TryFrom<i32> for $name {
            type Error = Error;
            fn try_from(value: i32) -> Result<Self, Self::Error> {
                match value {
                    $(
                        value if value == Self::$variant as i32 => Ok(Self::$variant),
                    )+
                    _ => Err(Error::UnrecognizedEnumValue(value as isize, stringify!($name)))
                }
            }
        }

        impl TryFrom<isize> for $name {
            type Error = Error;
            fn try_from(value: isize) -> Result<Self, Self::Error> {
                match value {
                    $(
                        value if value == Self::$variant as isize => Ok(Self::$variant),
                    )+
                    _ => Err(Error::UnrecognizedEnumValue(value, stringify!($name)))
                }
            }
        }

        impl TryFrom<u32> for $name {
            type Error = Error;
            fn try_from(value: u32) -> Result<Self, Self::Error> {
                match value {
                    $(
                        value if value == Self::$variant as u32 => Ok(Self::$variant),
                    )+
                    _ => Err(Error::UnrecognizedEnumValue(value as isize, stringify!($name)))
                }
            }
        }
    };
}

const AWS_C_MQTT_PACKAGE_ID: isize = 5;
const AWS_ERROR_ENUM_STRIDE_BITS: isize = 10;

#[allow(dead_code)]
#[repr(C)]
pub(crate) struct SharedPtr {
    data: *mut c_void,
    ref_count: *mut c_void,
}

#[allow(dead_code)]
#[repr(C)]
pub(crate) struct UniquePtr {
    data: *mut c_void,
}

#[repr(C)]
pub(crate) struct Buffer {
    data: *const u8,
    len: usize,
}

impl Buffer {
    #[allow(dead_code)]
    pub(crate) const fn empty(&self) -> Self {
        Self {
            data: std::ptr::null(),
            len: 0,
        }
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.data.is_null() || self.len == 0
    }
}

#[no_mangle]
extern "C" fn is_buffer_empty(buffer: Buffer) -> bool {
    buffer.is_empty()
}

impl From<&Vec<u8>> for Buffer {
    fn from(value: &Vec<u8>) -> Self {
        Self {
            data: value.as_ptr(),
            len: value.len(),
        }
    }
}

impl From<&[u8]> for Buffer {
    fn from(value: &[u8]) -> Self {
        Self {
            data: value.as_ptr(),
            len: value.len(),
        }
    }
}

impl From<Buffer> for Vec<u8> {
    fn from(value: Buffer) -> Self {
        unsafe { std::slice::from_raw_parts(value.data, value.len) }.to_vec()
    }
}

enum_impl!(
    pub enum Qos {
        AtMostOnce = 0x0,
        AtLeastOnce = 0x1,
        ExactlyOnce = 0x2,
        // reserved = 3
        QosFailure = 0x80, // Only used in SUBACK packets
    }
);

enum_impl!(
    pub enum AwsMqttError {
        MqttInvalidReservedBits = AWS_C_MQTT_PACKAGE_ID * (1_isize << AWS_ERROR_ENUM_STRIDE_BITS),
        BufferTooBig,
        InvalidRemainingLength,
        UnsupportedProtocolName,
        UnsupportedProtocolLevel,
        InvalidCredentials,
        InvalidQos,
        InvalidPacketType,
        InvalidTopic,
        Timeout,
        ProtocolError,
        NotConnected,
        AlreadyConnected,
        BuiltWithoutWebsockets,
        UnexpectedHangup,
        ConnectionShutdown,
        ConnectionDestroyed,
        ConnectionDisconnecting,
        CancelledForCleanSession,
        QueueFull,
        ClientOptionsValidation,
        ConnectOptionsValidation,
        DisconnectOptionsValidation,
        PublishOptionsValidation,
        SubscribeOptionsValidation,
        UnsubscribeOptionsValidation,
        UserPropertyValidation,
        PacketValidation,
        EncodeFailure,
        DecodeProtocolError,
        ConnackConnectionRefused,
        ConnackTimeout,
        PingResponseTimeout,
        UserRequestedStop,
        DisconnectReceived,
        ClientTerminated,
        OperationFailedDueToOfflineQueuePolicy,
        EncodeSizeUnsupportedPacketType,
        OperationProcessingFailure,
        InvalidInboundTopicAlias,
        InvalidOutboundTopicAlias,
        InvalidUtf8String,
        ConnectionResetForAdapterConnect,
        ConnectionResubscribeNoTopics,
        EndMqttRange =
            (AWS_C_MQTT_PACKAGE_ID + 1_isize) * (1_isize << AWS_ERROR_ENUM_STRIDE_BITS) - 1,
    }
);

enum_impl!(
    pub enum AwsMqttConnectReturnCode {
        Accepted,
        UnacceptableProtocolVersion,
        IdentifierRejected,
        ServerUnavailable,
        BadUsernameOrPassword,
        NotAuthorized,
    }
);
