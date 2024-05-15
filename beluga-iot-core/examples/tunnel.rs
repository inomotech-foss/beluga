use beluga_aws_sdk::TunnelManager;
use beluga_mqtt::MqttClientBuilder;
use tokio_util::sync::CancellationToken;
use tracing::Level;

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

    let cancel = CancellationToken::new();

    let manager = TunnelManager::new(
        client.clone(),
        &tokio::fs::read_to_string("thing-name.in").await?,
        cancel.clone(),
    )
    .await?;

    tokio::signal::ctrl_c().await?;

    cancel.cancel();
    manager.shutdown().await;

    Ok(())
}
