use rumqttc::Publish;
use tokio::sync::broadcast;

use crate::{Error, MqttClient, Receiver, Result};

/// Represents an MQTT subscriber that can receive `Publish` messages.
#[derive(Debug)]
pub struct Subscriber(pub(super) Vec<Receiver>);

impl Subscriber {
    /// Asynchronously receives a [`Publish`] message from one of
    /// the underlying receivers.
    pub async fn recv(&mut self) -> Result<Publish> {
        // It's impossible, but let's catch it instead of further panic.
        if self.0.is_empty() {
            return Err(Error::EmptySubscriber);
        }

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

/// Represents an MQTT subscriber that owns the underlying subscriber and
/// tracks the subscribed topics. When the [`OwnedSubscriber`] is dropped,
/// it will unsubscribe from all the subscribed topics.
#[derive(Debug)]
pub struct OwnedSubscriber {
    pub(super) subscriber: Subscriber,
    pub(super) topics: Vec<String>,
    pub(super) mqtt: MqttClient,
}

impl OwnedSubscriber {
    /// Asynchronously receives a [`Publish`] message from one of
    /// the underlying receivers.
    pub async fn recv(&mut self) -> Result<Publish> {
        self.subscriber.recv().await
    }
}

impl Drop for OwnedSubscriber {
    fn drop(&mut self) {
        self.mqtt.schedule_unsubscribe_many(&self.topics);
    }
}
