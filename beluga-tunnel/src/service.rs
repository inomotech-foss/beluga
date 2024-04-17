use tokio::sync::mpsc::{Receiver, Sender};
use crate::Result;

#[allow(async_fn_in_trait)]
pub trait Service {
    async fn connect(
        &mut self,
        websocket_in: Sender<bytes::Bytes>,
        websocket_out: Receiver<bytes::Bytes>,
    ) -> Result<()>;

    async fn handle(&mut self) -> Result<()>;
}
