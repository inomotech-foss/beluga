use core::ops::DerefMut;
use std::sync::Arc;

pub use error::Error;
use manager::SubscriberManager;
use rumqttc::{
    AsyncClient, ConnectionError, Event, EventLoop, MqttOptions, Packet, SubscribeFilter, Transport,
};
pub use rumqttc::{Publish, QoS};
use tokio::sync::broadcast::Receiver;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error};

pub type Result<T> = core::result::Result<T, Error>;

mod error;
mod manager;

#[derive(Debug)]
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
        let thing_name = self.thing_name.ok_or(Error::ThingName)?;
        let mut options =
            MqttOptions::new(thing_name, self.endpoint.ok_or(Error::Endpoint)?, self.port);
        options.set_transport(Transport::tls(
            self.certificate_authority.ok_or(Error::Ca)?.to_vec(),
            (
                self.certificate.ok_or(Error::Certificate)?.to_vec(),
                self.private_key.ok_or(Error::PrivateKey)?.to_vec(),
            )
                .into(),
            None,
        ));

        let manager = Arc::new(Mutex::new(SubscriberManager::default()));
        let (client, event_loop) = AsyncClient::new(options, 10);
        let (close_tx, close_rx) = mpsc::channel::<()>(1);
        tokio::spawn(poll(
            client.clone(),
            event_loop,
            close_rx,
            Arc::clone(&manager),
        ));

        Ok(MqttClient {
            client,
            manager,
            close_tx,
            thing_name: thing_name.to_owned(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MqttClient {
    client: AsyncClient,
    thing_name: String,
    close_tx: mpsc::Sender<()>,
    manager: Arc<Mutex<SubscriberManager>>,
}

impl Drop for MqttClient {
    fn drop(&mut self) {
        let _ = self.close_tx.try_send(());
    }
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
        let mut manager = self.manager.lock().await;

        if let Some(rx) = manager.receiver(topic) {
            Ok(Subscriber(vec![rx]))
        } else {
            self.client.subscribe(topic, qos).await?;
            Ok(Subscriber(vec![manager.subscribe(topic)]))
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
    pub async fn subscribe_many<Iter>(&self, topics: Iter, qos: QoS) -> Result<Subscriber>
    where
        Iter: IntoIterator,
        Iter::Item: AsRef<str>,
    {
        let mut manager = self.manager.lock().await;

        let new_topics = manager.subscribed_diff(topics);
        self.client
            .subscribe_many(
                new_topics
                    .iter()
                    .map(|topic| SubscribeFilter::new(topic.to_owned(), qos)),
            )
            .await
            .map_err(Error::from)?;

        Ok(Subscriber(manager.subscribe_many(new_topics)))
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
        self.manager.lock().await.unsubscribe(topic);
        Ok(())
    }

    /// Schedules unsubscription from a single MQTT topic.
    ///
    /// This method schedules unsubscription from the provided topic. The
    /// actual unsubscription will happen at a later time.
    ///
    /// # Arguments
    /// * `topic` - The topic name to unsubscribe from.
    pub fn schedule_unsubscribe(&self, topic: &str) {
        let manager = self.manager.clone();
        let topic = topic.to_owned();

        tokio::spawn(async move {
            manager.lock().await.schedule_unsubscribe(&topic);
        });
    }

    /// Unsubscribes the client from multiple MQTT topics.
    /// If any unsubscribe operation fails, the function returns an error.
    ///
    /// # Arguments
    /// * `topics` - An iterator of topic strings to unsubscribe from.
    ///
    /// # Returns
    /// A [`Result`] indicating whether the unsubscribe operation was
    /// successful.
    pub async fn unsubscribe_many<Iter>(&self, topics: Iter) -> Result<()>
    where
        Iter: IntoIterator,
        Iter::Item: AsRef<str>,
    {
        let _ = futures::future::try_join_all(topics.into_iter().map(|topic| async move {
            self.client.unsubscribe(topic.as_ref()).await?;
            self.manager.lock().await.unsubscribe(topic.as_ref());
            Result::Ok(())
        }))
        .await?;

        Ok(())
    }

    /// Schedules unsubscription from multiple topics.
    ///
    /// This method schedules unsubscription from the provided topics. The
    /// actual unsubscription will happen at a later time
    ///
    /// # Arguments
    /// * `topics` - An iterator of topic names to unsubscribe from.
    pub fn schedule_unsubscribe_many<Iter>(&self, topics: Iter)
    where
        Iter: IntoIterator,
        Iter::Item: AsRef<str>,
    {
        let manager = self.manager.clone();
        let topics = topics
            .into_iter()
            .map(|topic| topic.as_ref().to_owned())
            .collect::<Vec<_>>();

        tokio::spawn(async move {
            manager.lock().await.schedule_unsubscribe_many(topics);
        });
    }

    /// Returns a reference to the name of the thing.
    pub fn thing_name(&self) -> &str {
        &self.thing_name
    }
}

async fn poll(
    client: AsyncClient,
    mut event_loop: EventLoop,
    mut close_rx: mpsc::Receiver<()>,
    manager: Arc<Mutex<SubscriberManager>>,
) {
    loop {
        tokio::select! {
            event = event_loop.poll() => {
                process_event(&client, event, manager.lock().await.deref_mut()).await;
            }
            _ = close_rx.recv() => {
                debug!("exit event poll loop");
                break;
            }
        }
    }
}

async fn process_event(
    client: &AsyncClient,
    event: std::result::Result<Event, ConnectionError>,
    manager: &mut SubscriberManager,
) {
    match event {
        Ok(event) => {
            if let Event::Incoming(Packet::Publish(packet)) = event {
                process_packet(packet, client, manager).await;
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

async fn process_packet(packet: Publish, client: &AsyncClient, manager: &mut SubscriberManager) {
    // Unsubscribes the client from the topics that are currently scheduled for
    // unsubscription.
    let removed_topics = manager
        .scheduled()
        .filter_map(|topic| {
            client
                .try_unsubscribe(topic)
                .ok()
                .and(Some(topic.to_owned()))
        })
        .collect::<Vec<_>>();

    for topic in removed_topics {
        manager.unsubscribe(&topic);
    }

    let topic = packet.topic.clone();

    if let Some(sender) = manager.sender(&topic) {
        if let Err(err) = sender.send(packet) {
            manager.unsubscribe(&topic);
            error!(error = &err as &dyn std::error::Error, topic = %topic, "couldn't provide packet for a subscriber")
        }
    }
}
