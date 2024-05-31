use std::sync::Arc;

pub use rumqttc::{ClientError, ConnectionError};
use tokio::sync::broadcast::error::RecvError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("missing endpoint")]
    Endpoint,
    #[error("missing thing name")]
    ThingName,
    #[error("missing certificate")]
    Certificate,
    #[error("missing private key")]
    PrivateKey,
    #[error("missing authority")]
    Ca,
    #[error(transparent)]
    ConnectionError(Arc<ConnectionError>),
    #[error(transparent)]
    Mqtt(Arc<ClientError>),
    #[error(transparent)]
    Receive(#[from] RecvError),
}

impl From<ClientError> for Error {
    fn from(value: ClientError) -> Self {
        Self::Mqtt(Arc::new(value))
    }
}

impl From<ConnectionError> for Error {
    fn from(value: ConnectionError) -> Self {
        Self::ConnectionError(Arc::new(value))
    }
}
