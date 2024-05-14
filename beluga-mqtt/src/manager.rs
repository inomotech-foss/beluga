use std::collections::{HashMap, HashSet};

use rumqttc::Publish;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::sync::mpsc;
use tracing::warn;

#[derive(Debug)]
pub(super) struct SubscriberManager {
    subscribed: HashMap<String, Sender<Publish>>,
    unsubscribed: HashSet<String>,
    close_tx: Option<mpsc::Sender<()>>,
}

impl SubscriberManager {
    /// Creates a new `Manager` instance with no `close_tx` channel sender.
    /// The `close_tx` channel is used to signal when the manager should be
    /// closed. By default, the `close_tx` channel is set to `None`.
    #[allow(dead_code)]
    pub(super) fn new() -> Self {
        Self {
            subscribed: HashMap::default(),
            unsubscribed: HashSet::default(),
            close_tx: None,
        }
    }

    /// Creates a new `Manager` instance with the provided `close_tx` channel
    /// sender. The `close_tx` channel is used to signal when the manager
    /// should be closed.
    pub(super) fn with_close_tx(close_tx: mpsc::Sender<()>) -> Self {
        Self {
            close_tx: Some(close_tx),
            subscribed: HashMap::default(),
            unsubscribed: HashSet::default(),
        }
    }

    /// Returns a reference to the `Sender` for the given topic, if the
    /// topic is currently subscribed.
    pub(super) fn sender(&self, topic: &str) -> Option<&Sender<Publish>> {
        self.subscribed.get(topic)
    }

