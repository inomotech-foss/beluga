use beluga::{ConfigBuilder, MqttClient, Qos};
use tracing::{info, Level};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_level(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .with_max_level(Level::TRACE)
        .init();

    let private_key_path = std::env::var("PRIVATE_KEY_PATH").unwrap();
    let cert_path = std::env::var("CERT_PATH").unwrap();
    let endpoint = std::env::var("ENDPOINT").unwrap();

    let private_key = std::fs::read(private_key_path).unwrap();
    let cert = std::fs::read(cert_path).unwrap();

    let client = MqttClient::create(
        ConfigBuilder::new()
            .with_endpoint(&endpoint)
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

    loop {
        let msg = client
            .publish("coooler", Qos::AtLeastOnce, false, "data".as_bytes())
            .await;

        info!("Message {msg:?}");
    }
}
