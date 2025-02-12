use std::collections::HashMap;
use std::time::Duration;

use beluga_iot_core::provision::{
    create_keys_and_certificate, register_thing, RegisterThingResponse,
};
use beluga_mqtt::{MqttClientBuilder, QoS};
use tracing::{info, Level};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let client = MqttClientBuilder::new()
        .ca(&tokio::fs::read("AmazonRootCA1.pem").await?)
        .certificate(&tokio::fs::read("certificate.pem.crt").await?)
        .private_key(&tokio::fs::read("private.pem.key").await?)
        .endpoint(&tokio::fs::read_to_string("endpoint.in").await?)
        .thing_name("it doesn't matter")
        .build()?;

    let mut params = HashMap::new();
    // Adds a "SerialNumber" parameter to the `params` HashMap with the value
    // "serial-number-1234". This parameter can be used when registering a new
    // thing with the AWS IoT Core service.
    params.insert("SerialNumber".to_owned(), "serial-number-1234".to_owned());

    let info = create_keys_and_certificate(client.clone()).await?;
    let RegisterThingResponse { thing_name, .. } =
        register_thing(client.clone(), &info, "template", Some(params)).await?;

    let client = MqttClientBuilder::new()
        .ca(&tokio::fs::read("AmazonRootCA1.pem").await?)
        .certificate(info.certificate.as_bytes())
        .private_key(info.key.as_bytes())
        .endpoint(&tokio::fs::read_to_string("endpoint.in").await?)
        .thing_name(&thing_name)
        .build()?;

    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            _  = interval.tick() => {
                client.publish(
                    "some-topic", QoS::AtLeastOnce, false,
                    bytes::Bytes::from_static(b"message"),
                ).await?;
                info!("Send a message");
            }
            _ = tokio::signal::ctrl_c() => {
                return Ok(());
            }
        }
    }
}
