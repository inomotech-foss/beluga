use beluga_iot_core::{details, JobStatus, JobsClient};
use beluga_mqtt::MqttClientBuilder;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let client = MqttClientBuilder::new()
        .ca(&tokio::fs::read("AmazonRootCA1.pem").await?)
        .certificate(&tokio::fs::read("certificate.pem.crt").await?)
        .private_key(&tokio::fs::read("private.pem.key").await?)
        .endpoint(&tokio::fs::read_to_string("endpoint.in").await?)
        .thing_name(&tokio::fs::read_to_string("thing-name.in").await?)
        .build()?;

    let jobs_client = JobsClient::new(client).await?;
    let mut job = jobs_client.job("id1").await?;

    let _ = job.document().unwrap();
    job.update_with_details(JobStatus::InProgress, details! { "name" => "device" })
        .await?;

    let job = jobs_client.job("id1").await?;
    info!("Name: {}", job.details().unwrap()["name"]);

    Ok(())
}
