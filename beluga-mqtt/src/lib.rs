use core::time::Duration;
use std::collections::HashSet;
use std::sync::Arc;

pub use error::Error;
use manager::SubscriberManager;
use rumqttc::{
    AsyncClient, ConnectionError, Event, EventLoop, MqttOptions, Packet, StateError,
    SubscribeFilter, Transport,
};
pub use rumqttc::{Publish, QoS};
pub use subscriber::{OwnedSubscriber, Subscriber};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, warn};

mod error;
mod manager;
mod subscriber;

pub type Result<T> = core::result::Result<T, Error>;

type Sender = tokio::sync::broadcast::Sender<Result<Publish>>;
type Receiver = tokio::sync::broadcast::Receiver<Result<Publish>>;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_MIN_RECONNECT_DELAY: Duration = Duration::from_secs(1);
const DEFAULT_MAX_RECONNECT_DELAY: Duration = Duration::from_secs(300);

/// A builder for creating an `MqttClient` with specific configurations.
#[derive(Debug, Default)]
pub struct MqttClientBuilder<'a> {
    certificate: Option<&'a [u8]>,
    private_key: Option<&'a [u8]>,
    certificate_authority: Option<&'a [u8]>,
    thing_name: Option<&'a str>,
    endpoint: Option<&'a str>,
    keep_alive: Option<Duration>,
    min_reconnect_delay: Option<Duration>,
    max_reconnect_delay: Option<Duration>,
    connect_timeout: Option<Duration>,
    subscriber_capacity: usize,
    port: Option<u16>,
}

