use std::collections::{HashMap, HashSet};

use rumqttc::Publish;
use tokio::sync::{broadcast, mpsc};

use crate::{QoS, Receiver, Result, Sender};

const HALF_USIZE_MAX: usize = usize::MAX / 2;
const HALF_PLUS_ONE: usize = usize::MAX / 2 + 1;

#[derive(Debug)]
struct Subscriber {
    sender: Sender,
    qos: QoS,
}

impl Subscriber {
    /// Returns a reference to the `Sender` associated with this `Subscribed`
    /// instance.
    const fn sender(&self) -> &Sender {
        &self.sender
    }

    /// Returns the QoS (Quality of Service) level associated with this
    /// `Subscribed` instance.
    const fn qos(&self) -> QoS {
        self.qos
    }
}

#[derive(Debug)]
pub(super) struct SubscriberManager {
    subscribed: HashMap<String, Subscriber>,
    unsubscribed: HashSet<String>,
    close_tx: Option<mpsc::Sender<()>>,
    channel_capacity: usize,
}

impl SubscriberManager {
    /// Creates a new [`SubscriberManager`] instance with the provided
    /// `capacity`. The `capacity` parameter sets the capacity of the
    /// broadcast channels used to distribute messages to subscribers. If
    /// `capacity` is 0, it is set to 1. If `capacity` is between 0 and
    /// `usize::MAX / 2`, it is used as is. If `capacity` is between
    /// `usize::MAX / 2 + 1` and `usize::MAX`, it is set to `usize::MAX /
    /// 2`.
    #[allow(dead_code)]
    pub(super) fn new(capacity: usize) -> Self {
        let capacity = match capacity {
            0 => 1,
            capacity @ 0..=HALF_USIZE_MAX => capacity,
            HALF_PLUS_ONE..=usize::MAX => HALF_USIZE_MAX,
            _ => HALF_USIZE_MAX,
        };

        Self {
            subscribed: HashMap::default(),
            unsubscribed: HashSet::default(),
            channel_capacity: capacity,
            close_tx: None,
        }
    }

    /// Creates a new `SubscriberManager` instance with the provided `close_tx`
    /// and `capacity`. The `close_tx` parameter is a channel sender that
    /// can be used to signal the manager to close. The `capacity` parameter
    /// sets the capacity of the broadcast channels used to distribute
    /// messages to subscribers. If `capacity` is 0, it is set to 1. If
    /// `capacity` is between 0 and `usize::MAX / 2`, it is used as is. If
    /// `capacity` is between `usize::MAX / 2 + 1` and `usize::MAX`, it is
    /// set to `usize::MAX / 2`.
    pub(super) fn with_close_tx(close_tx: mpsc::Sender<()>, capacity: usize) -> Self {
        let capacity = match capacity {
            0 => 1,
            capacity @ 0..=HALF_USIZE_MAX => capacity,
            HALF_PLUS_ONE..=usize::MAX => HALF_USIZE_MAX,
            _ => HALF_USIZE_MAX,
        };

        Self {
            close_tx: Some(close_tx),
            subscribed: HashMap::default(),
            unsubscribed: HashSet::default(),
            channel_capacity: capacity,
        }
    }

    /// Returns a reference to the `Sender` for the given topic, if the
    /// topic is currently subscribed.
    pub(super) fn sender(&self, topic: &str) -> Option<&Sender> {
        self.subscribed.get(topic).map(Subscriber::sender)
    }

    /// Returns a receiver for the given topic, if the topic is currently
    /// subscribed
    pub(super) fn receiver(&self, topic: &str) -> Option<Receiver> {
        self.subscribed
            .get(topic)
            .map(|subscriber| subscriber.sender().subscribe())
    }

    /// Returns the Quality of Service (QoS) level for the given topic, if the
    /// topic is currently subscribed.
    #[allow(dead_code)]
    pub(super) fn qos(&self, topic: &str) -> Option<QoS> {
        self.subscribed.get(topic).map(Subscriber::qos)
    }

    /// Returns an iterator over the topics that are not currently
    /// subscribed.
    pub(super) fn subscribed_diff<Iter>(&self, topics: Iter) -> Vec<String>
    where
        Iter: IntoIterator,
        Iter::Item: AsRef<str>,
    {
        topics
            .into_iter()
            .filter(|topic| !self.subscribed.contains_key(topic.as_ref()))
            .map(|topic| topic.as_ref().to_owned())
            .collect::<Vec<String>>()
    }

    pub(super) fn subscribe(&mut self, topic: &str, qos: QoS) -> Receiver {
        self.unsubscribed.remove(topic);

        if let Some(subscriber) = self.subscribed.get(topic) {
            subscriber.sender().subscribe()
        } else {
            let (sender, rx) = broadcast::channel::<Result<Publish>>(self.channel_capacity);
            self.subscribed
                .insert(topic.to_owned(), Subscriber { sender, qos });
            rx
        }
    }

