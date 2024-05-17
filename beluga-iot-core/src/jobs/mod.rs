use std::collections::HashMap;

use beluga_mqtt::{MqttClient, QoS, Subscriber};
use chrono::prelude::Utc;
use rand::distributions::Alphanumeric;
use rand::prelude::*;
use tracing::warn;

pub use self::data::JobStatus;
use self::data::{
    DescribeJobExecutionReq, GetPendingJobsExecutionReq, GetPendingJobsExecutionResp, JobExecution,
    JobExecutionSummary, StartNextPendingJobExecutionReq, StartNextPendingJobExecutionResp,
    UpdateJobExecutionReq, UpdateJobExecutionResp,
};
use crate::error::Error;
use crate::jobs::data::DescribeJobExecutionResp;
use crate::Result;

mod data;
mod datetime;

/// The `JobsClient` struct is responsible for interacting with AWS IoT Jobs
/// through an MQTT client.
#[derive(Debug, Clone)]
pub struct JobsClient {
    mqtt: MqttClient,
    subscriber: Subscriber,
}

impl JobsClient {
    /// Creates a new `JobsClient` instance with the provided MQTT client.
    ///
    /// This function sets up the necessary MQTT subscriptions to handle
    /// job-related events, such as job acceptance and rejection.
    ///
    /// # Arguments
    /// * `mqtt` - The MQTT client to use for communicating with the AWS IoT
    ///   Jobs service.
    ///
    /// # Returns
    /// A `Result` containing the new `JobsClient` instance, or an error if the
    /// MQTT subscriptions could not be established.
    pub async fn new(mqtt: MqttClient) -> Result<Self> {
        let subscriber = mqtt
            .subscribe_many(
                [
                    get_accepted(mqtt.thing_name()),
                    get_rejected(mqtt.thing_name()),
                    start_next_accepted(mqtt.thing_name()),
                    start_next_rejected(mqtt.thing_name()),
                ],
                QoS::AtLeastOnce,
            )
            .await?;

        Ok(Self { mqtt, subscriber })
    }

    /// Retrieves the job for the specified job ID.
    ///
    /// # Arguments
    /// * `job_id` - The ID of the job to retrieve.
    ///
    /// # Returns
    /// A `Result` containing the `Job` struct with the job execution details,
    /// or an error if the request fails.
    pub async fn job(&mut self, job_id: &str) -> Result<Job> {
        let thing_name = self.mqtt.thing_name();
        let topics = &[
            get_job_accepted(thing_name, job_id),
            get_job_rejected(thing_name, job_id),
        ];

        let mut subscriber = self.mqtt.subscribe_many(topics, QoS::AtLeastOnce).await?;

        let res = async {
            let get_job_token = token();
            self.mqtt
                .publish(
                    format!("$aws/things/{thing_name}/jobs/{job_id}/get"),
                    QoS::AtLeastOnce,
                    false,
                    bytes::Bytes::copy_from_slice(&serde_json::to_vec(&DescribeJobExecutionReq {
                        token: get_job_token.clone().into(),
                        ..Default::default()
                    })?),
                )
                .await?;

            loop {
                let packet = subscriber.recv().await?;

                if packet.topic == get_job_accepted(thing_name, job_id) {
                    let DescribeJobExecutionResp {
                        execution, token, ..
                    } = serde_json::from_slice::<DescribeJobExecutionResp>(&packet.payload)?;
                    if token.is_some_and(|token| token == get_job_token) {
                        let info = execution
                            .map(JobInfo::from)
                            .ok_or(Error::JobExecutionMissing(job_id.to_owned()))?;

                        return Ok(Job::new(info, self.mqtt.clone()));
                    }

                    warn!(packet = ?packet, "token mismatch during job retrieval");
                } else if packet.topic == get_job_rejected(thing_name, job_id) {
                    return Err(Error::GetJobRejected(job_id.to_owned()));
                }

                warn!(packet = ?packet, "unexpected packet received during job retrieval");
            }
        }
        .await;

        self.mqtt.unsubscribe_many(topics).await?;

        res
    }

    /// Returns all queued jobs.
    /// It's just a wrapper around `get()` but only returns the queued jobs.
    ///
    /// # Returns
    /// A `Result` containing a vector of queued `Job` instances.
    pub async fn queued_jobs(&mut self) -> Result<Vec<Job>> {
        let (_, queued_jobs) = self.get().await?;
        Ok(queued_jobs)
    }

