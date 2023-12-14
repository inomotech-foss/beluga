// required to make sure cargo will link the library even though we aren't using
// it in the Rust code.
extern crate aws_iot_device_sdk_sys;

use std::ffi::{c_void, IntoStringError, NulError};
use std::io::Error as IoError;
use std::string::FromUtf8Error;
use std::sync::OnceLock;

pub use common::{AwsMqttConnectReturnCode, AwsMqttError, Qos};
pub use mqtt::{
    ClientStatus, Config, ConfigBuilder, CreateMqttFuture, Message, MqttClient,
    OperationResponseFuture, SubscribeMessageFuture,
};
use thiserror::Error;
use tokio::time::error::Elapsed;

mod common;
mod mqtt;
pub mod tunnel;

type Result<T> = std::result::Result<T, Error>;

extern "C" {
    fn create_api_handle() -> *const c_void;
    fn drop_api_handle(handle: *const c_void);
}

struct ApiHandle(*const c_void);
unsafe impl std::marker::Send for ApiHandle {}
unsafe impl std::marker::Sync for ApiHandle {}

static HANDLE: OnceLock<ApiHandle> = OnceLock::new();

impl ApiHandle {
    fn handle() {
        let _ = HANDLE.get_or_init(|| ApiHandle(unsafe { create_api_handle() }));
    }
}

impl Drop for ApiHandle {
    fn drop(&mut self) {
        unsafe {
            drop_api_handle(self.0);
        }
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StringConversion(#[from] NulError),
    #[error(transparent)]
    Utf8(#[from] FromUtf8Error),
    #[error(transparent)]
    RustString(#[from] IntoStringError),
    #[error("couldn't create a mqtt client")]
    MqttClientCreate,
    #[error("couldn't create a tunnel client")]
    TunnelClientCreate,
    #[error("couldn't create a SSH tunnel")]
    TunnelCreate,
    #[error("the mqtt client hasn't connected")]
    NotConnected,
    #[error("AwsMqttError::{0}")]
    AwsMqttError(AwsMqttError),
    #[error("unknown mqtt error [{0}]")]
    AwsUnknownMqttError(i32),
    #[error("failure to receive a response from a publish")]
    AwsReceiveResponse,
    #[error("failure to receive a message from a subscribe future")]
    AwsReceiveMessage,
    #[error("miss endpoint for the Mqtt client")]
    MissEndpoint,
    #[error("miss client id for the Mqtt client")]
    MissClientId,
    #[error("miss the client's certificate")]
    MissCertificate,
    #[error("miss the client's private key")]
    MissPrivateKey,
    #[error("invalid topic [{0}]")]
    InvalidTopic(String),
    #[error(transparent)]
    Timeout(#[from] Elapsed),
    #[error("can't parse \"{0}\" to enum \"{1}\"")]
    UnrecognizedEnumValue(isize, &'static str),
    #[error("message hasn't any payload")]
    Payload,
    #[error("miss service id for tunneling")]
    ServiceId,
    #[error(transparent)]
    IoError(#[from] IoError),
}

impl From<AwsMqttError> for Error {
    fn from(value: AwsMqttError) -> Self {
        Self::AwsMqttError(value)
    }
}
