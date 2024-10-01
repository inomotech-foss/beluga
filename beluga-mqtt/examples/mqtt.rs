use core::time::Duration;

use beluga_mqtt::{MqttClient, MqttClientBuilder, QoS};
use tracing::{info, Level};

struct MqttClientWrapper {
    client: MqttClient,
    _ca_content: Vec<u8>,
    _cert_content: Vec<u8>,
    _key_content: Vec<u8>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let use_tls = true; // Or false, depending on your setup

    let thing_name_content = tokio::fs::read_to_string("thing-name.in").await?;
    let endpoint_content = tokio::fs::read_to_string("endpoint.in").await?;

    // Read the certificate files and store them in variables
    let ca_content_vec = tokio::fs::read("AmazonRootCA1.pem").await?;
    let cert_content_vec = tokio::fs::read("certificate.pem.crt").await?;
    let key_content_vec = tokio::fs::read("private.pem.key").await?;

    let mut builder = MqttClientBuilder::new()
        .thing_name(&thing_name_content)
        .endpoint(&endpoint_content);

    if use_tls {
        builder = builder.set_tls_transport(
            &ca_content_vec,
            &cert_content_vec,
            &key_content_vec,
        );
    } else {
        builder = builder.set_tcp_transport();
    }

    let client = builder.build()?;

    // Move the certificate data into the struct
    let mqtt_client_wrapper = MqttClientWrapper {
        client,
        _ca_content: ca_content_vec,
        _cert_content: cert_content_vec,
        _key_content: key_content_vec,
    };

    let mut msg_sub = mqtt_client_wrapper
        .client
        .subscribe_many(["message", "other", "some"], QoS::AtLeastOnce)
        .await?;

    let _j = tokio::spawn(async move {
        loop {
            let msg = msg_sub.recv().await.unwrap();
            info!("Message: {:?}", msg);
        }
    });

    loop {
        mqtt_client_wrapper
            .client
            .publish(
                "message",
                QoS::AtLeastOnce,
                false,
                bytes::Bytes::from_static(b"Hello World"),
            )
            .await?;

        tokio::time::sleep(Duration::from_secs(20)).await;
    }
}