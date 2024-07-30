use beluga_mqtt::Publish;

use crate::jobs::RejectedError;
use crate::provision::ProvisionError;

/// The main error type for the Beluga IoT Core crate.
///
/// This enum represents the various errors that can occur within the Beluga IoT
/// Core crate. It includes errors related to serialization, MQTT, tunneling,
/// task joining, job management, and CBOR (when the `cbor` feature is enabled).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error occurred during JSON serialization or deserialization.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// An error occurred within the Beluga MQTT crate.
    #[error(transparent)]
    Mqtt(#[from] beluga_mqtt::Error),
    /// An error occurred within the Beluga Tunnel crate.
    #[error(transparent)]
    Tunnel(#[from] beluga_tunnel::Error),
    /// An error occurred while joining a task.
    #[error(transparent)]
    TaskJoin(#[from] tokio::task::JoinError),
    /// The job ID is missing, which is required to perform the requested
    /// operation.
    #[error("the job ID is missing, which is required to perform the requested operation")]
    JobIdMissing,
    /// The job version is missing.
    #[error("the job version missing")]
    JobVersion,
    /// The job with the specified ID is missing execution information.
    #[error("the job with ID \"{0}\" is missing execution information")]
    JobExecutionMissing(String),
    /// An error occurred due to a rejected job request.
    #[error(transparent)]
    JobRejected(#[from] RejectedError),
    /// An unexpected packet was received for the specified topic.
    #[error("received unexpected packet, for the topic \"{}\"", .0.topic)]
    UnexpectedPacket(Publish),
    /// A token mismatch occurred, with the expected and received tokens
    /// provided.
    #[error("token mismatch: expected [{expected}], but received [{received}]")]
    TokenMismatch { expected: String, received: String },
    /// An error occurred related to CBOR serialization or deserialization.
    #[cfg(feature = "cbor")]
    #[error(transparent)]
    Cbor(#[from] CborError),
    /// An error occurred related to provisioning.
    #[error(transparent)]
    Provision(#[from] ProvisionError),
}

#[cfg(feature = "cbor")]
#[derive(Debug, thiserror::Error)]
pub enum CborError {
    #[error(transparent)]
    Serialize(#[from] ciborium::ser::Error<std::io::Error>),
    #[error(transparent)]
    Deserialize(#[from] ciborium::de::Error<std::io::Error>),
}