    /// Returns all progress jobs.
    /// It's just a wrapper around `get()` but only returns the progress jobs.
    ///
    /// # Returns
    /// A `Result` containing a vector of in-progress `Job` instances.
    pub async fn progress_jobs(&mut self) -> Result<Vec<Job>> {
        let (progress_jobs, _) = self.get().await?;
        Ok(progress_jobs)
    }

    /// Retrieves the list of pending jobs from the AWS IoT Jobs service.
    ///
    /// This function publishes a request to the AWS IoT Jobs service to
    /// retrieve the list of pending jobs for the current device. It then waits
    /// for the response, which contains the lists of in-progress and queued
    /// jobs. The function returns these lists as a tuple.
    ///
    /// # Errors
    /// This function can return the following errors:
    /// - `Error::GetJobsRejected`: The request to retrieve the pending jobs was
    ///   rejected by the AWS IoT Jobs service.
    /// - Any other errors that may occur during the MQTT communication or JSON
    ///   deserialization.
    ///
    /// # Returns
    /// A `Result` containing a tuple of vectors: the first vector contains
    /// in-progress jobs, and the second vector contains queued jobs.
    pub async fn get(&mut self) -> Result<(Vec<Job>, Vec<Job>)> {
        let get_jobs_token = token();
        self.mqtt
            .publish(
                format!("$aws/things/{}/jobs/get", self.mqtt.thing_name()).as_str(),
                QoS::AtLeastOnce,
                false,
                bytes::Bytes::copy_from_slice(&serde_json::to_vec(&GetPendingJobsExecutionReq {
                    token: get_jobs_token.clone(),
                })?),
            )
            .await?;

        loop {
            let packet = self.subscriber.recv().await?;

            if packet.topic == get_accepted(self.mqtt.thing_name()) {
                let GetPendingJobsExecutionResp {
                    in_progress_jobs,
                    queued_jobs,
                    token,
                    ..
                } = serde_json::from_slice::<GetPendingJobsExecutionResp>(&packet.payload)?;
                if token.is_some_and(|token| token == get_jobs_token) {
                    return Ok((
                        in_progress_jobs
                            .into_iter()
                            .map(|summary| Job::new(JobInfo::from(summary), self.mqtt.clone()))
                            .collect(),
                        queued_jobs
                            .into_iter()
                            .map(|summary| Job::new(JobInfo::from(summary), self.mqtt.clone()))
                            .collect(),
                    ));
                }

                warn!(packet = ?packet, "token mismatch during pending jobs retrieval");
            } else if packet.topic == get_rejected(self.mqtt.thing_name()) {
                return Err(Error::GetJobsRejected);
            }

            warn!(packet = ?packet, "unexpected packet received during pending jobs retrieval");
        }
    }

    /// Starts the next pending job execution for the current MQTT client.
    ///
    /// This function publishes a message to the
    /// `$aws/things/{thing_name}/jobs/start-next` topic to request the next
    /// pending job execution. It then waits for a response on the
    /// `start_next_accepted` and `start_next_rejected` topics, and returns the
    /// new `Job` instance if the request is accepted, or an error if it is
    /// rejected.
    ///
    /// # Arguments
    /// * `details` - An optional `HashMap` that can be used to provide
    ///   additional details for the job execution.
    ///
    /// # Returns
    /// A `Result` containing the new `Job` instance, or an error if the request
    /// is rejected.
    pub async fn start_next(&mut self, details: Option<HashMap<String, String>>) -> Result<Job> {
        let start_next_job_token = token();
        self.mqtt
            .publish(
                format!("$aws/things/{}/jobs/start-next", self.mqtt.thing_name()).as_str(),
                QoS::AtLeastOnce,
                false,
                bytes::Bytes::copy_from_slice(&serde_json::to_vec(
                    &StartNextPendingJobExecutionReq {
                        token: start_next_job_token.clone(),
                        details,
                        step_timeout: None,
                    },
                )?),
            )
            .await?;

        loop {
            let packet = self.subscriber.recv().await?;

            if packet.topic == start_next_accepted(self.mqtt.thing_name()) {
                let StartNextPendingJobExecutionResp {
                    execution, token, ..
                } = serde_json::from_slice::<StartNextPendingJobExecutionResp>(&packet.payload)?;
                if token.is_some_and(|token| token == start_next_job_token) {
                    return Ok(Job::new(JobInfo::from(execution), self.mqtt.clone()));
                }

                warn!(packet = ?packet, "token mismatch during next job execution start");
            } else if packet.topic == start_next_rejected(self.mqtt.thing_name()) {
                return Err(Error::GetJobsRejected);
            }

            warn!(packet = ?packet, "unexpected packet received during next job execution start");
        }
    }
}

