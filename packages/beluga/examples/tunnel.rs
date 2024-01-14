use beluga::tunnel::TunnelClient;
use beluga::{ConfigBuilder, MqttClient, Qos};
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_level(true)
        .with_file(false)
        .with_line_number(false)
        .with_target(true)
        .with_max_level(Level::DEBUG)
        .init();

    let private_key = std::fs::read("...").unwrap();
    let cert = std::fs::read("...").unwrap();

    let client = MqttClient::create(
        ConfigBuilder::new()
            .with_endpoint("...")
            .unwrap()
            .with_client_id("myclientid")
            .unwrap()
            .with_clean_session()
            .with_private_key(private_key)
            .with_cert(cert)
            .build()
            .unwrap(),
    )
    .await
    .unwrap();

    let _tunnel_client = TunnelClient::create(client.clone(), Qos::AtLeastOnce, "...")
        .await
        .unwrap();

    let _ = tokio::signal::ctrl_c().await;
}
