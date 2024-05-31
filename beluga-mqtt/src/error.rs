use rumqttc::{ClientError, ConnectionError};
use tokio::sync::broadcast::error::RecvError;

#[derive(Debug, thiserror::Error)]
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
    ConnectionError(#[from] ConnectionError),
    #[error(transparent)]
    Mqtt(#[from] ClientError),
    #[error(transparent)]
    Receive(#[from] RecvError),
}