    /// Returns a receiver for the given topic, if the topic is currently
    /// subscribed
    pub(super) fn receiver(&self, topic: &str) -> Option<Receiver<Publish>> {
        self.subscribed.get(topic).map(|sender| sender.subscribe())
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

    pub(super) fn subscribe(&mut self, topic: &str) -> Receiver<Publish> {
        self.unsubscribed.remove(topic);

        if let Some(sender) = self.subscribed.get(topic) {
            sender.subscribe()
        } else {
            let (tx, rx) = broadcast::channel::<Publish>(10);
            self.subscribed.insert(topic.to_owned(), tx);
            rx
        }
    }

    pub(super) fn subscribe_many<Iter>(&mut self, topics: Iter) -> Vec<Receiver<Publish>>
    where
        Iter: IntoIterator,
        Iter::Item: AsRef<str>,
    {
        topics
            .into_iter()
            .map(|topic| self.subscribe(topic.as_ref()))
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

    /// Returns an iterator over the list of unsubscribed topics.
    pub(super) fn scheduled(&self) -> impl Iterator<Item = &str> {
        self.unsubscribed.iter().map(AsRef::as_ref)
    }
}

impl Drop for SubscriberManager {
    fn drop(&mut self) {
        // Attempts to send a close signal to the subscriber manager's close
        // channel. If the send operation fails, a warning is logged
        // with the error.
        if let Some(close_tx) = self.close_tx.take() {
            if let Err(err) = close_tx.try_send(()) {
                warn!(error = %err, "failed to close subscriber manager");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state() {
        let manager = SubscriberManager::new();
        assert!(manager.subscribed.is_empty());
        assert!(manager.unsubscribed.is_empty());
    }

    #[test]
    fn subscribe() {
        let mut manager = SubscriberManager::new();
        let topic = "topic";

        let _receiver = manager.subscribe(topic);

        assert!(manager.subscribed.contains_key(topic));
        assert!(!manager.unsubscribed.contains(topic));
        assert!(manager.receiver(topic).is_some());
    }

    #[test]
    fn subscribe_existing() {
        let mut manager = SubscriberManager::new();
        let topic = "topic";

        let _ = manager.subscribe(topic);
        let _ = manager.subscribe(topic);

        assert_eq!(manager.subscribed.len(), 1);
        assert!(manager.subscribed.contains_key(topic));
    }

    #[test]
    fn unsubscribe() {
        let mut manager = SubscriberManager::new();
        let topic = "topic";

        manager.subscribe(topic);
        manager.unsubscribe(topic);

        assert!(!manager.subscribed.contains_key(topic));
        assert!(!manager.unsubscribed.contains(topic));
    }

    #[test]
    fn schedule_unsubscribe() {
        let mut manager = SubscriberManager::new();
        let topic = "topic";

        manager.subscribe(topic);
        manager.schedule_unsubscribe(topic);

        assert!(!manager.subscribed.contains_key(topic));
        assert!(manager.unsubscribed.contains(topic));
    }

    #[test]
    fn subscribed_diff() {
        let mut manager = SubscriberManager::new();
        manager.subscribe_many(["topic1", "topic2", "topic3"]);
        let diff = manager.subscribed_diff(["topic4", "topic5", "topic2"]);
        assert_eq!(diff, ["topic4", "topic5"]);
    }

    #[test]
    fn subscribed_diff_empty() {
        let mut manager = SubscriberManager::new();
        manager.subscribe_many(["topic1", "topic2", "topic3"]);
        let diff = manager.subscribed_diff(["topic2", "topic3"]);
        assert!(diff.is_empty());
    }

    #[test]
    fn receiver() {
        let mut manager = SubscriberManager::new();
        let topic = "topic";
        manager.subscribe(topic);
        let receiver = manager.receiver(topic);
        assert!(receiver.is_some());
    }

    #[test]
    fn schedule_unsubscribe_many() {
        let mut manager = SubscriberManager::new();

        manager.subscribe("topic1");
        manager.subscribe("topic2");
        manager.schedule_unsubscribe_many(["topic1", "topic2"]);

        assert!(manager.unsubscribed.contains("topic1"));
        assert!(manager.unsubscribed.contains("topic2"));
    }

    #[test]
    fn scheduled() {
        let mut manager = SubscriberManager::new();
        manager.schedule_unsubscribe("topic1");
        manager.schedule_unsubscribe("topic2");

        let scheduled: Vec<_> = manager.scheduled().collect();
        assert!(scheduled.contains(&"topic1"));
        assert!(scheduled.contains(&"topic2"));
    }

    #[test]
    fn subscribe_empty_topic() {
        let mut manager = SubscriberManager::new();
        let empty_topic = "";

        let _ = manager.subscribe(empty_topic);

        assert!(manager.subscribed.contains_key(empty_topic));
    }

    #[test]
    fn unsubscribe_non_existing() {
        let mut manager = SubscriberManager::new();
        let non_existing_topic = "non_existing_topic";

        manager.unsubscribe(non_existing_topic);

        assert!(!manager.subscribed.contains_key(non_existing_topic));
        assert!(!manager.unsubscribed.contains(non_existing_topic));
    }

    #[test]
    fn subscribe_many() {
        let mut manager = SubscriberManager::new();
        let topics = vec!["topic1", "topic2", "topic3"];

        let receivers = manager.subscribe_many(topics.clone());

        assert_eq!(receivers.len(), 3);
        for topic in topics {
            assert!(manager.subscribed.contains_key(topic));
        }
    }

    #[test]
    fn schedule_unsubscribe_already_unsubscribed() {
        let mut manager = SubscriberManager::new();
        let topic = "already_unsubscribed";

        manager.schedule_unsubscribe(topic);
        manager.schedule_unsubscribe(topic); // Second time

        assert!(manager.unsubscribed.contains(topic));
        assert_eq!(manager.unsubscribed.len(), 1);
    }

    #[test]
    fn combination_of_operations() {
        let mut manager = SubscriberManager::new();

        // Subscribe to multiple topics
        let topics = vec!["topic1", "topic2", "topic3"];
        manager.subscribe_many(topics.clone());

        // Unsubscribe one topic
        manager.unsubscribe("topic2");

        // Schedule unsubscription for another topic
        manager.schedule_unsubscribe("topic3");

        // Check that the states are correct
        assert!(manager.subscribed.contains_key("topic1"));
        assert!(!manager.subscribed.contains_key("topic2"));
        assert!(manager.unsubscribed.contains("topic3"));

        let scheduled: Vec<_> = manager.scheduled().collect();
        assert_eq!(scheduled, vec!["topic3"]);
    }
}