    pub(super) fn subscribe_many<Iter>(&mut self, topics: Iter, qos: QoS) -> Vec<Receiver>
    where
        Iter: IntoIterator,
        Iter::Item: AsRef<str>,
    {
        topics
            .into_iter()
            .map(|topic| self.subscribe(topic.as_ref(), qos))
            .collect::<Vec<_>>()
    }

    /// Removes the given topic from the list of subscribed topics and the
    /// list of unsubscribed topics.
    pub(super) fn unsubscribe(&mut self, topic: &str) {
        self.subscribed.remove(topic);
        self.unsubscribed.remove(topic);
    }

    /// Schedules the unsubscription of the given topic.
    pub(super) fn schedule_unsubscribe(&mut self, topic: &str) {
        self.subscribed.remove(topic);
        self.unsubscribed.insert(topic.to_owned());
    }

    /// Schedules the unsubscription of the given topics.
    pub(super) fn schedule_unsubscribe_many<Iter>(&mut self, topics: Iter)
    where
        Iter: IntoIterator,
        Iter::Item: AsRef<str>,
    {
        for topic in topics {
            self.subscribed.remove(topic.as_ref());
            self.unsubscribed.insert(topic.as_ref().to_owned());
        }
    }

    /// Returns a reference to the set of topics that have been scheduled for
    /// unsubscription.
    pub(super) fn scheduled(&self) -> &HashSet<String> {
        &self.unsubscribed
    }

    /// Returns an iterator over the senders that have subscribed to this
    /// manager.
    pub(super) fn subscribers(&self) -> impl Iterator<Item = &Sender> {
        self.subscribed.values().map(Subscriber::sender)
    }

    /// Returns an iterator over the topics that have been subscribed to, along
    /// with their associated QoS level.
    pub(super) fn topics_with_qos(&self) -> impl Iterator<Item = (&str, QoS)> {
        self.subscribed
            .iter()
            .map(|(topic, subscriber)| (topic.as_str(), subscriber.qos()))
    }

    #[allow(dead_code)]
    /// Returns the capacity of the channel used by this subscriber manager.
    pub(super) const fn capacity(&self) -> usize {
        self.channel_capacity
    }
}

