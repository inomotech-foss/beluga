use core::net::SocketAddrV4;

use beluga_tunnel::{Error, Result, Service};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

#[derive(Debug, Default)]
pub struct SshService {
    handle: Option<JoinHandle<Result<()>>>,
}

impl Service for SshService {
    async fn connect(
        &mut self,
        websocket_in: tokio::sync::mpsc::Sender<bytes::Bytes>,
        mut websocket_out: tokio::sync::mpsc::Receiver<bytes::Bytes>,
    ) -> Result<()> {
        let mut stream =
            TcpStream::connect::<SocketAddrV4>(SocketAddrV4::new([127, 0, 0, 1].into(), 22))
                .await?;

        let handle = tokio::spawn(async move {
            let (mut reader, mut writer) = stream.split();
            let mut buff = bytes::BytesMut::with_capacity(2048);

            loop {
                tokio::select! {
                    res = reader.read_buf(&mut buff) => {
                        res.map_err(|err| {
                            Error::Service(std::io::Error::other(format!(
                                "couldn't read from ssh reader cause:\"{err}\"")))
                        })?;

                        websocket_in
                            .send(buff.clone().freeze())
                            .await
                            .map_err(|err| {
                                Error::Service(std::io::Error::other(format!(
                                    "couldn't write to websocket channel cause: \"{err}\"")))
                            })?;
                        buff.clear();
                    }
                    bytes = websocket_out.recv() => {
                        let Some(mut data) = bytes else {
                            return Ok::<(), Error>(());
                        };
                        writer.write_all_buf(&mut data).await.map_err(|err| {
                            Error::Service(std::io::Error::other(format!(
                                "couldn't write to ssh socket cause:\"{err}\"")))
                        })?;
                    }
                }
            }
        });

        self.handle = Some(handle);

        Ok(())
    }

    async fn handle(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.await.map_err(|err| {
                Error::Service(std::io::Error::other(format!(
                    "service handle error: \"{err}\""
                )))
            })??;
        }

        Ok(())
    }
}
