use beluga_mqtt::{MqttClient, Publish, QoS};
use beluga_tunnel::Tunnel;
use tokio::sync::broadcast::error::RecvError;
use tokio_util::sync::{CancellationToken, DropGuard};
use tokio_util::task::TaskTracker;
use tracing::{debug, error, warn};

use self::error::Error;
pub use self::jobs::{Job, JobStatus, JobsClient};

mod error;
mod jobs;
pub mod provision;

type Result<T> = std::result::Result<T, Error>;

/// Creates a details `HashMap` from a list of key-value pairs.
///
/// This macro takes a list of key-value pairs and returns a `HashMap` with the
/// provided keys and values. The keys and values are converted to `String`
/// instances before being added to the `HashMap`.
///
/// # Example
///
/// let details = details! {
///     "serial" => "01234567",
/// };
#[macro_export]
macro_rules! details {
    {$($k: expr => $v: expr),* $(,)?} => {
        ::std::collections::HashMap::<String, String>::from([$(($k.to_owned(), $v.to_owned()),)*])
    };
}

pub struct TunnelManager {
    task_tracker: TaskTracker,
    cancel_guard: Option<DropGuard>,
}

impl TunnelManager {
    /// Runs a new tunnel service in response to a tunnel notification.
    ///
    /// This function is spawned as a new task when a tunnel notification is
    /// received. It creates a new `Service` instance and starts the tunnel
    /// using the payload from the received MQTT packet.
    ///
    /// # Arguments
    ///
    /// * `mqtt` - The [`MqttClient`] instance to use for the MQTT subscription.
    /// * `thing_name` - The name of the AWS IoT thing to subscribe to.
    pub async fn new(mqtt: MqttClient, thing_name: &str) -> Result<Self> {
        let mut subscriber = mqtt
            .subscribe(
                format!("$aws/things/{thing_name}/tunnels/notify").as_str(),
                QoS::AtLeastOnce,
            )
            .await?;

        let task_tracker = TaskTracker::new();
        let tracker = task_tracker.clone();

        // let's make sure that we cancel all the tasks on TokenManager drop
        let cancel = CancellationToken::new();
        let cancel_task = cancel.clone();
        task_tracker.spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_task.cancelled() => {
                        return Result::Ok(());
                    }
                    res = subscriber.recv() => {
                        let packet = match res {
                            Ok(packet) => packet,
                            // This is (probably) the only case we can't recover from.
                            // Ideally subscriptions should yield `Option<Publish>` instead of a `Result<_>`, but this is the API for now.
                            Err(beluga_mqtt::Error::Receive(RecvError::Closed)) => {
                                warn!("subscriber closed");
                                return Result::Ok(());
                            }
                            // The error can be from a disconnect, so we ignore it.
                            Err(err) => {
                                error!(err = &err as &dyn std::error::Error, "failed to receive packet");
                                continue;
                            }
                        };
                        debug!("spawn new service");
                        tracker.spawn(service(packet, cancel_task.clone()));
                    }
                }
            }
        });

        Ok(Self {
            task_tracker,
            cancel_guard: Some(cancel.drop_guard()),
        })
    }

    /// Graceful shutdown of the tunnel manager.
    pub async fn shutdown(&mut self) {
        if let Some(cancel) = self.cancel_guard.take() {
            cancel.disarm().cancel();
        } else {
            warn!("manager is already shutdown");
        }
        self.task_tracker.close();
        self.task_tracker.wait().await;
    }
}

/// Runs the SSH service for a new tunnel.
///
/// This function is spawned as a new task when a tunnel notification is
/// received. It creates a new Service instance and starts the tunnel using the
/// payload from the received MQTT packet.
///
/// The function will run until the `cancel` token is triggered, indicating that
/// the tunnel should be shut down.
///
/// # Arguments
///
/// * `packet` - The MQTT [`Publish`] packet containing the tunnel payload.
/// * `cancel` - A [`CancellationToken`] that can be used to cancel the service.
///
/// # Returns
///
/// A [`Result`] indicating whether the service ran successfully.
async fn service(packet: Publish, cancel: CancellationToken) -> Result<()> {
    let service = beluga_ssh_service::SshService;
    let tunnel = Tunnel::new(packet.payload).await?;
    tokio::select! {
        _ = cancel.cancelled() => {
            warn!("ssh service cancelled");
            Ok(())
        }
        res = tunnel.start(service) => {
            res?;
            Ok(())
        }
    }
}
