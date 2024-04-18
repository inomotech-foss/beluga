use bytes::{Buf, BufMut};
use futures::prelude::sink::SinkExt;
use futures::prelude::stream::StreamExt;
use prost::Message as _;
use proto::{Message as Msg, Type};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::handshake::client::{generate_key, Request};
use tokio_tungstenite::tungstenite::{http, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

mod error;
mod proto;
mod service;

// public use
pub use error::Error;
pub use service::Service;
pub type Result<T> = core::result::Result<T, Error>;

pub struct Tunnel {
    web_socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Tunnel {
    /// Creates a new [`Tunnel`] instance from the provided payload.
    ///
    /// The payload is expected to be a JSON-encoded [`Notify`] struct, which
    /// contains the necessary information to establish a secure tunnel
    /// connection with AWS IoT Secure Tunneling.
    ///
    /// The function performs the following checks:
    /// - Ensures the `client_mode` is "destination"
    /// - Ensures the `services` list contains the "SSH" service
    ///
    /// If the checks pass, the function creates a new WebSocket connection to
    /// the AWS IoT Secure Tunneling endpoint and returns a [`Tunnel`]
    /// instance.
    pub async fn new(payload: bytes::Bytes) -> Result<Self> {
        let Notify {
            client_access_token,
            client_mode,
            region,
            services,
        } = serde_json::from_slice::<Notify>(&payload)?;

        if client_mode != "destination" {
            return Err(Error::ClientMode);
        }

        if !services
            .iter()
            .map(String::as_str)
            .any(|service| service == "SSH")
        {
            return Err(Error::NoSSHService);
        }

        // Constructs an HTTP request for establishing a WebSocket connection to the AWS
        // IoT Secure Tunneling service. This request can be used to establish a
        // WebSocket connection to the AWS IoT Secure Tunneling service, which
        // is a necessary step for setting up a secure tunnel between a client and a
        // remote device.
        let req = Request::builder()
            .method("GET")
            .header("access-token", client_access_token)
            .header(http::header::HOST, format!("data.tunneling.iot.{region}.amazonaws.com"))
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", generate_key())
            .header("Sec-WebSocket-Protocol", "aws.iot.securetunneling-3.0")
            .uri(format!("wss://data.tunneling.iot.{region}.amazonaws.com/tunnel?local-proxy-mode=destination"))
            .body(())?;

        let (web_socket, ..) =
            tokio_tungstenite::connect_async_tls_with_config(req, None, true, None).await?;

        Ok(Tunnel { web_socket })
    }

    /// Starts the [`Tunnel`] service, handling incoming and outgoing
    /// messages.
    ///
    /// This function is responsible for managing the WebSocket connection,
    /// processing received data, and forwarding messages to the underlying
    /// service.
    ///
    /// The function takes a [`Service`] instance as a parameter, which is
    /// responsible for handling the actual processing of the messages. The
    /// [`Service`] trait defines the interface for this processing, including
    /// methods for connecting to the service, handling incoming messages,
    /// and closing the connection.
    pub async fn start<S>(self, mut service: S) -> Result<()>
    where
        S: Service,
    {
        let (mut write, mut read) = self.web_socket.split();
        let (tx_out, websocket_out) = mpsc::channel::<bytes::Bytes>(10);
        let (websocket_in, mut rx_in) = mpsc::channel::<bytes::Bytes>(10);
        let (close_tx, mut close_rx) = mpsc::channel::<()>(1);

        let mut websocket_in = Some(websocket_in);
        let mut websocket_out = Some(websocket_out);
        let mut stream_id = 0;

        loop {
            tokio::select! {
                msg = read.next() => {
                    let bytes = msg
                        .ok_or(Error::WebSocketClosed)?
                        .map(|msg| bytes::Bytes::from_iter(msg.into_data()))?;

                    let messages = process_received_data(bytes)?;

                    for msg in messages {
                        match msg.msg_type() {
                            Type::Unknown => {
                                return Err(Error::WebSocketUnknownMessage);
                            }
                            Type::Data => {
                                tx_out.send(bytes::Bytes::copy_from_slice(&msg.payload)).await?;
                            }
                            Type::StreamStart => {
                                let Some((websocket_in, websocket_out)) = websocket_in.take().zip(websocket_out.take())
                                else {
                                    return Err(Error::Service(std::io::Error::other(
                                        "restart of the same tunnel isn't supported",
                                    )));
                                };
                                stream_id = msg.stream_id;
                                service.connect(websocket_in, websocket_out, close_tx.clone()).await?;
                            }
                            Type::StreamReset => {
                                return Ok(());
                            }
                            Type::SessionReset => {
                                return Ok(());
                            }
                            Type::ServiceIds => {
                                // pass
                            }
                            Type::ConnectionStart => {
                                // pass
                            }
                            Type::ConnectionReset => {
                                return Ok(());
                            }
                        }
                    }
                }
                data = rx_in.recv() => {
                    let bytes = data.ok_or(Error::ServiceClosed)?;
                    let msg = Msg {
                        msg_type: Type::Data.into(),
                        ignorable: false,
                        stream_id,
                        payload: bytes.to_vec(),
                        ..Default::default()
                    };

                    let mut out_payload = bytes::BytesMut::new();
                    serialize_messages(&mut out_payload, msg)?;
                    write.send(Message::Binary(out_payload.to_vec())).await?;
                }
                _ = close_rx.recv() => {
                    return Err(Error::Service(std::io::Error::other(
                        "underlying communication service is closed",
                    )));
                }
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Notify {
    #[serde(rename = "clientAccessToken")]
    client_access_token: String,
    #[serde(rename = "clientMode")]
    client_mode: String,
    region: String,
    services: Vec<String>,
}

fn process_received_data(mut data: bytes::Bytes) -> Result<Vec<Msg>> {
    let mut messages = Vec::new();

    while data.remaining() > (u16::BITS / 8) as usize {
        let len = usize::from(data.get_u16());
        let Some(raw) = data.get(..len) else {
            // remote lied about the length
            break;
        };
        let msg = Msg::decode(raw)?;
        data.advance(len);
        messages.push(msg);
    }

    Ok(messages)
}

fn serialize_messages(buf: &mut bytes::BytesMut, message: Msg) -> Result<()> {
    let len = message
        .encoded_len()
        .try_into()
        .map_err(|_| Error::EncodedLength)?;
    buf.put_u16(len);
    message.encode(buf)?;
    Ok(())
}
