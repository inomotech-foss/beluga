// use std::sync::Arc;

// use rumqttc::{
//     AsyncClient, Event, EventLoop, MqttOptions, NetworkOptions, Outgoing,
// Packet, Transport, };

mod error;
mod mqtt;

pub use error::Error;

pub type Result<T> = core::result::Result<T, Error>;
// use tokio::sync::Mutex;
// use tokio::task::JoinHandle;

// type Result<T> = core::result::Result<T, Error>;

// #[derive(Debug, Clone)]
// pub struct MqttClient {
//     client: AsyncClient,
//     worker: Arc<Mutex<JoinHandle<()>>>,
// }

// #[derive(Debug, Default)]
// pub struct MqttClientBuilder<'a> {
//     certificate: Option<&'a [u8]>,
//     private_key: Option<&'a [u8]>,
//     certificate_authority: Option<&'a [u8]>,
//     thing_name: Option<&'a str>,
//     endpoint: Option<&'a str>,
//     port: u16,
// }

// impl<'a> MqttClientBuilder<'a> {
//     pub fn new() -> Self {
//         Self {
//             port: 8883,
//             ..Default::default()
//         }
//     }

//     pub const fn thing_name(mut self, name: &'a str) -> Self {
//         self.thing_name = Some(name);
//         self
//     }

//     pub const fn endpoint(mut self, endpoint: &'a str) -> Self {
//         self.endpoint = Some(endpoint);
//         self
//     }

//     pub const fn port(mut self, port: u16) -> Self {
//         self.port = port;
//         self
//     }

//     pub const fn certificate(mut self, cert: &'a [u8]) -> Self {
//         self.certificate = Some(cert);
//         self
//     }

//     pub const fn private_key(mut self, key: &'a [u8]) -> Self {
//         self.private_key = Some(key);
//         self
//     }

//     pub const fn ca(mut self, ca: &'a [u8]) -> Self {
//         self.certificate_authority = Some(ca);
//         self
//     }

//     pub fn build(self) -> Result<MqttClient> {
//         let mut options = MqttOptions::new(
//             self.thing_name.ok_or(Error::ThingName)?,
//             self.endpoint.ok_or(Error::Endpoint)?,
//             self.port,
//         );
//         options.set_transport(Transport::tls(
//             self.certificate_authority.ok_or(Error::Ca)?.to_vec(),
//             (
//                 self.certificate.ok_or(Error::Certificate)?.to_vec(),
//                 self.private_key.ok_or(Error::PrivateKey)?.to_vec(),
//             )
//                 .into(),
//             None,
//         ));

//         let (client, event_loop) = AsyncClient::new(options, 10);
//         let worker = tokio::spawn(poll(event_loop));

//         Ok(MqttClient {
//             client,
//             worker: Arc::new(Mutex::new(worker)),
//         })
//     }
// }