impl Drop for SubscriberManager {
    fn drop(&mut self) {
        // Attempts to send a close signal to the subscriber manager's close
        // channel.
        if let Some(close_tx) = self.close_tx.take() {
            // there is an possibility that polling didn't start yet.
            let _ = close_tx.try_send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state() {
        let manager = SubscriberManager::new(1);
        assert!(manager.subscribed.is_empty());
        assert!(manager.unsubscribed.is_empty());
    }

    #[test]
    fn subscribe() {
        let mut manager = SubscriberManager::new(1);
        let topic = "topic";

        let _receiver = manager.subscribe(topic, QoS::AtLeastOnce);

        assert!(manager.subscribed.contains_key(topic));
        assert!(!manager.unsubscribed.contains(topic));
        assert!(manager.receiver(topic).is_some());
    }

    #[test]
    fn subscribe_existing() {
        let mut manager = SubscriberManager::new(1);
        let topic = "topic";

        let _ = manager.subscribe(topic, QoS::AtLeastOnce);
        let _ = manager.subscribe(topic, QoS::AtLeastOnce);

        assert_eq!(manager.subscribed.len(), 1);
        assert!(manager.subscribed.contains_key(topic));
    }

    #[test]
    fn unsubscribe() {
        let mut manager = SubscriberManager::new(1);
        let topic = "topic";

        manager.subscribe(topic, QoS::AtLeastOnce);
        manager.unsubscribe(topic);

        assert!(!manager.subscribed.contains_key(topic));
        assert!(!manager.unsubscribed.contains(topic));
    }

    #[test]
    fn schedule_unsubscribe() {
        let mut manager = SubscriberManager::new(1);
        let topic = "topic";

        manager.subscribe(topic, QoS::AtLeastOnce);
        manager.schedule_unsubscribe(topic);

        assert!(!manager.subscribed.contains_key(topic));
        assert!(manager.unsubscribed.contains(topic));
    }

    #[test]
    fn subscribed_diff() {
        let mut manager = SubscriberManager::new(1);
        manager.subscribe_many(["topic1", "topic2", "topic3"], QoS::AtLeastOnce);
        let diff = manager.subscribed_diff(["topic4", "topic5", "topic2"]);
        assert_eq!(diff, ["topic4", "topic5"]);
    }

    #[test]
    fn subscribed_diff_empty() {
        let mut manager = SubscriberManager::new(1);
        manager.subscribe_many(["topic1", "topic2", "topic3"], QoS::AtLeastOnce);
        let diff = manager.subscribed_diff(["topic2", "topic3"]);
        assert!(diff.is_empty());
    }

    #[test]
    fn receiver() {
        let mut manager = SubscriberManager::new(1);
        let topic = "topic";
        manager.subscribe(topic, QoS::AtMostOnce);
        let receiver = manager.receiver(topic);
        assert!(receiver.is_some());
    }

    #[test]
    fn schedule_unsubscribe_many() {
        let mut manager = SubscriberManager::new(1);

        manager.subscribe("topic1", QoS::AtMostOnce);
        manager.subscribe("topic2", QoS::AtMostOnce);
        manager.schedule_unsubscribe_many(["topic1", "topic2"]);

        assert!(manager.unsubscribed.contains("topic1"));
        assert!(manager.unsubscribed.contains("topic2"));
    }

    #[test]
    fn scheduled() {
        let mut manager = SubscriberManager::new(1);
        manager.schedule_unsubscribe("topic1");
        manager.schedule_unsubscribe("topic2");

        let scheduled = manager.scheduled();
        assert!(scheduled.contains("topic1"));
        assert!(scheduled.contains("topic2"));
    }

    #[test]
    fn subscribers() {
        let mut manager = SubscriberManager::new(1);
        manager.subscribe("topic1", QoS::AtMostOnce);
        manager.subscribe("topic2", QoS::AtMostOnce);

        let subscribers: Vec<_> = manager.subscribers().collect();
        assert_eq!(subscribers.len(), 2);
    }

    #[test]
    fn subscribe_empty_topic() {
        let mut manager = SubscriberManager::new(1);
        let empty_topic = "";

        let _ = manager.subscribe(empty_topic, QoS::ExactlyOnce);

        assert!(manager.subscribed.contains_key(empty_topic));
    }

    #[test]
    fn unsubscribe_non_existing() {
        let mut manager = SubscriberManager::new(1);
        let non_existing_topic = "non_existing_topic";

        manager.unsubscribe(non_existing_topic);

        assert!(!manager.subscribed.contains_key(non_existing_topic));
        assert!(!manager.unsubscribed.contains(non_existing_topic));
    }

    #[test]
    fn subscribe_many() {
        let mut manager = SubscriberManager::new(1);
        let topics = vec!["topic1", "topic2", "topic3"];

        let receivers = manager.subscribe_many(topics.clone(), QoS::ExactlyOnce);

        assert_eq!(receivers.len(), 3);
        for topic in topics {
            assert!(manager.subscribed.contains_key(topic));
        }
    }

    #[test]
    fn schedule_unsubscribe_already_unsubscribed() {
        let mut manager = SubscriberManager::new(1);
        let topic = "already_unsubscribed";

        manager.schedule_unsubscribe(topic);
        manager.schedule_unsubscribe(topic); // Second time

        assert!(manager.unsubscribed.contains(topic));
        assert_eq!(manager.unsubscribed.len(), 1);
    }

    #[test]
    fn combination_of_operations() {
        let mut manager = SubscriberManager::new(1);

        // Subscribe to multiple topics
        let topics = vec!["topic1", "topic2", "topic3"];
        manager.subscribe_many(topics.clone(), QoS::ExactlyOnce);

        // Unsubscribe one topic
        manager.unsubscribe("topic2");

        // Schedule unsubscription for another topic
        manager.schedule_unsubscribe("topic3");

        // Check that the states are correct
        assert!(manager.subscribed.contains_key("topic1"));
        assert!(!manager.subscribed.contains_key("topic2"));
        assert!(manager.unsubscribed.contains("topic3"));

        let scheduled: Vec<_> = manager.scheduled().iter().collect();
        assert_eq!(scheduled, vec!["topic3"]);
    }

    #[test]
    fn capacity() {
        let manager = SubscriberManager::new(usize::MAX);
        assert_eq!(manager.capacity(), HALF_USIZE_MAX);

        let manager = SubscriberManager::new(0);
        assert_eq!(manager.capacity(), 1);

        let manager = SubscriberManager::new(5);
        assert_eq!(manager.capacity(), 5);

        let manager = SubscriberManager::new(HALF_PLUS_ONE);
        assert_eq!(manager.capacity(), HALF_USIZE_MAX);

        let manager = SubscriberManager::new(HALF_USIZE_MAX);
        assert_eq!(manager.capacity(), HALF_USIZE_MAX);
    }

    #[tokio::test]
    async fn with_close_tx_closes_manager() {
        let (close_tx, mut close_rx) = mpsc::channel(1);
        let manager = SubscriberManager::with_close_tx(close_tx, 1);

        // Drop the manager
        drop(manager);

        // Ensure close signal is received
        assert!(close_rx.recv().await.is_some());
    }
}
