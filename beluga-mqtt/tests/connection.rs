use std::time::Duration;

use beluga_mqtt::{QoS, Subscriber};
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
    // Sleeps for 100 milliseconds to ensure the client connection is established
    // before the client is dropped. This is necessary to avoid race conditions
    // in the test.
    tokio::time::sleep(Duration::from_millis(100)).await;

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

#[tokio::test]
async fn reconnect_multiple_topics() {
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
    // Sleeps for 100 milliseconds to ensure the client connection is established
    // before the client is dropped. This is necessary to avoid race conditions
    // in the test.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let s1 = client
        .subscribe("topic1", rumqttc::QoS::AtLeastOnce)
        .await
        .unwrap();

    let s2 = client
        .subscribe("topic2", rumqttc::QoS::AtLeastOnce)
        .await
        .unwrap();

    let s3 = client
        .subscribe_many(["topic3", "topic4"], rumqttc::QoS::AtLeastOnce)
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

    let publish = |topic, msg: bytes::Bytes| {
        let client_cp = client.clone();
        async move {
            while client_cp
                .publish(topic, QoS::AtLeastOnce, false, msg.clone())
                .await
                .is_err()
            {}
        }
    };

    let fut = async {
        publish("topic1", "message1".into()).await;
        publish("topic2", "message2".into()).await;
        publish("topic4", "message4".into()).await;
    };

    if timeout(Duration::from_secs(1), fut).await.is_err() {
        panic!("couldn't publish message in time");
    }

    let receive = |mut s: Subscriber| async move {
        loop {
            if let Ok(packet) = s.recv().await {
                break packet;
            }
        }
    };

    if let Ok(packet) = timeout(Duration::from_secs(10), receive(s1)).await {
        assert_eq!(packet.payload, "message1");
    } else {
        panic!("couldn't receive a message in time");
    }

    if let Ok(packet) = timeout(Duration::from_secs(10), receive(s2)).await {
        assert_eq!(packet.payload, "message2");
    } else {
        panic!("couldn't receive a message in time");
    }

    if let Ok(packet) = timeout(Duration::from_secs(10), receive(s3)).await {
        assert_eq!(packet.payload, "message4");
    } else {
        panic!("couldn't receive a message in time");
    }
}
