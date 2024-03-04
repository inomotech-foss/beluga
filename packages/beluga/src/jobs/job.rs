use std::ffi::{c_char, CString};
use std::ops::Deref;
use std::os::raw::c_void;
use std::sync::Arc;

use aws_iot_device_sdk_sys::root::Aws::Iotjobs::JobStatus;
use futures::FutureExt;
use tokio::sync::{mpsc, oneshot};
use tracing::error;

use super::common::{JobDescription, RejectedOwned};
use super::job_callbacks::{
    create_describe_job_execution_accepted, create_describe_job_execution_rejected,
    create_publish_completed_job, create_subscribe_completed_job,
    create_update_job_execution_accepted, create_update_job_execution_rejected, JobInterface,
};
use crate::common::SharedPtr;
use crate::mqtt::InternalMqttClient;
use crate::{Error, JobExecutionSummaryOwned, MqttClient, Qos, Result};

#[repr(C)]
pub(crate) struct InternalJob {
    client: SharedPtr,
    interface: *const c_void,
}

#[derive(Debug)]
pub(super) struct InternalJobPointer {
    internal_job: *const InternalJob,
}

impl Deref for InternalJobPointer {
    type Target = *const InternalJob;
    fn deref(&self) -> &Self::Target {
        &self.internal_job
    }
}

unsafe impl Send for InternalJobPointer {}
unsafe impl Sync for InternalJobPointer {}

extern "C" {
    fn internal_job(
        mqtt_client: *const InternalMqttClient,
        interface: *const c_void,
        qos: Qos,
        thing_name: *const c_char,
        job_id: *const c_char,
    ) -> *const InternalJob;
    fn publish_describe_execution(
        job: *const InternalJob,
        qos: Qos,
        callback: *const c_void,
        request: DescribeExecutionRequest,
    ) -> bool;
    fn publish_update_execution(
        job: *const InternalJob,
        qos: Qos,
        callback: *const c_void,
        request: UpdateExecutionRequest,
    ) -> bool;
    fn drop_job(job: *const InternalJob);
}

#[repr(C)]
struct DescribeExecutionRequest {
    execution_number: *const i64,
    include_document: *const bool,
    job_id: *const c_char,
}

#[repr(C)]
struct UpdateExecutionRequest {
    execution_number: *const i64,
    include_execution_state: *const bool,
    job_id: *const c_char,
    expected_version: *const i32,
    include_document: *const bool,
    status: *const JobStatus,
    /// Specifies the amount of time this device has to finish execution of this
    /// job in minutes.
    step_timeout: *const i64,
}

#[derive(Debug)]
pub struct Job {
    _interface: Box<JobInterface>,
    _mqtt_client: Arc<MqttClient>,
    internal_job: InternalJobPointer,
    describe_rx: mpsc::Receiver<JobDescription>,
    describe_rejected_rx: mpsc::Receiver<RejectedOwned>,
    update_rx: mpsc::Receiver<JobDescription>,
    update_rejected_rx: mpsc::Receiver<RejectedOwned>,
    thing_name: String,
    job_id: String,
    status: JobStatus,
    document: Option<serde_json::Value>,
    execution_number: i64,
}

#[derive(Debug)]
pub struct Description {
    pub thing_name: String,
    pub job_id: String,
    pub execution_number: i64,
    pub status: JobStatus,
    pub document: Option<serde_json::Value>,
}

impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            drop_job(*self.internal_job);
        }
    }
}

impl Job {
    pub async fn new_with_summary(
        mqtt_client: Arc<MqttClient>,
        thing_name: &str,
        status: JobStatus,
        summary: JobExecutionSummaryOwned,
    ) -> Result<Self> {
        let JobExecutionSummaryOwned {
            job_id: Some(job_id),
            execution_number: Some(execution_number),
            ..
        } = summary
        else {
            error!(?summary, "summary misses the critical info for a job");
            return Err(Error::JobCreate);
        };

        Job::new(
            mqtt_client,
            Qos::AtLeastOnce,
            Description {
                thing_name: thing_name.to_owned(),
                job_id,
                execution_number,
                status,
                document: None,
            },
        )
        .await
    }

    pub async fn new(
        mqtt_client: Arc<MqttClient>,
        qos: Qos,
        Description {
            thing_name,
            job_id,
            status,
            execution_number,
            document,
        }: Description,
    ) -> Result<Self> {
        let c_thing_name = CString::new(thing_name.clone()).map_err(Error::StringConversion)?;
        let c_job_id = CString::new(job_id.clone()).map_err(Error::StringConversion)?;

        let (subscribe_completed_tx, mut subscribe_completed_rx) = mpsc::channel::<bool>(1);
        let (describe_tx, describe_rx) = mpsc::channel::<JobDescription>(1);
        let (describe_rejected_tx, describe_rejected_rx) = mpsc::channel::<RejectedOwned>(1);
        let (update_tx, update_rx) = mpsc::channel::<JobDescription>(1);
        let (update_rejected_tx, update_rejected_rx) = mpsc::channel::<RejectedOwned>(1);

        let job_interface = Box::new(JobInterface {
            describe_job_execution_accepted: Box::new(create_describe_job_execution_accepted(
                describe_tx,
            )),
            describe_job_execution_rejected: Box::new(create_describe_job_execution_rejected(
                describe_rejected_tx,
            )),
            update_job_execution_accepted: Box::new(create_update_job_execution_accepted(
                update_tx,
            )),
            update_job_execution_rejected: Box::new(create_update_job_execution_rejected(
                update_rejected_tx,
            )),
            subscribe_completed_job: Box::new(create_subscribe_completed_job(
                subscribe_completed_tx,
            )),
            publish_completed_job: Box::new(create_publish_completed_job()),
        });

        let internal_job = {
            let internal_client = mqtt_client.internal_client();
            let client = internal_client.lock();

            unsafe {
                internal_job(
                    **client,
                    (job_interface.as_ref() as *const JobInterface).cast(),
                    qos,
                    c_thing_name.as_ptr(),
                    c_job_id.as_ptr(),
                )
            }
        };

        if internal_job.is_null() {
            return Err(Error::JobCreate);
        }

        let Some(true) = subscribe_completed_rx.recv().await else {
            return Err(Error::JobCreate);
        };

        Ok(Job {
            _interface: job_interface,
            _mqtt_client: mqtt_client,
            internal_job: InternalJobPointer { internal_job },
            describe_rx,
            describe_rejected_rx,
            update_rx,
            update_rejected_rx,
            thing_name,
            job_id,
            status,
            execution_number,
            document,
        })
    }

