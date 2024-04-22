#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Mqtt(#[from] beluga_mqtt::Error),
    #[error(transparent)]
    Tunnel(#[from] beluga_tunnel::Error),
    #[error(transparent)]
    TaskJoin(#[from] tokio::task::JoinError),
}
