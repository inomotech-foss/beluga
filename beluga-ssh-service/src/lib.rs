use core::net::SocketAddrV4;
use std::net::Ipv4Addr;

use beluga_tunnel::{Error, Result, Service};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Clone, Copy, Debug)]
pub struct SshService {
    port: u16,
}

impl SshService {
    pub const fn source() -> Self {
        SshService { port: 8022 }
    }

    pub const fn destination() -> Self {
        SshService { port: 22 }
    }

    pub const fn port(&self) -> u16 {
        self.port
    }
}

impl Service for SshService {
    async fn connect(
        &self,
        websocket_in: Sender<bytes::Bytes>,
        websocket_out: Receiver<bytes::Bytes>,
        close_service: Sender<()>,
    ) -> Result<()> {
        let stream = TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, self.port)).await?;

        let handle = tokio::spawn(serve(stream, websocket_in, websocket_out));

        tokio::spawn(async move {
            let _ = handle.await;
            let _ = close_service.send(()).await;
        });

        Ok(())
    }

    async fn bind(
        &self,
        websocket_in: Sender<bytes::Bytes>,
        websocket_out: Receiver<bytes::Bytes>,
        close_service: Sender<()>,
    ) -> Result<()> {
        let listener =
            TcpListener::bind::<SocketAddrV4>(SocketAddrV4::new(Ipv4Addr::LOCALHOST, self.port))
                .await?;
        let (stream, addr) = listener.accept().await?;
        tracing::info!("Got new client : {addr:?}");

        let handle = tokio::spawn(serve(stream, websocket_in, websocket_out));

        tokio::spawn(async move {
            let _ = handle.await;
            let _ = close_service.send(()).await;
        });

        Ok(())
    }
}

async fn serve(
    mut stream: TcpStream,
    websocket_in: Sender<bytes::Bytes>,
    mut websocket_out: Receiver<bytes::Bytes>,
) -> Result<()> {
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
}
