use core::time::Duration;
use std::sync::Arc;

pub use error::Error;
use manager::SubscriberManager;
use rumqttc::{
    AsyncClient, ConnectionError, Event, EventLoop, MqttOptions, Packet, SubscribeFilter, Transport,
};
pub use rumqttc::{Publish, QoS};
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{debug, error};

pub type Result<T> = core::result::Result<T, Error>;

type Sender = tokio::sync::broadcast::Sender<Result<Publish>>;
type Receiver = tokio::sync::broadcast::Receiver<Result<Publish>>;

mod error;
mod manager;

/// Represents an MQTT subscriber that can receive `Publish` messages.
#[derive(Debug)]
pub struct Subscriber(Vec<Receiver>);

impl Subscriber {
    /// Asynchronously receives a [`Publish`] message from one of
    /// the underlying receivers.
    pub async fn recv(&mut self) -> Result<Publish> {
        let (packet, ..) = futures::future::select_all(self.0.iter_mut().map(|receiver| {
            Box::pin(async move {
                // This loop continuously fetch the underlying receiver for
                // new messages. If a message is available, it
                // is returned. If no messages are available and the receiver
                // has been lagged, the loop continues to the
                // next iteration.
                loop {
                    let result = receiver.recv().await;
                    if let Err(broadcast::error::RecvError::Lagged(_)) = result {
                        continue;
                    } else {
                        return result;
                    }
                }
            })
        }))
        .await;

        packet.map_err(Error::from)?
    }
}

impl Clone for Subscriber {
    fn clone(&self) -> Self {
        Self(
            self.0
                .iter()
                .map(|receiver| receiver.resubscribe())
                .collect(),
        )
    }
}

/// A builder for creating an `MqttClient` with specific configurations.
#[derive(Debug, Default)]
pub struct MqttClientBuilder<'a> {
    certificate: Option<&'a [u8]>,
    private_key: Option<&'a [u8]>,
    certificate_authority: Option<&'a [u8]>,
    thing_name: Option<&'a str>,
    endpoint: Option<&'a str>,
    keep_alive: Option<Duration>,
    port: u16,
}

impl<'a> MqttClientBuilder<'a> {
    /// Creates a new `MqttClientBuilder` with default settings.
    ///
    /// # Returns
    /// A new `MqttClientBuilder` instance.
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

