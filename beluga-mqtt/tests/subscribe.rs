use std::time::Duration;

use beluga_mqtt::{Publish, QoS};
use common::{ca, client, mqtt_server, port, server_certs, signed_cert};
use tokio::sync::oneshot;

mod common;

#[tokio::test]
async fn single_subscribe_and_receive_immediately() {
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

    let client_cp = client.clone();
    let (data_tx, data_rx) = oneshot::channel::<Publish>();
    tokio::spawn(async move {
        let mut s = client_cp
            .subscribe("some", rumqttc::QoS::AtLeastOnce)
            .await
            .unwrap();

        let data = s.recv().await.unwrap();
        data_tx.send(data).unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    client
        .publish("some", QoS::AtLeastOnce, false, "not he-he".into())
        .await
        .unwrap();

    tokio::select! {
        p = data_rx => {
            let packet = p.unwrap();
            assert_eq!(packet.payload, "not he-he");
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => {
            panic!("timeout");
        }
    }
}

#[tokio::test]
async fn single_subscribe() {
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
    let mut subscriber = client
        .subscribe("some", rumqttc::QoS::AtLeastOnce)
        .await
        .unwrap();

    client
        .publish("some", QoS::AtLeastOnce, false, "not he-he".into())
        .await
        .unwrap();

    tokio::select! {
        p = subscriber.recv() => {
            let packet = p.unwrap();
            assert_eq!(packet.payload, "not he-he");
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => {
            panic!("timeout");
        }
    }
}

#[tokio::test]
async fn multiple_subscribe() {
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

    let mut s1 = client
        .subscribe("topic1", rumqttc::QoS::AtLeastOnce)
        .await
        .unwrap();

    let mut s2 = client
        .subscribe("topic2", rumqttc::QoS::AtLeastOnce)
        .await
        .unwrap();

    client
        .publish("topic1", QoS::AtLeastOnce, false, "message1".into())
        .await
        .unwrap();
    client
        .publish("topic2", QoS::AtLeastOnce, false, "message2".into())
        .await
        .unwrap();

    tokio::select! {
        p1 = s1.recv() => {
            let packet = p1.unwrap();
            assert_eq!(packet.payload, "message1");
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => {
            panic!("timeout on topic1");
        }
    }

    tokio::select! {
        p2 = s2.recv() => {
            let packet = p2.unwrap();
            assert_eq!(packet.payload, "message2");
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => {
            panic!("timeout on topic2");
        }
    }
}

#[tokio::test]
async fn qos_levels() {
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

    for qos in &[QoS::AtMostOnce, QoS::AtLeastOnce, QoS::ExactlyOnce] {
        let mut s = client.subscribe("qos_topic", *qos).await.unwrap();

        client
            .publish(
                "qos_topic",
                *qos,
                false,
                format!("message with QoS {:?}", qos).into(),
            )
            .await
            .unwrap();

        tokio::select! {
            p = s.recv() => {
                let packet = p.unwrap();
                assert_eq!(packet.payload, format!("message with QoS {:?}", qos));
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                panic!("timeout for QoS {:?}", qos);
            }
        }
    }
}

#[tokio::test]
async fn subscribe_many() {
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

    let mut subscriber = client
        .subscribe_many(&["topic_a", "topic_b"], QoS::AtLeastOnce)
        .await
        .unwrap();

    client
        .publish("topic_a", QoS::AtLeastOnce, false, "message for a".into())
        .await
        .unwrap();
    client
        .publish("topic_b", QoS::AtLeastOnce, false, "message for b".into())
        .await
        .unwrap();

    for _ in 0..2 {
        tokio::select! {
            packet = subscriber.recv() => {
                let packet = packet.unwrap();
                match packet.topic.as_str() {
                    "topic_a" => assert_eq!(packet.payload, "message for a"),
                    "topic_b" => assert_eq!(packet.payload, "message for b"),
                    _ => panic!("unexpected topic"),
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                panic!("timeout");
            }
        }
    }
}

#[tokio::test]
async fn subscribe_many_qos_levels() {
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

    for qos in &[QoS::ExactlyOnce, QoS::AtLeastOnce, QoS::AtMostOnce] {
        let mut subscriber = client
            .subscribe_many(&["qos_topic_a", "qos_topic_b"], *qos)
            .await
            .unwrap();

        client
            .publish(
                "qos_topic_a",
                *qos,
                false,
                format!("message for qos_topic_a with QoS {:?}", qos).into(),
            )
            .await
            .unwrap();
        client
            .publish(
                "qos_topic_b",
                *qos,
                false,
                format!("message for qos_topic_b with QoS {:?}", qos).into(),
            )
            .await
            .unwrap();

        for _ in 0..2 {
            tokio::select! {
                packet = subscriber.recv() => {
                    let packet = packet.unwrap();
                    match packet.topic.as_str() {
                        "qos_topic_a" => assert_eq!(packet.payload, format!("message for qos_topic_a with QoS {:?}", qos)),
                        "qos_topic_b" => assert_eq!(packet.payload, format!("message for qos_topic_b with QoS {:?}", qos)),
                        _ => panic!("unexpected topic"),
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    panic!("timeout");
                }
            }
        }

        client
            .unsubscribe_many(&["qos_topic_a", "qos_topic_b"])
            .await
            .unwrap();
    }
}
