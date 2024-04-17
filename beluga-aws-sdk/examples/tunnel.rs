use beluga_aws_sdk::TunnelManager;
use beluga_mqtt::MqttClientBuilder;
use tracing::Level;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let client = MqttClientBuilder::new()
        .ca(include_bytes!("../../AmazonRootCA1.pem"))
        .certificate(include_bytes!("../../certificate.pem.crt"))
        .private_key(include_bytes!("../../private.pem.key"))
        .endpoint(include_str!("../../endpoint.in"))
        .thing_name(include_str!("../../thing-name.in"))
        .build()?;

    // for Github actions
    // let client = MqttClientBuilder::new()
    //     .ca(&[])
    //     .certificate(&[])
    //     .private_key(&[])
    //     .endpoint("")
    //     .thing_name("")
    //     .build()?;

    let mut manager =
        TunnelManager::new(client.clone(), include_str!("../../thing-name.in")).await?;
    let tunnel_handle = manager.recv().await?;
    tunnel_handle.await??;

    Ok(())
}
