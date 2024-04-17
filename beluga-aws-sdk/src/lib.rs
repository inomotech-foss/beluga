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
    pub async fn new(mqtt: MqttClient, thing_name: &str) -> Result<Self> {
        let subscriber = mqtt
            .subscribe(
                format!("$aws/things/{thing_name}/tunnels/notify").as_str(),
                QoS::AtLeastOnce,
            )
            .await?;

        Ok(Self { subscriber })
    }

    pub async fn recv(&mut self) -> Result<JoinHandle<Result<()>>> {
        let packet = self.subscriber.recv().await?;
        Ok(tokio::spawn(async move {
            let service = beluga_ssh_service::SshService::default();
            let tunnel = Tunnel::new(packet.payload).await?;
            tunnel.start(service).await?;
            Ok(())
        }))
    }
}
