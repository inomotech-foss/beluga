use rumqttc::ClientError;

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
    Mqtt(#[from] ClientError)
}
