use std::time::Duration;

use beluga_mqtt::{Error, QoS};
use common::{ca, client, mqtt_server, port, server_certs, signed_cert};
use tokio::sync::oneshot;

mod common;

#[tokio::test]
async fn unsubscribe() {
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

    let mut subscription = client
        .subscribe("unsubscribe_topic", QoS::AtLeastOnce)
        .await
        .unwrap();
    client.unsubscribe("unsubscribe_topic").await.unwrap();

    client
        .publish(
            "unsubscribe_topic",
            QoS::AtLeastOnce,
            false,
            "message after unsubscribe".into(),
        )
        .await
        .unwrap();

    let recv_result = subscription.recv().await;
    assert!(matches!(recv_result, Err(Error::Receive(_))));
}

#[tokio::test]
async fn unsubscribe_single() {
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

    let subscriber = client
        .subscribe("unsubscribe_topic", QoS::AtLeastOnce)
        .await
        .unwrap();

    let mut subscriber_cp = subscriber.clone();
    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        assert!(matches!(subscriber_cp.recv().await, Err(Error::Receive(_))));
        let _ = tx.send(());
    });

    client.unsubscribe("unsubscribe_topic").await.unwrap();

    client
        .publish(
            "unsubscribe_topic",
            QoS::AtLeastOnce,
            false,
            "message after unsubscribe".into(),
        )
        .await
        .unwrap();

    tokio::time::timeout(Duration::from_secs(1), rx)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn unsubscribe_and_resubscribe() {
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
        .subscribe("topic_to_unsubscribe", QoS::AtLeastOnce)
        .await
        .unwrap();

    client.unsubscribe("topic_to_unsubscribe").await.unwrap();

    client
        .publish(
            "topic_to_unsubscribe",
            QoS::AtLeastOnce,
            false,
            "message after unsubscribe 1".into(),
        )
        .await
        .unwrap();

    // Resubscribe
    let mut resubscriber = client
        .subscribe("topic_to_unsubscribe", QoS::AtLeastOnce)
        .await
        .unwrap();

    client
        .publish(
            "topic_to_unsubscribe",
            QoS::AtLeastOnce,
            false,
            "message after resubscribe 2".into(),
        )
        .await
        .unwrap();

    let msg = resubscriber.recv().await.unwrap();
    assert_eq!(msg.payload, "message after resubscribe 2");
    assert!(matches!(subscriber.recv().await, Err(Error::Receive(_))));
}

#[tokio::test]
async fn unsubscribe_many() {
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
        .subscribe_many(&["topic1", "topic2"], QoS::AtLeastOnce)
        .await
        .unwrap();

    client
        .unsubscribe_many(&["topic1", "topic2"])
        .await
        .unwrap();

    client
        .publish(
            "topic1",
            QoS::AtLeastOnce,
            false,
            "message for topic1".into(),
        )
        .await
        .unwrap();
    client
        .publish(
            "topic2",
            QoS::AtLeastOnce,
            false,
            "message for topic2".into(),
        )
        .await
        .unwrap();

    assert!(matches!(subscriber.recv().await, Err(Error::Receive(_))));
}
