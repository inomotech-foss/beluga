use core::time::Duration;

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
        .thing_name(&tokio::fs::read_to_string("thing-name.in").await?)
        .build()?;

    let mut msg_sub = client
        .subscribe_many(["message", "other", "some"], QoS::AtLeastOnce)
        .await?;

    let _j = tokio::spawn(async move {
        loop {
            let msg = msg_sub.recv().await.unwrap();
            info!("Message: {msg:?}");
        }
    });

    loop {
        client
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
