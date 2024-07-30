use std::time::Duration;

use beluga_mqtt::{MqttClientBuilder, QoS};
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

#[tokio::test]
async fn lost_old_messages_with_limited_capacity() {
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

    let client = MqttClientBuilder::new()
        .ca(ca.pem().as_bytes())
        .certificate(client_cert.pem().as_bytes())
        .private_key(client_key.serialize_pem().as_bytes())
        .thing_name("ThingName")
        .endpoint("127.0.0.1")
        .subscriber_capacity(1)
        .port(port)
        .build()
        .unwrap();

    let mut s = client.subscribe("topic", QoS::AtLeastOnce).await.unwrap();
    client
        .publish("topic", QoS::AtLeastOnce, false, "message1".into())
        .await
        .unwrap();
    client
        .publish("topic", QoS::AtLeastOnce, false, "message2".into())
        .await
        .unwrap();

    let packet = s.recv().await.unwrap();
    assert_eq!(packet.payload.as_ref(), b"message2");
}

#[tokio::test]
async fn keep_old_messages_with_sufficient_subscriber_capacity() {
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

    let client = MqttClientBuilder::new()
        .ca(ca.pem().as_bytes())
        .certificate(client_cert.pem().as_bytes())
        .private_key(client_key.serialize_pem().as_bytes())
        .thing_name("ThingName")
        .endpoint("127.0.0.1")
        .subscriber_capacity(2)
        .port(port)
        .build()
        .unwrap();

    let mut s = client.subscribe("topic", QoS::AtLeastOnce).await.unwrap();
    client
        .publish("topic", QoS::AtLeastOnce, false, "message1".into())
        .await
        .unwrap();
    client
        .publish("topic", QoS::AtLeastOnce, false, "message2".into())
        .await
        .unwrap();

    let packet = s.recv().await.unwrap();
    assert_eq!(packet.payload.as_ref(), b"message1");

    let packet = s.recv().await.unwrap();
    assert_eq!(packet.payload.as_ref(), b"message2");
}
