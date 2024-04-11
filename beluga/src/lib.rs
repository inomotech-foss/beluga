mod error;
mod mqtt;

pub type Result<T> = core::result::Result<T, Error>;

pub use error::Error;
pub use mqtt::{MqttClient, MqttClientBuilder, Subscriber};
pub use rumqttc::QoS;
