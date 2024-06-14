use std::time::Duration;

use beluga_mqtt::QoS;
use common::{ca, client, mqtt_server, port, server_certs, signed_cert};

mod common;

#[tokio::test]
async fn retained_messages() {
    let temp_dir = tempfile::tempdir().unwrap();
    let (ca, ca_key) = ca().unwrap();
    let (client_cert, client_key) = signed_cert(&ca, &ca_key).unwrap();
    let (ca_path, cert_path, key_path) = server_certs(temp_dir.path(), &ca, &ca_key).await.unwrap();
    let port = port();

    let _guard = mqtt_server(
        ca_path.to_str().unwrap().to_owned(),
        cert_path.to_str().unwrap().to_owned(),
        key_path.to_str().unwrap().to_owned(),
        port,
    )
    .drop_guard();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = client(&ca, &client_cert, &client_key, port, "ThingName").unwrap();

    client
        .publish(
            "retained_topic",
            QoS::AtLeastOnce,
            true,
            "retained_message".into(),
        )
        .await
        .unwrap();

    let mut s = client
        .subscribe("retained_topic", rumqttc::QoS::AtLeastOnce)
        .await
        .unwrap();

    tokio::select! {
        p = s.recv() => {
            let packet = p.unwrap();
            assert_eq!(packet.payload, "retained_message");
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => {
            panic!("timeout");
        }
    }
}

#[tokio::test]
async fn large_payload() {
    let temp_dir = tempfile::tempdir().unwrap();
    let (ca, ca_key) = ca().unwrap();
    let (client_cert, client_key) = signed_cert(&ca, &ca_key).unwrap();
    let (ca_path, cert_path, key_path) = server_certs(temp_dir.path(), &ca, &ca_key).await.unwrap();
    let port = port();

    let _guard = mqtt_server(
        ca_path.to_str().unwrap().to_owned(),
        cert_path.to_str().unwrap().to_owned(),
        key_path.to_str().unwrap().to_owned(),
        port,
    )
    .drop_guard();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = client(&ca, &client_cert, &client_key, port, "ThingName").unwrap();
    let large_message = "A".repeat(1024);
    let mut s = client
        .subscribe("large_payload_topic", QoS::AtLeastOnce)
        .await
        .unwrap();

    client
        .publish(
            "large_payload_topic",
            QoS::AtLeastOnce,
            false,
            large_message.clone().into(),
        )
        .await
        .unwrap();

    tokio::select! {
        p = s.recv() => {
            let packet = p.unwrap();
            assert_eq!(packet.payload, large_message);
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => {
            panic!("timeout");
        }
    }
}
