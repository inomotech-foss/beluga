use anyhow::Context;
use beluga_aws_sdk::{JobStatus, JobsClient};
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
    let mut next_job = jobs_client
        .start_next(None)
        .await?
        .context("No pending jobs")?;

    info!("Next Job {next_job:?}");

    next_job.update(JobStatus::Succeeded).await?;
    Ok(())
}