impl<'a> MqttClientBuilder<'a> {
    /// Creates a new `MqttClientBuilder` with default settings.
    ///
    /// # Returns
    /// A new `MqttClientBuilder` instance.
    pub fn new() -> Self {
        Self {
            subscriber_capacity: 10,
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
        self.port = Some(port);
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

    /// Sets the connect timeout.
    ///
    /// This timeout is used when connecting to the broker.
    /// The value only has a resolution of seconds.
    ///
    /// Any value less than 1 second will be treated as 0 seconds,
    /// which will cause the connection to fail instantly.
    ///
    /// Defaults to 5 seconds.
    pub const fn connect_timeout(mut self, time: Duration) -> Self {
        self.connect_timeout = Some(time);
        self
    }

    /// Sets the minimum delay between reconnection attempts.
    ///
    /// If set to `Duration::ZERO`, the client will always attempt to reconnect
    /// immediately without delay, effectively disabling backoff.
    /// This is not recommended for production use.
    ///
    /// Defaults to 1 second.
    pub const fn min_reconnect_delay(mut self, time: Duration) -> Self {
        self.min_reconnect_delay = Some(time);
        self
    }

    /// Sets the maximum delay between reconnection attempts.
    ///
    /// If set to a value less than the minimum reconnect delay, the minimum
    /// reconnect delay will be used instead.
    ///
    /// Defaults to 5 minutes.
    pub const fn max_reconnect_delay(mut self, time: Duration) -> Self {
        self.max_reconnect_delay = Some(time);
        self
    }

    /// Sets the maximum capacity for a [`Subscriber`] receiver channel.
    /// This allows controlling the buffer size for incoming messages to
    /// subscribers. A larger buffer can prevent message loss, but may
    /// consume more memory.
    pub const fn subscriber_capacity(mut self, size: usize) -> Self {
        self.subscriber_capacity = size;
        self
    }

    pub fn build(self) -> Result<MqttClient> {
        let thing_name = self.thing_name.ok_or(Error::ThingName)?;
        let endpoint = self.endpoint.ok_or(Error::Endpoint)?;

        // Determine if TLS is being used
        let is_tls = self.certificate_authority.is_some()
            && self.certificate.is_some()
            && self.private_key.is_some();

        // Set the default port based on whether TLS is used
        let port = self.port.unwrap_or(if is_tls { 8883 } else { 1883 });

        let mut options = MqttOptions::new(thing_name, endpoint, port);

        if is_tls {
            options.set_transport(Transport::tls(
                self.certificate_authority.ok_or(Error::Ca)?.to_vec(),
                (
                    self.certificate.ok_or(Error::Certificate)?.to_vec(),
                    self.private_key.ok_or(Error::PrivateKey)?.to_vec(),
                )
                    .into(),
                None,
            ));
        } else {
            options.set_transport(Transport::Tcp);
        }
        if let Some(duration) = self.keep_alive {
            options.set_keep_alive(duration);
        }

        let (client, mut event_loop) = AsyncClient::new(options, 10);
        event_loop.network_options.set_connection_timeout(
            self.connect_timeout
                .unwrap_or(DEFAULT_CONNECT_TIMEOUT)
                .as_secs(),
        );

        let (close_tx, close_rx) = mpsc::channel::<()>(1);
        let ctx = Arc::new(Mutex::new(MqttContext::new(
            SubscriberManager::with_close_tx(close_tx, self.subscriber_capacity),
            None,
        )));

        tokio::spawn(poll(PollContext::new(
            client.clone(),
            event_loop,
            close_rx,
            ctx.clone(),
            self.min_reconnect_delay
                .unwrap_or(DEFAULT_MIN_RECONNECT_DELAY),
            self.max_reconnect_delay
                .unwrap_or(DEFAULT_MAX_RECONNECT_DELAY),
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

        if ctx.manager.scheduled().contains(topic.as_ref()) {
            warn!(
                "topic \"{}\" is already scheduled for unsubscription",
                topic.as_ref()
            );
        }

        if let Some(rx) = ctx.manager.receiver(topic.as_ref()) {
            Ok(Subscriber(vec![rx]))
        } else {
            self.client.subscribe(topic.as_ref(), qos).await?;
            Ok(Subscriber(vec![ctx.manager.subscribe(topic.as_ref(), qos)]))
        }
    }

    /// Subscribes to a given MQTT topic with the specified Quality of Service
    /// [QoS] level and returns an [OwnedSubscriber] that can be used to receive
    /// published messages.
    ///
    /// # Arguments
    /// - `topic`: A string slice representing the topic to subscribe to.
    /// - `qos`: The Quality of Service level for the subscription.
    ///
    /// # Returns
    /// A [Result] containing an [OwnedSubscriber] if the subscription is
    /// successful, otherwise an [Error].
    pub async fn subscribe_owned(
        &self,
        topic: impl AsRef<str>,
        qos: QoS,
    ) -> Result<OwnedSubscriber> {
        let mut ctx = self.ctx.lock().await;
        ctx.check_connection().await?;

        if ctx.manager.scheduled().contains(topic.as_ref()) {
            warn!(
                "topic \"{}\" is already scheduled for unsubscription",
                topic.as_ref()
            );
        }

        if let Some(rx) = ctx.manager.receiver(topic.as_ref()) {
            Ok(OwnedSubscriber {
                subscriber: Subscriber(vec![rx]),
                topics: vec![topic.as_ref().to_owned()],
                mqtt: self.clone(),
            })
        } else {
            self.client.subscribe(topic.as_ref(), qos).await?;
            Ok(OwnedSubscriber {
                subscriber: Subscriber(vec![ctx.manager.subscribe(topic.as_ref(), qos)]),
                topics: vec![topic.as_ref().to_owned()],
                mqtt: self.clone(),
            })
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

        let mut new_topics = ctx.manager.subscribed_diff(topics);
        if new_topics.is_empty() {
            return Err(Error::ConnectionError(Arc::new(
                ConnectionError::MqttState(StateError::EmptySubscription),
            )));
        }
        warn_if_topics_scheduled(ctx.manager.scheduled(), &mut new_topics);

        self.client
            .subscribe_many(
                new_topics
                    .iter()
                    .map(|topic| SubscribeFilter::new(topic.to_owned(), qos)),
            )
            .await?;

        Ok(Subscriber(ctx.manager.subscribe_many(new_topics, qos)))
    }

    /// Subscribes to multiple MQTT topics with the specified Quality of Service
    /// [QoS] level and returns an [OwnedSubscriber] that can be used to receive
    /// published messages.
    ///
    /// # Arguments
    /// - `topics`: An iterator of topic strings to subscribe to.
    /// - `qos`: The Quality of Service level for the subscriptions.
    ///
    /// # Returns
    /// A [Result] containing an [OwnedSubscriber] if the subscriptions are
    /// successful, otherwise an [Error].
    pub async fn subscribe_many_owned<Iter>(
        &self,
        topics: Iter,
        qos: QoS,
    ) -> Result<OwnedSubscriber>
    where
        Iter: IntoIterator,
        Iter::Item: AsRef<str>,
    {
        // This method checks the connection and locks the context. It is used
        // to ensure the connection is valid before performing other
        // operations on the context.
        let mut ctx = self.ctx.lock().await;
        ctx.check_connection().await?;

        let mut new_topics = ctx.manager.subscribed_diff(topics);
        if new_topics.is_empty() {
            return Err(Error::ConnectionError(Arc::new(
                ConnectionError::MqttState(StateError::EmptySubscription),
            )));
        }
        warn_if_topics_scheduled(ctx.manager.scheduled(), &mut new_topics);

        self.client
            .subscribe_many(
                new_topics
                    .iter()
                    .map(|topic| SubscribeFilter::new(topic.to_owned(), qos)),
            )
            .await?;

        Ok(OwnedSubscriber {
            subscriber: Subscriber(ctx.manager.subscribe_many(&new_topics, qos)),
            topics: new_topics,
            mqtt: self.clone(),
        })
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

/// Warns the user if any of the new topics being subscribed to are already
/// scheduled for unsubscription.
///
/// This function checks if any of the new topics being subscribed to are
/// already scheduled for unsubscription. If so, it logs a warning message
/// with the list of topics that are already scheduled.
fn warn_if_topics_scheduled(scheduled: &HashSet<String>, new_topics: &mut [String]) {
    if new_topics.iter_mut().any(|topic| scheduled.contains(topic)) {
        let topics = new_topics
            .iter_mut()
            .filter(|topic| scheduled.contains(*topic))
            .collect::<Vec<_>>();
        warn!("some of topics \"{topics:?}\" is already scheduled for unsubscription");
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
    min_reconnect_delay: Duration,
    max_reconnect_delay: Duration,
    /// Current delay between reconnection attempts.
    ///
    /// `Duration::ZERO` means that the client is currently connected.
    reconnect_delay: Duration,
}

impl PollContext {
    fn new(
        client: AsyncClient,
        event_loop: EventLoop,
        close_rx: mpsc::Receiver<()>,
        mqtt_ctx: Arc<Mutex<MqttContext>>,
        min_reconnect_delay: Duration,
        max_reconnect_delay: Duration,
    ) -> Self {
        let max_reconnect_delay = std::cmp::max(min_reconnect_delay, max_reconnect_delay);
        Self {
            client,
            event_loop,
            close_rx,
            mqtt_ctx,
            min_reconnect_delay,
            max_reconnect_delay,
            reconnect_delay: Duration::ZERO,
        }
    }
}

/// Asynchronous function that handles polling the MQTT event loop.
async fn poll(mut ctx: PollContext) {
    loop {
        // apply reconnect delay before calling poll() (which will try to reconnect
        // without any delay)
        if ctx.reconnect_delay != Duration::ZERO {
            warn!("reconnecting in {:?}", ctx.reconnect_delay);
            tokio::select! {
                _ = tokio::time::sleep(ctx.reconnect_delay) => {}
                _ = ctx.close_rx.recv() => {
                    break;
                }
            }
        }

        tokio::select! {
            event = ctx.event_loop.poll() => {
                // Processes an MQTT event and updates the MQTT context accordingly.
                //
                // This code first processes the MQTT event received from the event loop. If the event is a
                // successful `Publish` packet, it is passed to the `process_packet` function to handle the
                // incoming MQTT message.
                //
                // If the event is an error, the error is logged and the MQTT context's error state is updated.
                let res = process_event(event, &mut ctx).await;
                let mut mqtt_ctx = ctx.mqtt_ctx.lock().await;
                match res {
                    Ok(()) => {
                        // reset the reconnect delay in case of successful connection
                        ctx.reconnect_delay = Duration::ZERO;

                        // If the MQTT connection was previously in an error state, this code attempts to
                        // resubscribe the client to all the topics that were previously subscribed to.
                        // If the resubscription fails, the error state is preserved.
                        if mqtt_ctx.error.take().is_some() {
                            if let Err(err) = resubscribe(ctx.client.clone(), mqtt_ctx.manager.topics_with_qos()).await {
                                mqtt_ctx.error = Some(err);
                            }
                        }
                    }
                    Err(connection_err) => {
                        // Sends an error to all subscribers of the MQTT subscription manager.
                        //
                        // When an error occurs in the MQTT connection, this code iterates through all
                        // the subscribers in the MQTT  manager and sends the error to each one of them.
                        let error = Error::from(connection_err);
                        for tx in mqtt_ctx.manager.subscribers() {
                            let _ = tx.send(Err(error.clone()));
                        }

                        mqtt_ctx.error = Some(error);

                        if ctx.reconnect_delay == Duration::ZERO {
                            // first time we're trying to reconnect, so set the delay to the minimum
                            ctx.reconnect_delay = ctx.min_reconnect_delay;
                        } else {
                            // increase the delay exponentially up to the maximum
                            ctx.reconnect_delay = std::cmp::min(ctx.reconnect_delay * 2, ctx.max_reconnect_delay);
                        }
                    }
                }
            }
            _ = ctx.close_rx.recv() => {
                break;
            }
        }
    }
    debug!("exit event poll loop");
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
        .iter()
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
            // let's try to unsubscribe in case of error
            manager.schedule_unsubscribe(&topic);
            error!(error = &err as &dyn std::error::Error, topic = %topic, "couldn't provide packet for a subscriber")
        }
    }
}

async fn resubscribe(client: AsyncClient, topics: impl Iterator<Item = (&str, QoS)>) -> Result<()> {
    futures::future::try_join_all(topics.map(|(topic, qos)| {
        let client = client.clone();
        async move { client.subscribe(topic, qos).await }
    }))
    .await?;

    Ok(())
}
