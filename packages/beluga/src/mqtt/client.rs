use std::collections::{HashMap, HashSet};
use std::ffi::{c_char, c_void, CString, NulError};
use std::fmt::Debug;
use std::future;
use std::ops::Deref;
use std::sync::Arc;

use ::futures::future::BoxFuture;
use crossbeam::queue::SegQueue;
use itertools::Itertools;
use parking_lot::{const_fair_mutex, const_mutex, FairMutex, Mutex};
use smallvec::SmallVec;
use strum::{AsRefStr, Display, EnumString};
use tokio::sync::*;

use super::callbacks::{
    create_closed_callback, create_completed_callback, create_interrupted_callback,
    create_message_callback, create_notify_callback, create_resumed_callback,
    create_sub_ack_callback, Interface,
};
use super::Message;
use crate::common::{Buffer, SharedPtr};
use crate::{
    ApiHandle, AwsMqttError, Config, CreateMqttFuture, Error, OperationResponseFuture, Qos, Result,
    SubscribeMessageFuture,
};

extern "C" {
    fn internal_mqtt_client(
        config: ClientConfig,
        interface: *const c_void,
    ) -> *const InternalMqttClient;

    fn subscribe(client: *const InternalMqttClient, topic: *const c_char, qos: Qos) -> u16;
    fn subscribe_multiple(
        client: *const InternalMqttClient,
        topics: *mut *const c_char,
        topics_len: usize,
        qos: Qos,
    ) -> u16;
    fn unsubscribe(client: *const InternalMqttClient, topic: *const c_char) -> u16;

    fn publish(
        client: *const InternalMqttClient,
        topic: *const c_char,
        qos: Qos,
        retain: bool,
        data: Buffer,
    ) -> u16;

    fn disconnect(client: *const InternalMqttClient);
    fn drop_client(client: *const InternalMqttClient);
}

#[repr(C)]
pub(super) struct ClientConfig {
    pub(super) endpoint: *const c_char,
    pub(super) port: u16,
    pub(super) client_id: *const c_char,
    pub(super) clean_session: bool,
    pub(super) keep_alive_s: u16,
    pub(super) ping_timeout_ms: u32,
    pub(super) username: *const c_char,
    pub(super) password: *const c_char,
    pub(super) certificate: Buffer,
    pub(super) private_key: Buffer,
}

#[repr(C)]
pub(crate) struct InternalMqttClient {
    connection: SharedPtr,
    interface: *const c_void,
}

#[derive(Debug)]
pub(crate) struct InternalMqttClientPointer {
    internal_client: *const InternalMqttClient,
}

impl Deref for InternalMqttClientPointer {
    type Target = *const InternalMqttClient;
    fn deref(&self) -> &Self::Target {
        &self.internal_client
    }
}

unsafe impl Send for InternalMqttClientPointer {}
unsafe impl Sync for InternalMqttClientPointer {}

#[derive(Debug, Default, Clone, Copy, Display, EnumString, AsRefStr)]
pub enum ClientStatus {
    Connected,
    Interrupted,
    Closed,
    #[default]
    Unknown,
}

#[derive(Debug)]
pub(super) struct Subscriber {
    // The `topics` field in the `Subscriber` struct is a `SmallVec` of `SmallVec` arrays. It is
    // used to store the topics that the subscriber is interested in receiving messages for.
    // Each topic is represented as a byte array with a maximum length of 10 bytes. The
    // `SmallVec` allows for efficient storage of topics, with the ability to store up to 3
    // topics without allocating additional memory.
    topics: SmallVec<[SmallVec<[u8; 10]>; 3]>,
    sender: oneshot::Sender<Message>,
}

impl Subscriber {
    pub(super) fn new(topics: &[&str], sender: oneshot::Sender<Message>) -> Self {
        Self {
            topics: topics
                .iter()
                .map(|topic| SmallVec::from_slice(topic.as_bytes()))
                .collect::<_>(),
            sender,
        }
    }

    pub(super) fn contains(&self, topic: &str) -> bool {
        self.topics
            .iter()
            .any(|it| it.as_slice() == topic.as_bytes())
    }

    pub(super) fn send_message(self, message: Message) {
        let _ = self.sender.send(message);
    }

    pub(super) fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

/// The `MqttClient` struct represents an MQTT client, with various fields for
/// internal client management and communication.
#[derive(Clone)]
pub struct MqttClient {
    internal_client: Arc<Mutex<InternalMqttClientPointer>>,
    _interface: Arc<Mutex<Interface>>,
    status: Arc<FairMutex<ClientStatus>>,
    publish_notifiers: Arc<FairMutex<HashMap<u16, oneshot::Sender<i32>>>>,
    subscribers: Arc<SegQueue<Subscriber>>,
    subscription: Arc<FairMutex<HashSet<String>>>,
    unsubscribe_notifiers: Arc<FairMutex<HashMap<u16, oneshot::Sender<i32>>>>,
}

impl Drop for MqttClient {
    fn drop(&mut self) {
        unsafe {
            // let's first try to disconnect the client with 150 milliseconds timeout
            let mut guard = self.internal_client.lock();
            disconnect(guard.internal_client);
            // drop the client itself
            drop_client(guard.internal_client);
            guard.internal_client = std::ptr::null();
        }
    }
}

impl MqttClient {
    pub fn create(config: Config) -> BoxFuture<'static, Result<Arc<MqttClient>>> {
        ApiHandle::handle();

