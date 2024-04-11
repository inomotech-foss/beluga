use beluga::{MqttClientBuilder, QoS};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let client = MqttClientBuilder::new()
        .ca(include_bytes!("../../AmazonRootCA1.pem"))
        .certificate(include_bytes!("../../certificate.pem.crt"))
        .private_key(include_bytes!("../../private.pem.key"))
        .endpoint(include_str!("../../endpoint.in"))
        .thing_name(include_str!("../../thing-name.in"))
        .build()?;

    let mut msg_sub = client.subscribe("message", QoS::AtLeastOnce).await?;
    let j = tokio::spawn(async move {
        let msg = msg_sub.recv().await.unwrap();
        info!("Message: {msg:?}");
    });

    client
        .publish(
            "message",
            QoS::AtLeastOnce,
            false,
            bytes::Bytes::from_static(b"Hello World"),
        )
        .await?;

    j.await?;

    Ok(())
}
