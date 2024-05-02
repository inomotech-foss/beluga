#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Serialize(#[from] serde_json::Error),
    #[error(transparent)]
    Mqtt(#[from] beluga_mqtt::Error),
    #[error(transparent)]
    Tunnel(#[from] beluga_tunnel::Error),
    #[error(transparent)]
    TaskJoin(#[from] tokio::task::JoinError),
    #[error("the job ID is missing, which is required to perform the requested operation")]
    JobIdMissing,
    #[error("the job version missing")]
    JobVersion,
    #[error("request to get jobs was rejected")]
    GetJobsRejected,
    #[error("request to start the next job from the queue was rejected")]
    StartNextJobRequestRejected,
    #[error("request to update job with ID \"{0}\" was rejected")]
    UpdateJobRequestRejected(String),
}
