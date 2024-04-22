use beluga_mqtt::{MqttClient, Publish, QoS};
use beluga_tunnel::Tunnel;
use error::Error;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::debug;

mod error;

type Result<T> = std::result::Result<T, Error>;

pub struct TunnelManager {
    _handler: JoinHandle<Result<()>>,
    _guard: DropGuard,
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

        let cancel = CancellationToken::new();
        let cancel_child = cancel.child_token();

        let _handler = tokio::spawn(async move {
            let mut set = JoinSet::new();
            loop {
                tokio::select! {
                    _ = cancel_child.cancelled() => {
                        debug!("let's shutdown all the services");
                        set.shutdown().await;
                        return Ok(());
                    }
                    packet = subscriber.recv() => {
                        debug!("spawn new service");
                        set.spawn(service(packet?, cancel_child.clone()));
                    }
                }
            }
        });

        Ok(Self {
            _handler,
            _guard: cancel.drop_guard(),
        })
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
            debug!("ssh service cancelled");
            Ok(())
        }
        res = tunnel.start(service) => {
            res?;
            Ok(())
        }
    }
}
