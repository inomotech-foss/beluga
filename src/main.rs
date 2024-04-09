use anyhow::{bail, Context};
use bytes::{Buf, BufMut};
use prost::Message as _;
use core::net::SocketAddrV4;
use enumn::N;
use futures_util::{future, pin_mut, SinkExt, StreamExt, TryStreamExt};
use http::Request;
use rumqttc::{AsyncClient, MqttOptions, Packet, QoS, TlsConfiguration, Transport};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, Mutex},
    task::JoinHandle,
};
use tokio_tungstenite::tungstenite::{handshake::client::generate_key, Message};

mod proto;

/*
{
    "clientAccessToken": "destination-client-access-token",
    "clientMode": "destination",
    "region": "aws-region",
    "services": ["destination-service"]
}
*/

#[derive(Debug, Deserialize, Serialize)]
struct Notify {
    #[serde(rename = "clientAccessToken")]
    client_access_token: String,
    #[serde(rename = "clientMode")]
    client_mode: String,
    region: String,
    services: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut mqttoptions = MqttOptions::new("thing-name", "endpoint", 8883);
    mqttoptions.set_keep_alive(std::time::Duration::from_secs(5));

    let ca = include_bytes!("../AmazonRootCA1.pem");
    let client_cert = include_bytes!("../certificate.pem.crt");
    let client_key = include_bytes!("../private.pem.key");

    let transport = Transport::Tls(TlsConfiguration::Simple {
        ca: ca.to_vec(),
        alpn: None,
        client_auth: Some((client_cert.to_vec(), client_key.to_vec())),
    });

    mqttoptions.set_transport(transport);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    client
        .subscribe("$aws/things/thing-name/tunnels/notify", QoS::AtLeastOnce)
        .await?;

    loop {
        match eventloop.poll().await {
            Ok(v) => {
                // println!("Event = {v:?}");

                if let rumqttc::Event::Incoming(Packet::Publish(p)) = v {
                    let Notify {
                        client_access_token,
                        client_mode,
                        region,
                        services,
                    } = serde_json::from_slice::<Notify>(&p.payload).unwrap();

                    let req = Request::builder()
                        .method("GET")
                        .header("access-token", client_access_token)
                        // .header(":path", "/tunnel?local-proxy-mode=1") // dst mode
                        // .header("client-token", "some")
                        .header(http::header::HOST, format!("data.tunneling.iot.{region}.amazonaws.com"))
                        .header("Connection", "Upgrade")
                        .header("Upgrade", "websocket")
                        .header("Sec-WebSocket-Version", "13")
                        .header("Sec-WebSocket-Key", generate_key())
                        .header("Sec-WebSocket-Protocol", "aws.iot.securetunneling-3.0")
                        .uri(format!("wss://data.tunneling.iot.{region}.amazonaws.com/tunnel?local-proxy-mode=destination"))
                        // .uri("data.tunneling.iot.{region}.amazonaws.com/tunnel?local-proxy-mode=1")
                        // .header(key, value)
                        // .uri(format!("data.tunneling.iot.{region}.amazonaws.com"))
                        .body(())?;

                    tokio::spawn(async move {
                        let (stream, resp) =
                            tokio_tungstenite::connect_async_tls_with_config(req, None, true, None)
                                .await
                                .unwrap();

                        println!("Response {resp:?}");
                        println!("client mode {client_mode:?}");

                        let (mut write, mut read) = stream.split();

                        let (tx_out, mut rx_out) = mpsc::channel::<Vec<u8>>(100);
                        let (tx_in, rx_in) = mpsc::channel::<Vec<u8>>(100);

                        let tx_out = Arc::new(Mutex::new(tx_out));
                        let rx_in = Arc::new(Mutex::new(rx_in));

                        loop {
                            tokio::select! {
                                data = rx_out.recv() => {
                                    let msg = data.unwrap();
                                    write.send(Message::Binary(msg)).await.unwrap();
                                }
                                msg = read.next() => {
                                    if let Some(Ok(msg)) = msg {
                                        let mut bytes = bytes::Bytes::from_iter(msg.into_data());
                                        if let Ok(messages) = process_received_data(&mut bytes) {
                                            for msg in messages {
                                                println!("Message: {:?}", msg);
                                                match msg.r#type() {
                                                    proto::Type::Data => {
                                                        let data = msg.payload;
                                                        tx_in.send(data.to_vec()).await.unwrap();
                                                    }
                                                    proto::Type::StreamStart => {
                                                        println!("Stream start");
                                                        ssh(tx_out.clone(), rx_in.clone()).await;
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }



                                    }
                                }

                            }
                        }
                    });
                }
            }
            Err(e) => {
                println!("Error = {e:?}");
                break;
            }
        }
    }

    Ok(())
}

async fn ssh(tx: Arc<Mutex<mpsc::Sender<Vec<u8>>>>, rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>) {
    tokio::spawn(async move {
        let stream =
            TcpStream::connect::<SocketAddrV4>(SocketAddrV4::new([127, 0, 0, 1].into(), 22))
                .await
                .unwrap();

        let (mut reader, mut writer) = stream.into_split();
        let mut buff = [0; 1024];

        loop {
            // let data = rx.recv().await.unwrap();
            // println!("Write packet {:?}", &data);
            // writer.write_all(&data).await.unwrap();

            let size = reader.read(&mut buff).await.unwrap();

            if size == 0 {
                continue;
            }

            println!("Read SSH packet {:?}", &buff[..size]);
            let msg = proto::Message {
                r#type: proto::Type::Data as _,
                stream_id: 5,
                ignorable: false,
                payload: buff[..size].to_owned(),
                service_id: "SSH".to_owned(),
                available_service_ids: Vec::new(),
                connection_id: 0,
            };

            let mut buf = bytes::BytesMut::new();
            serialize_messages(&mut buf, &[&msg]).unwrap();
            tx.lock().await.send(buf.to_vec()).await.unwrap();

            let data = rx.lock().await.recv().await.unwrap();
            println!("Write packet {:?}", &data);
            writer.write_all(&data).await.unwrap();
        }
    });
}


fn process_received_data(data: &mut bytes::Bytes) -> anyhow::Result<Vec<proto::Message>> {
    let mut messages = Vec::new();

    while data.remaining() > (u16::BITS / 8) as usize {
        let len = usize::from(data.get_u16());
        let msg = proto::Message::decode(data.take(len))?;
        messages.push(msg);
    }
    Ok(messages)
}

fn serialize_messages(buf: &mut bytes::BytesMut, messages: &[&proto::Message]) -> anyhow::Result<()> {
    for msg in messages {
        let len = msg.encoded_len().try_into()?;
        buf.put_u16(len);
        msg.encode(buf)?;
    }
    Ok(())
}
