use beluga_mqtt::MqttClient;
use error::Error;

mod error;
mod proto;
mod services;
mod tunnel;

pub type Result<T> = core::result::Result<T, Error>;

pub use tunnel::Tunnel;

#[derive(Debug)]
pub struct TunnelManager {
    mqtt: MqttClient,
}

impl TunnelManager {
    pub fn new(mqtt: MqttClient) -> Self {
        Self { mqtt }
    }
}

#[derive(Debug)]
pub struct SecureTunnel {
    mqtt: MqttClient,
}
