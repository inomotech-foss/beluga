use core::time::Duration;

use beluga_mqtt::{MqttClientBuilder, QoS};
use tracing::{info, Level};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // let client = MqttClientBuilder::new()
    //     .ca(include_bytes!("../../AmazonRootCA1.pem"))
    //     .certificate(include_bytes!("../../certificate.pem.crt"))
    //     .private_key(include_bytes!("../../private.pem.key"))
    //     .endpoint(include_str!("../../endpoint.in"))
    //     .thing_name(include_str!("../../thing-name.in"))
    //     .build()?;

    // for Github actions
    let client = MqttClientBuilder::new()
        .ca(&[])
        .certificate(&[])
        .private_key(&[])
        .endpoint("")
        .thing_name("")
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