    /// Set number of seconds after which client should ping the broker
    /// if there is no other data exchange
    pub const fn keep_alive(mut self, time: Duration) -> Self {
        self.keep_alive = Some(time);
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

        if let Some(duration) = self.keep_alive {
            options.set_keep_alive(duration);
        }

        let (client, event_loop) = AsyncClient::new(options, 10);
        let (close_tx, close_rx) = mpsc::channel::<()>(1);
        let ctx = Arc::new(Mutex::new(MqttContext::new(
            SubscriberManager::with_close_tx(close_tx),
            None,
        )));

        tokio::spawn(poll(PollContext::new(
            client.clone(),
            event_loop,
            close_rx,
            ctx.clone(),
        )));

        Ok(MqttClient {
            client,
            ctx,
            thing_name: thing_name.to_owned(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MqttClient {
    client: AsyncClient,
    thing_name: String,
    ctx: Arc<Mutex<MqttContext>>,
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
    pub async fn subscribe(&self, topic: impl AsRef<str>, qos: QoS) -> Result<Subscriber> {
        let mut ctx = self.ctx.lock().await;
        ctx.check_connection().await?;

        if let Some(rx) = ctx.manager.receiver(topic.as_ref()) {
            Ok(Subscriber(vec![rx]))
        } else {
            self.client.subscribe(topic.as_ref(), qos).await?;
            Ok(Subscriber(vec![ctx.manager.subscribe(topic.as_ref())]))
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
        // This method checks the connection and locks the context. It is used
        // to ensure the connection is valid before performing other
        // operations on the context.
        let mut ctx = self.ctx.lock().await;
        ctx.check_connection().await?;

        let new_topics = ctx.manager.subscribed_diff(topics);
        self.client
            .subscribe_many(
                new_topics
                    .iter()
                    .map(|topic| SubscribeFilter::new(topic.to_owned(), qos)),
            )
            .await?;

        Ok(Subscriber(ctx.manager.subscribe_many(new_topics)))
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
        topic: impl AsRef<str>,
        qos: QoS,
        retain: bool,
        payload: bytes::Bytes,
    ) -> Result<()> {
        self.ctx.lock().await.check_connection().await?;

        self.client
            .publish(topic.as_ref(), qos, retain, payload)
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
    pub async fn unsubscribe(&self, topic: impl AsRef<str>) -> Result<()> {
        let mut ctx = self.ctx.lock().await;
        ctx.check_connection().await?;

        self.client.unsubscribe(topic.as_ref()).await?;
        ctx.manager.unsubscribe(topic.as_ref());
        Ok(())
    }

    /// Schedules unsubscription from a single MQTT topic.
    ///
    /// This method schedules unsubscription from the provided topic. The
    /// actual unsubscription will happen at a later time.
    ///
    /// # Arguments
    /// * `topic` - The topic name to unsubscribe from.
    pub fn schedule_unsubscribe(&self, topic: impl AsRef<str>) {
        let ctx = self.ctx.clone();
        let topic = topic.as_ref().to_owned();

        tokio::spawn(async move {
            ctx.lock().await.manager.schedule_unsubscribe(&topic);
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
            let mut ctx = self.ctx.lock().await;
            ctx.check_connection().await?;

            self.client.unsubscribe(topic.as_ref()).await?;
            ctx.manager.unsubscribe(topic.as_ref());
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
        let ctx = self.ctx.clone();

        let topics = topics
            .into_iter()
            .map(|topic| topic.as_ref().to_owned())
            .collect::<Vec<_>>();

        tokio::spawn(async move {
            ctx.lock().await.manager.schedule_unsubscribe_many(topics);
        });
    }

    /// Returns the name of the thing associated with this `MqttClient`.
    pub fn thing_name(&self) -> &str {
        &self.thing_name
    }
}

/// A struct that holds the context for an MQTT connection, including a
/// subscriber manager and any connection errors that have occurred.
#[derive(Debug)]
struct MqttContext {
    manager: SubscriberManager,
    error: Option<Error>,
}

impl MqttContext {
    fn new(manager: SubscriberManager, error: Option<Error>) -> Self {
        Self { manager, error }
    }

    /// Checks the connection status and returns an error if there is one.
    ///
    /// This function checks the connection status of the MQTT client and
    /// returns an error if there is one. If there is no error, it returns
    /// `Ok(())`.
    async fn check_connection(&mut self) -> Result<()> {
        self.error.take().map_or_else(|| Ok(()), Err)
    }
}

/// It is a struct that holds the necessary components for polling
/// the MQTT broker.
struct PollContext {
    client: AsyncClient,
    event_loop: EventLoop,
    close_rx: mpsc::Receiver<()>,
    mqtt_ctx: Arc<Mutex<MqttContext>>,
}

impl PollContext {
    fn new(
        client: AsyncClient,
        event_loop: EventLoop,
        close_rx: mpsc::Receiver<()>,
        mqtt_ctx: Arc<Mutex<MqttContext>>,
    ) -> Self {
        Self {
            client,
            event_loop,
            close_rx,
            mqtt_ctx,
        }
    }
}

/// Asynchronous function that handles polling the MQTT event loop.
async fn poll(mut ctx: PollContext) {
    loop {
        tokio::select! {
            event = ctx.event_loop.poll() => {
                // Handles an event by processing it and updating the connection error
                // state if necessary.
                if let Err(connection_err) = process_event(event, &mut ctx).await {
                    let mut mqtt_ctx = ctx.mqtt_ctx.lock().await;

                    // Sends an error to all subscribers of the MQTT subscription manager.
                    //
                    // When an error occurs in the MQTT connection, this code iterates through all
                    // the subscribers in the MQTT  manager and sends the error to each one of them.
                    let error = Error::from(connection_err);
                    for tx in mqtt_ctx.manager.subscribers() {
                        let _ = tx.send(Err(error.clone()));
                    }

                    mqtt_ctx.error = Some(error);
                }
            }
            _ = ctx.close_rx.recv() => {
                debug!("exit event poll loop");
                break;
            }
        }
    }
}

/// Processes an MQTT event.
async fn process_event(
    event: std::result::Result<Event, ConnectionError>,
    ctx: &mut PollContext,
) -> std::result::Result<(), ConnectionError> {
    match event {
        Ok(event) => {
            if let Event::Incoming(Packet::Publish(packet)) = event {
                let mut mqtt_ctx = ctx.mqtt_ctx.lock().await;
                process_packet(packet, &ctx.client, &mut mqtt_ctx.manager).await;
            }
        }
        Err(conn_err) => {
            error!(
                error = &conn_err as &dyn std::error::Error,
                "connection error during polling"
            );
            return Err(conn_err);
        }
    }

    Ok(())
}

/// Processes a received MQTT packet.
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
        if let Err(err) = sender.send(Ok(packet)) {
            manager.unsubscribe(&topic);
            error!(error = &err as &dyn std::error::Error, topic = %topic, "couldn't provide packet for a subscriber")
        }
    }
}
