use rumqttc::ClientError;
use tokio::sync::broadcast::error::RecvError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("")]
    Endpoint,
    #[error("")]
    ThingName,
    #[error("")]
    Certificate,
    #[error("")]
    PrivateKey,
    #[error("")]
    Ca,
    #[error(transparent)]
    Mqtt(#[from] ClientError),
    #[error(transparent)]
    Receive(#[from] RecvError),
}