impl Drop for JobsClient {
    fn drop(&mut self) {
        self.mqtt.schedule_unsubscribe_many([
            get_accepted(self.mqtt.thing_name()).as_str(),
            get_rejected(self.mqtt.thing_name()).as_str(),
            start_next_accepted(self.mqtt.thing_name()).as_str(),
            start_next_rejected(self.mqtt.thing_name()).as_str(),
        ]);
    }
}

/// The `Job` struct represents a job execution and provides methods for
/// updating it's status.
#[derive(Debug, Clone)]
pub struct Job {
    info: JobInfo,
    mqtt: MqttClient,
    subscriber: Option<Subscriber>,
}

impl Job {
    /// Creates a new `Job` instance with the given job information and MQTT
    /// client.
    fn new(info: JobInfo, mqtt: MqttClient) -> Self {
        Self {
            info,
            mqtt,
            subscriber: Default::default(),
        }
    }

    /// Updates the status of the job to the specified status.
    ///
    /// This function publishes a message to the
    /// `$aws/things/{thing_name}/jobs/{job_id}/update` topic to update the
    /// job execution status. It then waits for a response on the
    /// `update_accepted` and `update_rejected` topics, and returns an error
    /// if the request is rejected.
    ///
    /// # Arguments
    /// * `status` - The new job status.
    ///
    /// # Returns
    /// A `Result` indicating the success or failure of the operation.
    ///
    /// # Errors
    /// This function may return the following errors:
    /// * `Error::JobIdMissing` - If the job ID is missing.
    /// * `Error::JobVersion` - If the job version is missing.
    /// * `Error::UpdateJobRequestRejected` - If the job update request is
    ///   rejected.
    pub async fn update(&mut self, status: JobStatus) -> Result<()> {
        let Some(job_id) = self.info.id.as_ref() else {
            return Err(Error::JobIdMissing);
        };

        let Some(version) = self.info.version else {
            return Err(Error::JobVersion);
        };

        let mut subscriber = if let Some(subscriber) = self.subscriber.take() {
            subscriber
        } else {
            self.mqtt
                .subscribe_many(
                    [
                        update_job_accepted(self.mqtt.thing_name(), job_id),
                        update_job_rejected(self.mqtt.thing_name(), job_id),
                    ],
                    QoS::AtLeastOnce,
                )
                .await?
        };

        self.mqtt
            .publish(
                &format!(
                    "$aws/things/{}/jobs/{job_id}/update",
                    self.mqtt.thing_name()
                ),
                QoS::AtLeastOnce,
                false,
                bytes::Bytes::copy_from_slice(&serde_json::to_vec(&UpdateJobExecutionReq::new(
                    status, version,
                ))?),
            )
            .await?;

        loop {
            let packet = subscriber.recv().await?;

            if packet.topic == update_job_accepted(self.mqtt.thing_name(), job_id) {
                let UpdateJobExecutionResp {
                    execution_state,
                    document,
                    ..
                } = serde_json::from_slice::<UpdateJobExecutionResp>(&packet.payload)?;

                if let Some(state) = execution_state {
                    self.info.status = state.status;
                    self.info.details = state.details;
                    self.info.version = state.version;
                    self.info.document = document;
                } else {
                    self.info.version = self.info.version.map(|it| it + 1);
                }

                self.subscriber = Some(subscriber);
                return Ok(());
            } else if packet.topic == update_job_rejected(self.mqtt.thing_name(), job_id) {
                self.subscriber = Some(subscriber);
                return Err(Error::UpdateJobRequestRejected(job_id.to_owned()));
            }

            warn!(packet = ?packet, "unexpected packet received during job update");
        }
    }

    /// Returns the ID of the job, if it exists.
    ///
    /// This method returns an `Option<&str>` that contains the job ID if it is
    /// available, or `None` if the job ID is not set.
    pub fn id(&self) -> Option<&str> {
        self.info.id.as_ref().map(AsRef::as_ref)
    }

    /// Returns the current status of the job, if available.
    pub fn status(&self) -> Option<&JobStatus> {
        self.info.status.as_ref()
    }

    /// Returns the version of the job, if available.
    pub fn version(&self) -> Option<i32> {
        self.info.version
    }