// async fn poll(mut event_loop: EventLoop) {
//     loop {
//         match event_loop.poll().await {
//             Ok(event) => match event {
//                 Event::Incoming(packet) => match packet {
//                     Packet::Connect(c) => {}
//                     Packet::ConnAck(_) => todo!(),
//                     Packet::Publish(p) => {}
//                     Packet::PubAck(_) => todo!(),
//                     Packet::PubRec(_) => todo!(),
//                     Packet::PubRel(_) => todo!(),
//                     Packet::PubComp(_) => todo!(),
//                     Packet::Subscribe(s) => {}
//                     Packet::SubAck(_) => todo!(),
//                     Packet::Unsubscribe(_) => todo!(),
//                     Packet::UnsubAck(_) => todo!(),
//                     Packet::PingReq => todo!(),
//                     Packet::PingResp => todo!(),
//                     Packet::Disconnect => todo!(),
//                 },
//                 Event::Outgoing(outgoing) => match outgoing {
//                     Outgoing::Publish(_) => todo!(),
//                     Outgoing::Subscribe(_) => todo!(),
//                     Outgoing::Unsubscribe(_) => todo!(),
//                     Outgoing::PubAck(_) => todo!(),
//                     Outgoing::PubRec(_) => todo!(),
//                     Outgoing::PubRel(_) => todo!(),
//                     Outgoing::PubComp(_) => todo!(),
//                     Outgoing::PingReq => todo!(),
//                     Outgoing::PingResp => todo!(),
//                     Outgoing::Disconnect => todo!(),
//                     Outgoing::AwaitAck(_) => todo!(),
//                 },
//             },
//             Err(conn_err) => {
//                 // conn_err
//             }
//         }
//     }
// }

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     let mut mqttoptions = MqttOptions::new(
//         "thing-name",
//         "endpoint",
//         8883,
//     );
//     mqttoptions.set_keep_alive(std::time::Duration::from_secs(5));

//     let ca = include_bytes!("../AmazonRootCA1.pem");
//     let client_cert = include_bytes!("../certificate.pem.crt");
//     let client_key = include_bytes!("../private.pem.key");

//     let transport = Transport::Tls(TlsConfiguration::Simple {
//         ca: ca.to_vec(),
//         alpn: None,
//         client_auth: Some((client_cert.to_vec(), client_key.to_vec())),
//     });

//     mqttoptions.set_transport(transport);

//     let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

//     client
//         .subscribe(
//             "$aws/things/thing-name/tunnels/notify",
//             QoS::AtLeastOnce,
//         )
//         .await?;

//     loop {
//         match eventloop.poll().await {
//             Ok(v) => {
//                 // println!("Event = {v:?}");

//                 if let rumqttc::Event::Incoming(Packet::Publish(p)) = v {
//                     let Notify {
//                         client_access_token,
//                         client_mode,
//                         region,
//                         services,
//                     } =
// serde_json::from_slice::<Notify>(&p.payload).unwrap();

//                     let req = Request::builder()
//                         .method("GET")
//                         .header("access-token", client_access_token)
//                         // .header(":path", "/tunnel?local-proxy-mode=1") //
// dst mode                         // .header("client-token", "some")
//                         .header(http::header::HOST,
// format!("data.tunneling.iot.{region}.amazonaws.com"))
// .header("Connection", "Upgrade")                         .header("Upgrade",
// "websocket")                         .header("Sec-WebSocket-Version", "13")
//                         .header("Sec-WebSocket-Key", generate_key())
//                         .header("Sec-WebSocket-Protocol",
// "aws.iot.securetunneling-3.0")
// .uri(format!("wss://data.tunneling.iot.{region}.amazonaws.com/tunnel?
// local-proxy-mode=destination"))                         //
// .uri("data.tunneling.iot.{region}.amazonaws.com/tunnel?local-proxy-mode=1")
//                         // .header(key, value)
//                         //
// .uri(format!("data.tunneling.iot.{region}.amazonaws.com"))
// .body(())?;

//                     tokio::spawn(async move {
//                         let (stream, resp) =
//
// tokio_tungstenite::connect_async_tls_with_config(req, None, true, None)
//                                 .await
//                                 .unwrap();

//                         println!("Response {resp:?}");
//                         println!("client mode {client_mode:?}");

//                         let (mut write, mut read) = stream.split();

//                         let (tx_out, mut rx_out) =
// mpsc::channel::<Vec<u8>>(100);                         let (tx_in, rx_in) =
// mpsc::channel::<Vec<u8>>(100);

//                         let tx_out = Arc::new(Mutex::new(tx_out));
//                         let rx_in = Arc::new(Mutex::new(rx_in));

//                         loop {
//                             tokio::select! {
//                                 data = rx_out.recv() => {
//                                     let msg = data.unwrap();
//
// write.send(Message::Binary(msg)).await.unwrap();
// }                                 msg = read.next() => {
//                                     if let Some(Ok(msg)) = msg {
//                                         let mut bytes =
// bytes::Bytes::from_iter(msg.into_data());
// if let Ok(messages) = process_received_data(&mut bytes) {
// for msg in messages {
// println!("Message: {:?}", msg);
// if let Some(MessageType::Data) = msg.msg_type {
// let data = msg.payload.unwrap();
// tx_in.send(data.to_vec()).await.unwrap();
// }

//                                                 if let
// Some(MessageType::StreamStart) = msg.msg_type {
// ssh(tx_out.clone(), rx_in.clone()).await;
// println!("Stream start");                                                 }
//                                             }
//                                         }

//                                     }
//                                 }

//                             }
//                         }
//                     });
//                 }
//             }
//             Err(e) => {
//                 println!("Error = {e:?}");
//                 break;
//             }
//         }
//     }

//     Ok(())
// }
