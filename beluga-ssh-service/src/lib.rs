use core::net::SocketAddrV4;

use beluga_tunnel::{Error, Result, Service};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Default)]
pub struct SshService;

impl Service for SshService {
    async fn connect(
        &mut self,
        websocket_in: Sender<bytes::Bytes>,
        mut websocket_out: Receiver<bytes::Bytes>,
        close_service: Sender<()>,
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

        tokio::spawn(async move {
            let _ = handle.await;
            let _ = close_service.send(()).await;
        });

        Ok(())
    }
}
