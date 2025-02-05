use bytes::Bytes;
use tokio::sync;

use crate::Result;

/// A trait for giving a service both source and destination implementations for
/// communicating over the AWS localproxy protocol.
#[allow(async_fn_in_trait)]
pub trait Service {
    /// Manages the destination communication through a WebSocket.
    async fn connect(
        &self,
        websocket_in: sync::mpsc::Sender<Bytes>,
        websocket_out: sync::mpsc::Receiver<Bytes>,
        close_in: sync::mpsc::Sender<()>,
    ) -> Result<()>;

    /// Initiates and manages the source communication through a WebSocket.
    async fn bind(
        &self,
        websocket_in: sync::mpsc::Sender<Bytes>,
        websocket_out: sync::mpsc::Receiver<Bytes>,
        close_in: sync::mpsc::Sender<()>,
    ) -> Result<()>;
}
