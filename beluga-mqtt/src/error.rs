use std::sync::Arc;

pub use rumqttc::{ClientError, ConnectionError};
use tokio::sync::broadcast::error::RecvError;

/// An error that can occur when using the Beluga MQTT client.
///
/// This enum represents the various errors that can occur when working with the
/// Beluga MQTT client. It includes errors related to missing configuration,
/// connection issues, and MQTT-specific errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// The MQTT endpoint is missing.
    #[error("missing endpoint")]
    Endpoint,
    /// The thing name is missing.
    #[error("missing thing name")]
    ThingName,
    /// The subscriber doesn't contain any receivers.
    #[error("subscriber doesn't contain any receivers")]
    EmptySubscriber,
    /// An error occurred while connecting to the MQTT broker.
    #[error(transparent)]
    ConnectionError(Arc<ConnectionError>),
    /// An error occurred while interacting with the MQTT client.
    #[error(transparent)]
    Mqtt(Arc<ClientError>),
    /// An error occurred while receiving data from the MQTT broker.
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
