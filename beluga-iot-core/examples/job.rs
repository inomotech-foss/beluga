use core::time::Duration;

use beluga_aws_sdk::{JobStatus, JobsClient};
use beluga_mqtt::MqttClientBuilder;
use tracing::Level;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let client = MqttClientBuilder::new()
        .ca(&tokio::fs::read("AmazonRootCA1.pem").await?)
        .certificate(&tokio::fs::read("certificate.pem.crt").await?)
        .private_key(&tokio::fs::read("private.pem.key").await?)
        .endpoint(&tokio::fs::read_to_string("endpoint.in").await?)
        .thing_name(&tokio::fs::read_to_string("thing-name.in").await?)
        .build()?;

    let jobs_client = JobsClient::new(client).await?;
    let (_in_progress, queued) = jobs_client.get().await?;

    for mut job in queued {
        job.update(JobStatus::InProgress).await?;
        tokio::time::sleep(Duration::from_secs(15)).await;
        job.update(JobStatus::Succeeded).await?;
    }

    Ok(())
}
