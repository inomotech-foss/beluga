use std::sync::Arc;

use beluga_mqtt::{MqttClient, Publish, QoS};
use beluga_tunnel::Tunnel;
use error::Error;
use tokio::sync::Mutex;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::{debug, warn};

mod error;

type Result<T> = std::result::Result<T, Error>;

pub struct TunnelManager {
    tasks: Arc<Mutex<JoinSet<Result<()>>>>,
    handle: Option<JoinHandle<Result<()>>>,
    guard: Option<DropGuard>,
}

impl TunnelManager {
    /// Creates a new [`TunnelManager`] instance by subscribing to the specified
    /// MQTT topic for tunnel notifications.
    ///
    /// This function creates a new [`TunnelManager`] instance by subscribing to
    /// the MQTT topic `$aws/things/{thing_name}/tunnels/notify` with a quality
    /// of service level of at least once.
    ///
    /// # Arguments
    ///
    /// * `mqtt` - The `MqttClient` instance to use for the MQTT subscription.
    /// * `thing_name` - The name of the AWS IoT thing to subscribe to.
    pub async fn new(mqtt: MqttClient, thing_name: &str) -> Result<Self> {
        let mut subscriber = mqtt
            .subscribe(
                format!("$aws/things/{thing_name}/tunnels/notify").as_str(),
                QoS::AtLeastOnce,
            )
            .await?;

        let tasks = Arc::new(Mutex::new(JoinSet::new()));
        let cancel = CancellationToken::new();

        let cancel_child = cancel.child_token();

        let tasks_cp = tasks.clone();
        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_child.cancelled() => {
                        return Result::Ok(());
                    }
                    packet = subscriber.recv() => {
                        debug!("spawn new service");
                        tasks_cp.lock().await.spawn(service(packet?, cancel_child.clone()));
                    }
                }
            }
        });

        Ok(Self {
            tasks,
            handle: Some(handle),
            guard: Some(cancel.drop_guard()),
        })
    }

    /// Shuts down the tunnel manager and waits for all tasks to complete.
    ///
    /// This function first checks if the tunnel manager has already been shut
    /// down. If so, it logs a warning and returns.
    ///
    /// If the tunnel manager has not been shut down, this function does the
    /// following:
    ///
    /// 1. Retrieves the cancellation token and the handle to the tunnel
    ///    manager's own task.
    /// 2. Cancels the cancellation token, signaling that the tunnel manager
    ///    should be shut down.
    /// 3. Waits for all tasks managed by the tunnel manager to complete,
    ///    handling any errors that occur.
    /// 4. Waits for the tunnel manager's own task to complete, handling any
    ///    errors that occur.
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some((cancel, self_handle)) = self
            .guard
            .take()
            .map(DropGuard::disarm)
            .zip(self.handle.take())
        {
            cancel.cancel();
            let mut tasks = self.tasks.lock().await;

            // Waits for all tasks to complete and handles any errors that occur.
            while let Some(task_execution_res) = tasks.join_next().await {
                task_execution_res??;
            }

            self_handle.await??;
        } else {
            warn!("Tunnel manager already shut down all the tasks");
        }

        Ok(())
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
