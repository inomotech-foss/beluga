use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub use error::Error;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, SubscribeFilter, Transport};
pub use rumqttc::{Publish, QoS};
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::error;

pub type Result<T> = core::result::Result<T, Error>;

mod error;

pub struct Subscriber(Vec<Receiver<Publish>>);

impl Subscriber {
    /// Asynchronously receives a [`Publish`] message from one of
    /// the underlying receivers.
    pub async fn recv(&mut self) -> Result<Publish> {
        let (packet, ..) = futures::future::select_all(
            self.0
                .iter_mut()
                .map(|receiver| Box::pin(async move { receiver.recv().await })),
        )
        .await;
        packet.map_err(Error::from)
    }
}

impl Clone for Subscriber {
    fn clone(&self) -> Self {
        Self(
            self.0
                .iter()
                .map(|receiver| receiver.resubscribe())
                .collect::<Vec<_>>(),
        )
    }
}

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

    /// Sets the name of the thing
    pub const fn thing_name(mut self, name: &'a str) -> Self {
        self.thing_name = Some(name);
        self
    }

    /// Sets the MQTT endpoint to connect to.
    pub const fn endpoint(mut self, endpoint: &'a str) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    /// Sets the MQTT port to connect to.
    pub const fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets the certificate to use for the MQTT connection.
    pub const fn certificate(mut self, cert: &'a [u8]) -> Self {
        self.certificate = Some(cert);
        self
    }

    /// Sets the private key to use for the MQTT connection.
    pub const fn private_key(mut self, key: &'a [u8]) -> Self {
        self.private_key = Some(key);
        self
    }

    /// Sets the certificate authority to use for the MQTT connection.
    pub const fn ca(mut self, ca: &'a [u8]) -> Self {
        self.certificate_authority = Some(ca);
        self
    }

    /// Builds an MQTT client with the configured options.
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
            subscribers,
            _worker: Arc::new(Mutex::new(worker)),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MqttClient {
    client: AsyncClient,
    subscribers: Arc<Mutex<HashMap<String, Sender<Publish>>>>,
    _worker: Arc<Mutex<JoinHandle<()>>>,
}

impl MqttClient {
    /// Subscribes to a given topic with the specified [QoS] level and returns a
    /// [Subscriber].
    ///
    /// # Arguments
    /// - `topic`: A string slice representing the topic to subscribe to.
    /// - `qos`: The Quality of Service level for the subscription.
    ///
    /// # Returns
    /// A [Result] containing a [Subscriber] if the subscription is successful,
    /// otherwise an [Error].
    pub async fn subscribe(&self, topic: &str, qos: QoS) -> Result<Subscriber> {
        let mut subs = self.subscribers.lock().await;

        if let Some(sender) = subs.get(topic) {
            Ok(Subscriber(vec![sender.subscribe()]))
        } else {
            self.client.subscribe(topic, qos).await?;
            let (tx, rx) = broadcast::channel::<Publish>(10);
            subs.insert(topic.to_owned(), tx);
            Ok(Subscriber(vec![rx]))
        }
    }

    /// Subscribes to multiple MQTT topics with the specified Quality of Service
    /// (QoS) level and returns a [Subscriber] that can be used to receive
    /// published messages.
    ///
    /// # Arguments
    /// - `topics`: An iterator of topic strings to subscribe to.
    /// - `qos`: The Quality of Service level for the subscriptions.
    ///
    /// # Returns
    /// A [Result] containing a [Subscriber] if the subscriptions are
    /// successful, otherwise an [Error].
    pub async fn subscribe_many(
        &self,
        topics: impl IntoIterator<Item = &str>,
        qos: QoS,
    ) -> Result<Subscriber> {
        let topics = HashSet::<&str>::from_iter(topics.into_iter());

        let mut subs = self.subscribers.lock().await;

        let new_topics = HashSet::from_iter(subs.keys().map(String::as_str))
            .symmetric_difference(&topics)
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        self.client
            .subscribe_many(
                new_topics
                    .iter()
                    .map(|topic| SubscribeFilter::new(topic.to_owned(), qos)),
            )
            .await
            .map_err(Error::from)?;

        let all_keys = subs
            .keys()
            .map(ToOwned::to_owned)
            .chain(new_topics.into_iter())
            .collect::<Vec<_>>();

        let receivers = all_keys
            .into_iter()
            .map(|topic| {
                let sender = subs.entry(topic.to_owned()).or_insert_with(|| {
                    let (tx, ..) = broadcast::channel::<Publish>(10);
                    tx
                });
                sender.subscribe()
            })
            .collect::<Vec<_>>();

        Ok(Subscriber(receivers))
    }

    /// Publishes a message to the MQTT broker.
    ///
    /// # Arguments
    /// - `topic`: The topic to publish the message to.
    /// - `qos`: The Quality of Service level for the publication.
    /// - `retain`: Whether the message should be retained by the broker.
    /// - `payload`: The message payload to publish.
    ///
    /// # Returns
    /// A [Result] indicating whether the publication was successful.
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

    /// Unsubscribes from a given topic.
    ///
    /// # Arguments
    /// - `topic`: The topic to unsubscribe from.
    ///
    /// # Returns
    /// A [Result] indicating whether the unsubscription was successful.
    pub async fn unsubscribe(&self, topic: &str) -> Result<()> {
        self.client.unsubscribe(topic).await?;
        let _ = self.subscribers.lock().await.remove(topic);
        Ok(())
    }
}

async fn poll(
    mut event_loop: EventLoop,
    subscribers: Arc<Mutex<HashMap<String, Sender<Publish>>>>,
) {
    loop {
        match event_loop.poll().await {
            Ok(event) => {
                if let Event::Incoming(Packet::Publish(packet)) = event {
                    let mut subs = subscribers.lock().await;
                    let topic = packet.topic.clone();
                    if let Some(subscriber) = subs.get(&topic) {
                        if let Err(err) = subscriber.send(packet) {
                            subs.remove(&topic);
                            error!(error = &err as &dyn std::error::Error, topic = %topic, "couldn't provide packet for a subscriber")
                        }
                    }
                }
            }
            Err(conn_err) => {
                error!(
                    error = &conn_err as &dyn std::error::Error,
                    "connection error during polling"
                );
            }
        }
    }
}