    /// Returns the execution number of the job, if available.
    ///
    /// The execution number represents the number of times this job has been
    /// executed. It can be useful for tracking the progress or status of a
    /// job.
    pub fn execution_number(&self) -> Option<i64> {
        self.info.execution
    }

    /// Returns a reference to the details map associated with the job, if it
    /// exists.
    ///
    /// The details map is a collection of key-value pairs that provide
    /// additional information about the job.
    pub fn details(&self) -> Option<&HashMap<String, String>> {
        self.info.details.as_ref()
    }

    /// Returns a clone of the document associated with the job, if any.
    pub fn document(&self) -> Option<serde_json::Value> {
        self.info.document.clone()
    }

    /// Returns the time when the job was queued, if available.
    pub fn queued_at(&self) -> Option<chrono::DateTime<Utc>> {
        self.info.queued_at
    }

    /// Returns the time when the job was started, if available.
    pub fn started_at(&self) -> Option<chrono::DateTime<Utc>> {
        self.info.started_at
    }

    /// Returns the last time the job was updated.
    pub fn last_updated_at(&self) -> Option<chrono::DateTime<Utc>> {
        self.info.last_update_at
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        if let Some((id, _subscriber)) = self.info.id.as_ref().zip(self.subscriber.as_mut()) {
            self.mqtt.schedule_unsubscribe_many([
                update_job_accepted(self.mqtt.thing_name(), id),
                update_job_rejected(self.mqtt.thing_name(), id),
            ])
        }
    }
}

#[must_use]
#[inline(always)]
fn get_job_accepted(thing_name: &str, job_id: &str) -> String {
    format!("$aws/things/{thing_name}/jobs/{job_id}/get/accepted")
}

#[must_use]
#[inline(always)]
fn get_job_rejected(thing_name: &str, job_id: &str) -> String {
    format!("$aws/things/{thing_name}/jobs/{job_id}/get/rejected")
}

#[must_use]
#[inline(always)]
fn update_job_accepted(thing_name: &str, job_id: &str) -> String {
    format!("$aws/things/{thing_name}/jobs/{job_id}/update/accepted")
}

#[must_use]
#[inline(always)]
fn update_job_rejected(thing_name: &str, job_id: &str) -> String {
    format!("$aws/things/{thing_name}/jobs/{job_id}/update/rejected")
}

#[must_use]
#[inline(always)]
fn get_accepted(thing_name: &str) -> String {
    format!("$aws/things/{thing_name}/jobs/get/accepted")
}

#[must_use]
#[inline(always)]
fn get_rejected(thing_name: &str) -> String {
    format!("$aws/things/{thing_name}/jobs/get/rejected")
}

#[must_use]
#[inline(always)]
fn start_next_accepted(thing_name: &str) -> String {
    format!("$aws/things/{thing_name}/jobs/start-next/accepted")
}

#[must_use]
#[inline(always)]
fn start_next_rejected(thing_name: &str) -> String {
    format!("$aws/things/{thing_name}/jobs/start-next/rejected")
}

/// Generates a random token string.
///
/// # Returns
/// A random token string consisting of 16 alphanumeric characters.
#[must_use]
#[inline(always)]
fn token() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(15)
        .map(char::from)
        .collect()
}

#[derive(Debug, Clone, Default)]
struct JobInfo {
    id: Option<String>,
    version: Option<i32>,
    execution: Option<i64>,
    details: Option<HashMap<String, String>>,
    document: Option<serde_json::Value>,
    status: Option<JobStatus>,
    queued_at: Option<chrono::DateTime<Utc>>,
    started_at: Option<chrono::DateTime<Utc>>,
    last_update_at: Option<chrono::DateTime<Utc>>,
}

impl From<JobExecutionSummary> for JobInfo {
    fn from(value: JobExecutionSummary) -> Self {
        Self {
            id: value.job_id,
            version: value.version,
            execution: value.execution,
            queued_at: value.queued_at,
            started_at: value.started_at,
            last_update_at: value.last_update_at,
            ..Default::default()
        }
    }
}

impl From<JobExecution> for JobInfo {
    fn from(value: JobExecution) -> Self {
        Self {
            id: value.job_id,
            version: value.version,
            execution: value.execution,
            details: value.details,
            document: value.document,
            status: value.status,
            queued_at: value.queued_at,
            started_at: value.started_at,
            last_update_at: value.last_update_at,
        }
    }
}
