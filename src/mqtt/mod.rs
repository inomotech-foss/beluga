use std::ffi::CString;
use std::time::Duration;

use client::ClientConfig;
pub(crate) use client::InternalMqttClient;
pub use client::{ClientStatus, MqttClient};
pub use futures::{CreateMqttFuture, OperationResponseFuture, SubscribeMessageFuture};
use itertools::Itertools;

use crate::{Error, Qos, Result};

mod callbacks;
mod client;
mod futures;

/// The struct represents a message in a MQTT broker
#[derive(Debug, Clone)]
pub struct Message {
    /// Property represents the topic of the message.
    /// Topic is a string that identifies the subject or category of the
    /// message. It is used to route the message to the appropriate
    /// subscribers.
    pub topic: String,
    /// Represents binary data of the message.
    pub data: Vec<u8>,
    /// Stands for "duplicate" and it is a boolean value that indicates whether
    /// the message is a duplicate or not.
    pub dup: bool,
    /// Stands for Quality of Service. It is a measure of the reliability and
    /// guarantee of message delivery in a messaging system. In the context
    /// of the [`Message`] struct, the `qos` property represents the Quality
    /// of Service level for the message.
    pub qos: Qos,
    /// Property indicates whether the message should be retained by the broker.
    /// When a message is published with the `retain` flag set to true, the
    /// broker will store the message and deliver it to any new subscribers
    /// that join the topic.
    pub retain: bool,
}

/// The `Config` represents the configuration settings for a MQTT client.
pub struct Config {
    endpoint: CString,
    port: u16,
    client_id: CString,
    clean_session: bool,
    keep_alive_s: u16,
    ping_timeout_ms: u32,
    username: CString,
    password: CString,
    cert: Vec<u8>,
    private_key: Vec<u8>,
}

impl From<&Config> for ClientConfig {
    fn from(config: &Config) -> Self {
        Self {
            endpoint: config.endpoint.as_ptr(),
            client_id: config.client_id.as_ptr(),
            clean_session: config.clean_session,
            keep_alive_s: config.keep_alive_s,
            ping_timeout_ms: config.ping_timeout_ms,
            certificate: config.cert.as_slice().into(),
            private_key: config.private_key.as_slice().into(),
            port: config.port,
            username: config.username.as_ptr(),
            password: config.password.as_ptr(),
        }
    }
}

/// The `ConfigBuilder` struct is used to build a configuration object with
/// various optional fields.
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    endpoint: Option<CString>,
    client_id: Option<CString>,
    clean_session: bool,
    keep_alive_s: Option<u16>,
    ping_timeout_ms: Option<u32>,
    cert: Option<Vec<u8>>,
    private_key: Option<Vec<u8>>,
    port: Option<u16>,
    username: Option<CString>,
    password: Option<CString>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the endpoint for a MQTT client
    ///
    /// # Arguments:
    ///
    /// - `endpoint`: A string representing the endpoint URL.
    pub fn with_endpoint(mut self, endpoint: &str) -> Result<Self> {
        self.endpoint = Some(CString::new(endpoint).map_err(Error::StringConversion)?);
        Ok(self)
    }

    /// Sets the client ID for a MQTT client
    ///
    /// # Arguments:
    ///
    /// - `client_id`: A string representing the client ID.
    pub fn with_client_id(mut self, client_id: &str) -> Result<Self> {
        self.client_id = Some(CString::new(client_id).map_err(Error::StringConversion)?);
        Ok(self)
    }

    /// The `with_clean_session` function sets the `clean_session` flag to true
    pub fn with_clean_session(mut self) -> Self {
        self.clean_session = true;
        self
    }

    /// Sets the keep-alive duration for a MQTT client
    ///
    /// # Arguments:
    ///
    /// - `keep_alive`: It represents the duration for which a connection should
    ///   be kept alive.
    pub fn with_keep_alive(mut self, keep_alive: Duration) -> Self {
        self.keep_alive_s = Some(keep_alive.as_secs() as u16);
        self
    }

    /// Sets the ping timeout value for a MQTT client.
    ///
    /// # Arguments:
    ///
    /// - `timeout`: It specifies the duration of the ping timeout in
    ///   milliseconds.
    pub fn with_ping_timeout(mut self, timeout: Duration) -> Self {
        self.ping_timeout_ms = Some(timeout.as_millis() as u32);
        self
    }

    /// Sets the certificate for a MQTT client
    ///
    /// # Arguments:
    ///
    /// - `cert`: client's certificate
    pub fn with_cert(mut self, cert: impl IntoIterator<Item = u8>) -> Self {
        self.cert = Some(cert.into_iter().collect_vec());
        self
    }

    /// Sets the private key for a MQTT client
    ///
    /// # Arguments:
    ///
    /// * `private_key`: client's private key
    pub fn with_private_key(mut self, private_key: impl IntoIterator<Item = u8>) -> Self {
        self.private_key = Some(private_key.into_iter().collect_vec());
        self
    }

    /// The `with_port` function sets the port value for MQTT broker
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// The function sets the username for MQTT broker
    pub fn with_username(mut self, username: &str) -> Result<Self> {
        self.username = Some(CString::new(username).map_err(Error::StringConversion)?);
        Ok(self)
    }

    /// The function sets the password for MQTT broker
    pub fn with_password(mut self, password: &str) -> Result<Self> {
        self.password = Some(CString::new(password).map_err(Error::StringConversion)?);
        Ok(self)
    }

    pub fn build(self) -> Result<Config> {
        Ok(Config {
            endpoint: self.endpoint.ok_or(Error::MissEndpoint)?,
            client_id: self.client_id.ok_or(Error::MissClientId)?,
            clean_session: self.clean_session,
            keep_alive_s: self.keep_alive_s.unwrap_or(1_000),
            ping_timeout_ms: self.ping_timeout_ms.unwrap_or(500),
            cert: self.cert.unwrap_or_default(),
            private_key: self.private_key.unwrap_or_default(),
            port: self.port.unwrap_or_default(),
            username: self.username.unwrap_or_default(),
            password: self.password.unwrap_or_default(),
        })
    }
}
