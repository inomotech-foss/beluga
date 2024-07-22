use std::time::Duration;

use beluga_mqtt::QoS;
use common::{ca, client, mqtt_server, port, server_certs, signed_cert};
use tokio::time::timeout;

mod common;

#[tokio::test]
async fn connection_handling() {
    let temp_dir = tempfile::tempdir().unwrap();
    let (ca, ca_key) = ca().unwrap();
    let (client_cert_1, client_key_1) = signed_cert(&ca, &ca_key).unwrap();
    let (client_cert_2, client_key_2) = signed_cert(&ca, &ca_key).unwrap();
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

    let mqtt_client = client(&ca, &client_cert_1, &client_key_1, port, "ThingName").unwrap();
    let mut s = mqtt_client
        .subscribe("topic", rumqttc::QoS::AtLeastOnce)
        .await
        .unwrap();

    let second_client = client(&ca, &client_cert_2, &client_key_2, port, "OtherThingName").unwrap();

    second_client
        .publish("topic", QoS::AtLeastOnce, false, "message".into())
        .await
        .unwrap();

    tokio::select! {
        p = s.recv() => {
            let packet = p.unwrap();
            assert_eq!(packet.payload, "message");
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => {
            panic!("timeout");
        }
    }
}

#[tokio::test]
async fn reconnect_client() {
    let temp_dir = tempfile::tempdir().unwrap();
    let (ca, ca_key) = ca().unwrap();
    let (cert, key) = signed_cert(&ca, &ca_key).unwrap();
    let (ca_path, cert_path, key_path) = server_certs(temp_dir.path(), &ca, &ca_key).await.unwrap();
    let port = port();

    let handler = mqtt_server(
        ca_path.to_str().unwrap().to_owned(),
        cert_path.to_str().unwrap().to_owned(),
        key_path.to_str().unwrap().to_owned(),
        port,
    );

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = client(&ca, &cert, &key, port, "ThingName").unwrap();
    let mut s = client
        .subscribe("topic", rumqttc::QoS::AtLeastOnce)
        .await
        .unwrap();

    handler.shutdown();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let _guard = mqtt_server(
        ca_path.to_str().unwrap().to_owned(),
        cert_path.to_str().unwrap().to_owned(),
        key_path.to_str().unwrap().to_owned(),
        port,
    )
    .drop_guard();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client_cp = client.clone();

    let publish = async move {
        while client_cp
            .publish("topic", QoS::AtLeastOnce, false, "message".into())
            .await
            .is_err()
        {}
    };

    if timeout(Duration::from_secs(1), publish).await.is_err() {
        panic!("couldn't publish message in time");
    }

    let receive = async move {
        loop {
            if let Ok(packet) = s.recv().await {
                break packet;
            }
        }
    };

    if let Ok(packet) = timeout(Duration::from_secs(1), receive).await {
        assert_eq!(packet.payload, "message");
    } else {
        panic!("couldn't receive a message in time");
    }
}
