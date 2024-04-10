use std::collections::HashMap;
use std::sync::Arc;

use rumqttc::{
    AsyncClient, Event, EventLoop, MqttOptions, NetworkOptions, Outgoing, Packet, Publish, QoS,
    Transport,
};
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::error;

use crate::{Error, Result};

#[derive(Debug, Default)]
pub struct MqttClientBuilder<'a> {
    certificate: Option<&'a [u8]>,
    private_key: Option<&'a [u8]>,
    certificate_authority: Option<&'a [u8]>,
    thing_name: Option<&'a str>,
    endpoint: Option<&'a str>,
    port: u16,
}

impl<'a> MqttClientBuilder<'a> {
    pub fn new() -> Self {
        Self {
            port: 8883,
            ..Default::default()
        }
    }

    pub const fn thing_name(mut self, name: &'a str) -> Self {
        self.thing_name = Some(name);
        self
    }

    pub const fn endpoint(mut self, endpoint: &'a str) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    pub const fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub const fn certificate(mut self, cert: &'a [u8]) -> Self {
        self.certificate = Some(cert);
        self
    }

    pub const fn private_key(mut self, key: &'a [u8]) -> Self {
        self.private_key = Some(key);
        self
    }

    pub const fn ca(mut self, ca: &'a [u8]) -> Self {
        self.certificate_authority = Some(ca);
        self
    }

    pub fn build(self) -> Result<MqttClient> {
        let mut options = MqttOptions::new(
            self.thing_name.ok_or(Error::ThingName)?,
            self.endpoint.ok_or(Error::Endpoint)?,
            self.port,
        );
        options.set_transport(Transport::tls(
            self.certificate_authority.ok_or(Error::Ca)?.to_vec(),
            (
                self.certificate.ok_or(Error::Certificate)?.to_vec(),
                self.private_key.ok_or(Error::PrivateKey)?.to_vec(),
            )
                .into(),
            None,
        ));

        let subscribers = Arc::new(Mutex::new(HashMap::new()));
        let (client, event_loop) = AsyncClient::new(options, 10);
        let worker = tokio::spawn(poll(event_loop, subscribers.clone()));

        Ok(MqttClient {
            client,
            worker: Arc::new(Mutex::new(worker)),
            subscribers,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MqttClient {
    client: AsyncClient,
    worker: Arc<Mutex<JoinHandle<()>>>,
    subscribers: Arc<Mutex<HashMap<String, Sender<Publish>>>>,
}

impl MqttClient {
    pub async fn subscribe(&self, topic: &str, qos: QoS) -> Result<Receiver<Publish>> {
        self.client
            .subscribe(topic, qos)
            .await
            .map_err(Error::from)?;

        if let Some(sender) = self.subscribers.lock().await.get(topic) {
            Ok(sender.subscribe())
        } else {
            let (tx, rx) = broadcast::channel::<Publish>(10);
            self.subscribers.lock().await.insert(topic.to_owned(), tx);
            Ok(rx)
        }
    }

    pub async fn publish(
        &self,
        topic: &str,
        qos: QoS,
        retain: bool,
        payload: bytes::Bytes,
    ) -> Result<()> {
        self.client
            .publish(topic, qos, retain, payload)
            .await
            .map_err(Error::from)
    }
}

async fn poll(
    mut event_loop: EventLoop,
    subscribers: Arc<Mutex<HashMap<String, Sender<Publish>>>>,
) {
    loop {
        match event_loop.poll().await {
            Ok(event) => {
                if let Event::Incoming(packet) = event {
                    match packet {
                        Packet::Publish(packet) => {
                            let mut subs = subscribers.lock().await;
                            let topic = packet.topic.clone();
                            if let Some(subscriber) = subs.get(&topic) {
                                if let Err(err) = subscriber.send(packet) {
                                    subs.remove(&topic);
                                    error!(error = %err, topic = %topic, "couldn't provide packet for a subscriber")
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(conn_err) => {
                // conn_err
            }
        }
    }
}