        let client_config = ClientConfig::from(&config);
        let status = Arc::new(const_fair_mutex(Default::default()));
        let publish_notifiers = Arc::new(const_fair_mutex(Default::default()));
        let subscribers = Arc::new(SegQueue::new());
        let subscription = Arc::new(const_fair_mutex(Default::default()));
        let unsubscribe_notifiers = Arc::new(const_fair_mutex(Default::default()));

        let (client_tx, client_rx) = oneshot::channel::<ClientStatus>();

        let interface = Arc::new(const_mutex(Interface {
            completed: Box::new(create_completed_callback(
                status.clone(),
                Arc::new(const_fair_mutex(client_tx.into())),
            )),
            closed: Box::new(create_closed_callback(status.clone())),
            interrupted: Box::new(create_interrupted_callback(status.clone())),
            resumed: Box::new(create_resumed_callback(status.clone())),
            message: Box::new(create_message_callback(subscribers.clone())),
            sub_ack: Box::new(create_sub_ack_callback()),
            publish: Box::new(create_notify_callback(publish_notifiers.clone())),
            unsubscribe: Box::new(create_notify_callback(unsubscribe_notifiers.clone())),
        }));

        let internal_client = Arc::new(const_mutex(InternalMqttClientPointer {
            internal_client: unsafe {
                internal_mqtt_client(
                    client_config,
                    (&(*interface.lock()) as *const Interface).cast(),
                )
            },
        }));

        if internal_client.lock().is_null() {
            return Box::pin(future::ready(Err(Error::MqttClientCreate)));
        }

        Box::pin(CreateMqttFuture::new(
            MqttClient {
                _interface: interface,
                internal_client,
                status,
                publish_notifiers,
                subscribers,
                subscription,
                unsubscribe_notifiers,
            },
            client_rx,
        ))
    }

    /// Publishes a message to a specified topic with the given
    /// quality of service, retain flag, data, and optional timeout.
    ///
    /// # Arguments:
    ///
    /// - `topic`: The `topic` parameter is a string that represents the topic
    ///   to which the data will be
    /// published.
    /// - `qos`: The `qos` parameter stands for Quality of Service. It
    ///   determines the level of guarantee for
    /// message delivery.
    /// - `retain`: The `retain` parameter determines whether the message should
    ///   be retained by the broker
    /// or not. If `retain` is set to `true`, the broker will store the last
    /// message published on the topic and deliver it to any new
    /// subscribers.
    /// - `data`: Payload or message that you want to publish to the specified
    ///   topic.
    /// - `timeout`: The `timeout` parameter is an optional duration that
    ///   specifies the maximum amount of
    /// time to wait for the publish operation to complete. If the timeout is
    /// reached and the operation has not completed, an error will be
    /// returned.
    ///
    /// # Returns:
    ///
    /// Result indicating whether the operation was successful or not.
    pub fn publish(
        &self,
        topic: &str,
        qos: Qos,
        retain: bool,
        data: &[u8],
    ) -> BoxFuture<'static, Result<()>> {
        if !self.is_connected() {
            return Box::pin(future::ready(Err(Error::NotConnected)));
        }

        let Ok(c_topic) = CString::new(topic) else {
            return Box::pin(future::ready(Err(Error::InvalidTopic(topic.to_owned()))));
        };

        let packet_id = {
            let guard = self.internal_client.lock();
            unsafe {
                publish(
                    guard.internal_client,
                    c_topic.as_c_str().as_ptr(),
                    qos,
                    retain,
                    data.into(),
                )
            }
        };

        if packet_id == 0 {
            return Box::pin(future::ready(Err(AwsMqttError::ProtocolError.into())));
        }

        let (publish_tx, publish_rx) = oneshot::channel::<i32>();
        self.publish_notifiers.lock().insert(packet_id, publish_tx);

