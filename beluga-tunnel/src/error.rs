use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bad client mode, couldn't be a source mode")]
    ClientMode,
    #[error("services missing the SSH service")]
    NoSSHService,
    #[error("couldn't deserialize initial tunnel request cause of \"{0}\"")]
    NotifyDeserialization(#[from] serde_json::Error),
    #[error("websocket closed")]
    WebSocketClosed,
    #[error("service closed")]
    ServiceClosed,
    #[error("websocket receives unknown message")]
    WebSocketUnknownMessage,
    #[error("websocket can't send message to the service, cause \"{0}\"")]
    WebSocketSend(#[from] mpsc::error::SendError<bytes::Bytes>),
    #[error(transparent)]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error(transparent)]
    Service(#[from] std::io::Error),
    #[error(transparent)]
    DecodeProto(#[from] prost::DecodeError),
    #[error(transparent)]
    EncodeProto(#[from] prost::EncodeError),
    #[error("can't parse encoded length")]
    EncodedLength,
    #[error(transparent)]
    Http(#[from] tokio_tungstenite::tungstenite::http::Error),
}
