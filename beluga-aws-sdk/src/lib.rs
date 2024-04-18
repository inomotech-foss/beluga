use beluga_mqtt::{MqttClient, QoS, Subscriber};
use beluga_tunnel::Tunnel;
use error::Error;
use tokio::task::JoinHandle;

mod error;

type Result<T> = std::result::Result<T, Error>;

pub struct TunnelManager {
    subscriber: Subscriber,
}

impl TunnelManager {
    /// Creates a new [`TunnelManager`] instance by subscribing to the specified
    /// MQTT topic for tunnel notifications.
    ///
    /// This function creates a new [`TunnelManager`] instance by subscribing to
    /// the MQTT topic `$aws/things/{thing_name}/tunnels/notify` with a quality
    /// of service level of at least once. The resulting
    /// [`MqttSubscriber`](Subscriber) is stored in the [`TunnelManager`]
    /// instance.
    ///
    /// # Arguments
    ///
    /// * `mqtt` - The `MqttClient` instance to use for the MQTT subscription.
    /// * `thing_name` - The name of the AWS IoT thing to subscribe to.
    pub async fn new(mqtt: MqttClient, thing_name: &str) -> Result<Self> {
        let subscriber = mqtt
            .subscribe(
                format!("$aws/things/{thing_name}/tunnels/notify").as_str(),
                QoS::AtLeastOnce,
            )
            .await?;

        Ok(Self { subscriber })
    }

    /// Receives a packet from the subscriber and spawns a new task to start a
    /// tunnel.
    ///
    /// This function is responsible for handling the incoming packet from the
    /// subscriber and creating a new tunnel based on the payload of the
    /// packet. It then starts the tunnel using the provided `Service`.
    /// The function returns a `JoinHandle` for the spawned task, which can
    /// be used to await the completion of the tunnel operation.
    ///
    /// # Errors
    ///
    /// This function may return an error if there is a problem receiving the
    /// packet from the subscriber or starting the tunnel.
    pub async fn recv(&mut self) -> Result<JoinHandle<Result<()>>> {
        let packet = self.subscriber.recv().await?;
        Ok(tokio::spawn(async move {
            let service = beluga_ssh_service::SshService;
            let tunnel = Tunnel::new(packet.payload).await?;
            tunnel.start(service).await?;
            Ok(())
        }))
    }
}