        Box::pin(OperationResponseFuture::new(publish_rx))
    }

    /// Subscribes to a topic with a specified quality of service [`Qos`] and an
    /// optional timeout.
    ///
    /// # Arguments:
    ///
    /// - `topic`: A string representing the topic to subscribe to.
    /// - `qos`: The `qos` parameter stands for Quality of Service. It is used
    ///   to specify the level of
    /// guarantee for message delivery.
    /// * `timeout`: The `timeout` parameter is an optional duration that
    ///   specifies the maximum amount of
    /// time to wait for a response from the server when subscribing to a topic.
    /// If the timeout is reached and no response is received within that
    /// time, an error will be returned.
    ///
    /// # Returns:
    ///
    /// returns the [`Message`].
    pub fn subscribe(&self, topic: &str, qos: Qos) -> BoxFuture<Result<Message>> {
        if !self.is_connected() {
            return Box::pin(future::ready(Err(Error::NotConnected)));
        }

        let Ok(c_topic) = CString::new(topic) else {
            return Box::pin(future::ready(Err(Error::InvalidTopic(topic.to_owned()))));
        };

        let mut subscription = self.subscription.lock();

        if !subscription.contains(topic) {
            let packet_id = {
                let guard = self.internal_client.lock();
                unsafe { subscribe(guard.internal_client, c_topic.as_ptr(), qos) }
            };

            if packet_id == 0 {
                return Box::pin(future::ready(Err(AwsMqttError::ProtocolError.into())));
            }

            subscription.insert(topic.to_owned());
        }

        let (subscribe_tx, subscribe_rx) = oneshot::channel::<Message>();
        self.subscribers
            .push(Subscriber::new(&[topic], subscribe_tx));

        Box::pin(SubscribeMessageFuture::new(subscribe_rx))
    }

    /// Subscribes to multiple topics with a specified quality of service and an
    /// optional timeout.
    ///
    /// # Arguments:
    ///
    /// - `topics`: A slice of string references representing the topics to
    ///   subscribe to.
    /// - `qos`: The `qos` parameter stands for Quality of Service. It
    ///   determines the level of guarantee for
    /// message delivery between the client and the server.
    /// - `timeout`: The `timeout` parameter is an optional duration that
    ///   specifies the maximum amount of
    /// time to wait for the subscription to complete. If the subscription does
    /// not complete within the specified timeout duration, an error will be
    /// returned.
    ///
    /// # Returns:
    ///
    /// returns the [`Message`].
    pub fn subscribe_multiple(&self, topics: &[&str], qos: Qos) -> BoxFuture<Result<Message>> {
        if !self.is_connected() {
            return Box::pin(future::ready(Err(Error::NotConnected)));
        }

        let mut subscription = self.subscription.lock();

        let topics_diff = topics
            .iter()
            .map(ToString::to_string)
            .collect::<HashSet<_>>()
            .difference(&subscription)
            .map(ToOwned::to_owned)
            .collect_vec();

        if !topics_diff.is_empty() {
            let Ok::<Vec<CString>, NulError>(c_str_topics) = topics_diff
                .iter()
                .map(|topic| CString::new(topic.as_str()))
                .try_collect()
            else {
                return Box::pin(future::ready(Err(Error::InvalidTopic(
                    topics.iter().join(","),
                ))));
            };

            let mut topics_ptr = c_str_topics
                .iter()
                .map(|topic| topic.as_c_str().as_ptr())
                .collect_vec();

            let packet_id = {
                let guard = self.internal_client.lock();
                unsafe {
                    subscribe_multiple(
                        guard.internal_client,
                        topics_ptr.as_mut_ptr(),
                        topics.len(),
                        qos,
                    )
                }
            };

            if packet_id == 0 {
                return Box::pin(future::ready(Err(AwsMqttError::ProtocolError.into())));
            }

            subscription.extend(topics_diff);
        }

        let (subscribe_tx, subscribe_rx) = oneshot::channel::<Message>();
        self.subscribers.push(Subscriber::new(topics, subscribe_tx));

        Box::pin(SubscribeMessageFuture::new(subscribe_rx))
    }

    /// Unsubscribes from a specified topic.
    ///
    /// # Arguments:
    ///
    /// - `topic`: The `topic` parameter is a string that represents the topic
    ///   from which the client
    /// wants to unsubscribe.
    pub fn unsubscribe(&self, topic: &str) -> BoxFuture<Result<()>> {
        if !self.is_connected() {
            return Box::pin(future::ready(Err(Error::NotConnected)));
        }

        let Ok(c_topic) = CString::new(topic) else {
            return Box::pin(future::ready(Err(Error::InvalidTopic(topic.to_owned()))));
        };

        if self.subscription.lock().remove(topic) {
            let packet_id = {
                let guard = self.internal_client.lock();
                unsafe { unsubscribe(guard.internal_client, c_topic.as_ptr()) }
            };

            if packet_id == 0 {
                return Box::pin(future::ready(Err(AwsMqttError::ProtocolError.into())));
            }

            let (unsubscribe_tx, unsubscribe_rx) = oneshot::channel::<i32>();
            self.unsubscribe_notifiers
                .lock()
                .insert(packet_id, unsubscribe_tx);

            Box::pin(OperationResponseFuture::new(unsubscribe_rx))
        } else {
            Box::pin(future::ready(Ok(())))
        }
    }

    pub(crate) fn internal_client(&self) -> Arc<Mutex<InternalMqttClientPointer>> {
        self.internal_client.clone()
    }

    fn is_connected(&self) -> bool {
        matches!(*self.status.lock(), ClientStatus::Connected)
    }
}

impl Debug for MqttClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "MqttClient")
    }
}
