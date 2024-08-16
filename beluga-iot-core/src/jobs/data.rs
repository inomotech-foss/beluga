use std::collections::HashMap;
use std::fmt::Display;

use chrono::prelude::Utc;
use serde::{Deserialize, Serialize};

use crate::jobs::datetime;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobStatus {
    Queued,
    InProgress,
    Failed,
    Succeeded,
    Canceled,
    TimedOut,
    Rejected,
    Removed,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JobExecution {
    /// The unique identifier you assigned to this job when it was created.
    #[serde(rename = "jobId")]
    pub job_id: Option<String>,
    /// The name of the thing that is executing the job.
    #[serde(rename = "thingName")]
    pub thing_name: Option<String>,
    /// The content of the job document.
    #[serde(rename = "jobDocument")]
    pub document: Option<serde_json::Value>,
    /// The status of the job execution.
    pub status: Option<JobStatus>,
    /// A collection of name/value pairs that describe the status of the job
    /// execution. The maximum length of the value in the name/value pair is
    /// 1,024 characters.
    #[serde(rename = "statusDetails")]
    pub details: Option<HashMap<String, String>>,
    /// The time, in seconds since the epoch, when the job execution was
    /// enqueued.
    #[serde(default)]
    #[serde(rename = "queuedAt", with = "datetime")]
    pub queued_at: Option<chrono::DateTime<Utc>>,
    /// The time, in seconds since the epoch, when the job execution was
    /// started.
    #[serde(default)]
    #[serde(rename = "startedAt", with = "datetime")]
    pub started_at: Option<chrono::DateTime<Utc>>,
    /// The time, in seconds since the epoch, when the job execution was last
    /// updated.
    #[serde(default)]
    #[serde(rename = "lastUpdatedAt", with = "datetime")]
    pub last_update_at: Option<chrono::DateTime<Utc>>,
    /// The version of the job execution. Job execution versions are incremented
    /// each time they are updated by a device.
    #[serde(rename = "versionNumber")]
    pub version: Option<i32>,
    /// A number that identifies a particular job execution on a particular
    /// device. It can be used later in commands that return or update job
    /// execution information.
    #[serde(rename = "executionNumber")]
    pub execution: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JobExecutionState {
    /// The status of the job execution.
    pub status: Option<JobStatus>,
    /// A collection of name/value pairs that describe the status of the job
    /// execution. The maximum length of the value in the name/value pair is
    /// 1,024 characters.
    #[serde(rename = "statusDetails")]
    pub details: Option<HashMap<String, String>>,
    /// The version of the job execution. Job execution versions are incremented
    /// each time they are updated by a device.
    #[serde(rename = "versionNumber")]
    pub version: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JobExecutionSummary {
    /// The unique identifier you assigned to this job when it was created.
    #[serde(rename = "jobId")]
    pub job_id: Option<String>,
    /// The time, in seconds since the epoch, when the job execution was
    /// enqueued.
    #[serde(default)]
    #[serde(rename = "queuedAt", with = "datetime")]
    pub queued_at: Option<chrono::DateTime<Utc>>,
    /// The time, in seconds since the epoch, when the job execution started.
    #[serde(default)]
    #[serde(rename = "startedAt", with = "datetime")]
    pub started_at: Option<chrono::DateTime<Utc>>,
    /// The time, in seconds since the epoch, when the job execution was last
    /// updated.
    #[serde(default)]
    #[serde(rename = "lastUpdatedAt", with = "datetime")]
    pub last_update_at: Option<chrono::DateTime<Utc>>,
    /// The version of the job execution. Job execution versions are incremented
    /// each time AWS IoT Jobs receives an update from a device.
    #[serde(rename = "versionNumber")]
    pub version: Option<i32>,
    /// A number that identifies a particular job execution on a particular
    /// device.
    #[serde(rename = "executionNumber")]
    pub execution: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct GetPendingJobsExecutionReq {
    #[serde(rename = "clientToken")]
    pub(super) token: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct GetPendingJobsExecutionResp {
    #[serde(rename = "inProgressJobs")]
    pub(super) in_progress_jobs: Vec<JobExecutionSummary>,
    #[serde(rename = "queuedJobs")]
    pub(super) queued_jobs: Vec<JobExecutionSummary>,
    #[serde(default)]
    #[serde(with = "datetime")]
    pub(super) timestamp: Option<chrono::DateTime<Utc>>,
    #[serde(rename = "clientToken")]
    pub(super) token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct StartNextPendingJobExecutionReq {
    /// A collection of name-value pairs that describe the status of the job
    /// execution. If not specified, the statusDetails are unchanged.
    #[serde(rename = "statusDetails")]
    pub(super) details: Option<HashMap<String, String>>,
    /// Specifies the amount of time this device has to finish execution of this
    /// job. If the job execution status isn't set to a terminal state
    /// before this timer expires, or before the timer is reset, (by calling
    /// UpdateJobExecution, setting the status to IN_PROGRESS and specifying a
    /// new timeout value in field stepTimeoutInMinutes) the job execution
    /// status is set to TIMED_OUT. Setting this timeout has no effect on
    /// that job execution timeout that might have been specified when the job
    /// was created (CreateJob using the timeoutConfig field).
    #[serde(rename = "stepTimeoutInMinutes")]
    pub(super) step_timeout: Option<i64>,
    #[serde(rename = "clientToken")]
    pub(super) token: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct StartNextPendingJobExecutionResp {
    pub(super) execution: Option<JobExecution>,
    #[serde(with = "datetime")]
    pub(super) timestamp: Option<chrono::DateTime<Utc>>,
    #[serde(rename = "clientToken")]
    pub(super) token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub(super) struct DescribeJobExecutionReq {
    /// The unique identifier assigned to this job when it was created.
    #[serde(rename = "jobId")]
    pub(super) job_id: Option<String>,
    /// The name of the thing associated with the device.
    #[serde(rename = "thingName")]
    pub(super) thing_name: Option<String>,
    /// A number that identifies a job execution on a device. If not specified,
    /// the latest job execution is returned.
    #[serde(rename = "executionNumber")]
    pub(super) execution: Option<i64>,
    /// Unless set to false, the response contains the job document. The default
    /// is true.
    #[serde(rename = "includeJobDocument")]
    pub(super) include_job_doc: Option<bool>,
    #[serde(rename = "clientToken")]
    pub(super) token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct DescribeJobExecutionResp {
    pub(super) execution: Option<JobExecution>,
    #[serde(default)]
    #[serde(with = "datetime")]
    pub(super) timestamp: Option<chrono::DateTime<Utc>>,
    #[serde(rename = "clientToken")]
    pub(super) token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct UpdateJobExecutionReq {
    pub(super) status: JobStatus,
    #[serde(rename = "statusDetails")]
    pub(super) details: Option<HashMap<String, String>>,
    #[serde(rename = "expectedVersion")]
    pub(super) expected_version: i32,
    #[serde(rename = "executionNumber")]
    pub(super) execution: Option<i64>,
    /// When included and set to true, the response contains the
    /// JobExecutionState field. The default is false.
    #[serde(rename = "includeJobExecutionState")]
    pub(super) include_job_state: Option<bool>,
    /// When included and set to true, the response contains the JobDocument.
    /// The default is false.
    #[serde(rename = "includeJobDocument")]
    pub(super) include_job_doc: Option<bool>,
    /// Specifies the amount of time this device has to finish execution of this
    /// job. If the job execution status is not set to a terminal state before
    /// this timer expires, or before the timer is reset, the job execution
    /// status is set to TIMED_OUT. Setting or resetting this timeout has no
    /// effect on the job execution timeout that might have been specified when
    /// the job was created.
    #[serde(rename = "stepTimeoutInMinutes")]
    pub(super) step_timeout_min: Option<i64>,
    #[serde(rename = "clientToken")]
    pub(super) token: Option<String>,
}

impl UpdateJobExecutionReq {
    pub(super) fn new(
        status: JobStatus,
        expected_version: i32,
        details: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            status,
            details,
            expected_version,
            execution: None,
            include_job_state: None,
            include_job_doc: None,
            step_timeout_min: None,
            token: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct UpdateJobExecutionResp {
    #[serde(rename = "executionState")]
    pub(super) execution_state: Option<JobExecutionState>,
    #[serde(rename = "jobDocument")]
    pub(super) document: Option<serde_json::Value>,
    #[serde(default)]
    #[serde(with = "datetime")]
    pub(super) timestamp: Option<chrono::DateTime<Utc>>,
    #[serde(rename = "clientToken")]
    pub(super) token: Option<String>,
}

/// A struct representing an error that occurred when a request to the AWS IoT
/// Jobs service was rejected.
///
/// This struct contains information about the error, including an optional
/// client token, error message, timestamp, and job execution state (if
/// applicable).
#[derive(Debug, Deserialize, Clone, thiserror::Error)]
pub struct RejectedError {
    /// The error code that indicates why a request to the AWS IoT Jobs service
    /// was rejected.
    pub code: RejectedErrorCode,
    /// Opaque token that can correlate this response to the original request.
    #[serde(rename = "clientToken")]
    pub token: Option<String>,
    /// A text message that provides additional information.
    pub message: Option<String>,
    /// The date and time the response was generated by AWS IoT.
    #[serde(default)]
    #[serde(with = "datetime")]
    pub timestamp: Option<chrono::DateTime<Utc>>,
    /// A JobExecutionState object. This field is included only when the code
    /// field has the value `InvalidStateTransition` or `VersionMismatch`.
    #[serde(rename = "executionState")]
    pub execution_state: Option<JobExecutionState>,
}

/// An enum representing the possible error codes that can be returned when a
/// request to the AWS IoT Jobs service is rejected.
#[derive(Debug, Deserialize, Clone, Copy, strum::Display)]
pub enum RejectedErrorCode {
    /// The request was sent to a topic in the AWS IoT Jobs namespace that does
    /// not map to any API.
    InvalidTopic,
    /// The contents of the request could not be interpreted as valid
    /// UTF-8-encoded JSON.
    InvalidJson,
    /// The contents of the request were invalid. The message contains details
    /// about the error.
    InvalidRequest,
    /// An update attempted to change the job execution to a state that is
    /// invalid because of the job execution's current state. In this case,
    /// the body of the error message also contains the executionState
    /// field.
    InvalidStateTransition,
    /// The JobExecution specified by the request topic does not exist.
    ResourceNotFound,
    /// The expected version specified in the request does not match the version
    /// of the job execution in the AWS IoT Jobs service. In this case, the
    /// body of the error message also contains the executionState field.
    VersionMismatch,
    /// There was an internal error during the processing of the request.
    InternalError,
    /// The request was throttled.
    RequestThrottled,
    /// Occurs when a command to describe a job is performed on a job that is in
    /// a terminal state.
    TerminalStateReached,
}

impl Display for RejectedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = write!(f, "Rejected Error - Code: \"{}\"", self.code);

        if let Some(msg) = &self.message {
            res = write!(f, ", Message: \"{msg}\"");
        }

        if let Some(token) = &self.token {
            res = write!(f, ", Token: \"{token}\"");
        }

        res
    }
}
