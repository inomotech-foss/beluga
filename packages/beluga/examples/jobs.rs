use beluga::{ConfigBuilder, JobsClient, MqttClient, Qos};
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

    let jobs_client = JobsClient::new(client.clone(), Qos::AtLeastOnce, "...")
        .await
        .unwrap();

    let summary = jobs_client.pending_jobs(Qos::AtLeastOnce).await.unwrap();
    println!("Summary {summary:?}");

    let mut job = jobs_client
        .start_next_pending_job(Qos::AtLeastOnce, chrono::Duration::minutes(5).into())
        .await
        .unwrap();

    println!(
        "Pending Job: {:?}, {}, {}",
        job.document(),
        job.id(),
        job.name()
    );

    let descr = job.describe_execution(Qos::AtLeastOnce).await.unwrap();

    println!("{descr:?}");

    let descr = job
        .update(Qos::AtLeastOnce, 2, beluga::JobStatus::SUCCEEDED)
        .await
        .unwrap();

    println!("{descr:?}");
}