    /// Describes the execution of the job by publishing a request and waiting
    /// on the response.
    ///
    /// # Arguments
    /// - `qos` - quality of services for an execution, [`Qos`]
    ///
    /// # Returns
    /// The job execution description if successful, or an error if rejected or
    /// failed.
    pub async fn describe_execution(&mut self, qos: Qos) -> Result<JobDescription> {
        let (tx, rx) = oneshot::channel::<()>();
        let job_id = CString::new(self.id().to_owned())?;

        let execution_number = self.execution_number;
        let include_document = true;

        let request = DescribeExecutionRequest {
            execution_number: &execution_number,
            include_document: &include_document,
            job_id: job_id.as_ptr(),
        };

        let callback = Box::into_raw(Box::<Box<dyn FnOnce(i32)>>::new(Box::new(move |_io_err| {
            let _ = tx.send(());
        })));

        unsafe { publish_describe_execution(*self.internal_job, qos, callback.cast(), request) };
        rx.await.map_err(Error::Publish)?;

        futures::select! {
            descr = self.describe_rx.recv().fuse() => {
                if let Some(descr) = descr {
                    Ok(descr)
                } else {
                    Err(Error::Receive)
                }
            }
            rejected = self.describe_rejected_rx.recv().fuse() => {
                error!(rejected = ?rejected);
                Err(Error::Rejected(rejected.ok_or(Error::Receive)?))
            }
        }
    }

    /// Updates the job execution status and description. Publishes an update
    /// request and waits for the response.
    /// Increments the execution number after a successful update.
    ///
    /// # Arguments
    ///
    /// - `qos` - Specifies the Quality of Service level to use for the request.
    ///   [`Qos`]
    /// - `expected_version` - The expected current version of the job
    ///   execution.
    /// Each time you update the job execution, its version is incremented.
    /// If the version of the job execution stored in the AWS IoT Jobs service
    /// does not match, the update is rejected with a VersionMismatch error,
    /// and an ErrorResponse that contains the current job execution status data
    /// is returned.
    /// - `status` - The new status for the job execution (IN_PROGRESS, FAILED,
    ///   SUCCEEDED, or REJECTED).
    /// This must be specified on every update. [`JobStatus`]
    ///
    /// # Returns
    /// The updated job description if successful, or an error if rejected or
    /// failed.
    pub async fn update(
        &mut self,
        qos: Qos,
        expected_version: i32,
        status: JobStatus,
    ) -> Result<JobDescription> {
        let (tx, rx) = oneshot::channel::<()>();
        let job_id = CString::new(self.id().to_owned())?;

        let include_execution_state = true;
        let include_document = true;

        let request = UpdateExecutionRequest {
            execution_number: std::ptr::null(),
            include_execution_state: &include_execution_state,
            job_id: job_id.as_ptr(),
            expected_version: &expected_version,
            include_document: &include_document,
            status: &status,
            step_timeout: std::ptr::null(),
        };

        let callback = Box::into_raw(Box::<Box<dyn FnOnce(i32)>>::new(Box::new(move |_io_err| {
            let _ = tx.send(());
        })));

        unsafe { publish_update_execution(*self.internal_job, qos, callback.cast(), request) };
        rx.await.map_err(Error::Publish)?;

        futures::select! {
            descr = self.update_rx.recv().fuse() => {
                if let Some(descr) = descr {
                    self.execution_number += 1;
                    Ok(descr)
                } else {
                    Err(Error::Receive)
                }
            }
            rejected = self.update_rejected_rx.recv().fuse() => {
                error!(rejected = ?rejected);
                Err(Error::Rejected(rejected.ok_or(Error::Receive)?))
            }
        }
    }

    /// Returns the unique ID of the job.
    #[inline(always)]
    #[must_use]
    pub fn id(&self) -> &str {
        &self.job_id
    }

    /// Returns the name of the thing.
    #[inline(always)]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.thing_name
    }

    /// Returns the current status of the job.
    #[inline(always)]
    #[must_use]
    pub const fn status(&self) -> JobStatus {
        self.status
    }

    /// Returns the current execution number of the job.
    #[inline(always)]
    #[must_use]
    pub const fn execution_number(&self) -> i64 {
        self.execution_number
    }

    /// Returns the document associated with the job, if one exists.
    #[inline(always)]
    #[must_use]
    pub const fn document(&self) -> Option<&serde_json::Value> {
        self.document.as_ref()
    }
}
