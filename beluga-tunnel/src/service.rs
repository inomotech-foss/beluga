use tokio::sync::mpsc::{Receiver, Sender};

use crate::Result;

#[allow(async_fn_in_trait)]
pub trait Service {
    /// Connects the service to a WebSocket and handles the incoming and
    /// outgoing data.
    ///
    /// # Arguments
    /// * `websocket_in` - A [`Sender`] for sending bytes to the WebSocket.
    /// * `websocket_out` - A [`Receiver`] for receiving bytes from the
    ///   WebSocket.
    /// * `close_service` - A [`Sender`] for signaling the service to close.
    ///
    /// # Returns
    /// A [`Result`] indicating whether the connection was successful.
    async fn connect(
        &mut self,
        websocket_in: Sender<bytes::Bytes>,
        websocket_out: Receiver<bytes::Bytes>,
        close_service: Sender<()>,
    ) -> Result<()>;
}
