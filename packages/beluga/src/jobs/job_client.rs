use std::ffi::{c_char, CString};
use std::ops::Deref;
use std::os::raw::c_void;
use std::sync::Arc;

use futures::FutureExt;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::common::{JobDescription, JobInfoOwned, JobsSummaryOwned, NextPendingRequest};
use super::job::{Description, Job};
use super::job_client_callbacks::{
    create_get_pending_job_executions_accepted, create_get_pending_job_executions_rejected,
    create_job_executions_changed_events, create_next_job_execution_changed_events,
    create_publish_completed_jobs, create_start_next_pending_job_execution_accepted,
    create_start_next_pending_job_execution_rejected, create_subscribe_completed_jobs,
    JobsClientInterface,
};
use super::RejectedOwned;
use crate::common::SharedPtr;
use crate::mqtt::InternalMqttClient;
use crate::{Error, MqttClient, Qos, Result};

type ArcReceiver<T> = Arc<Mutex<mpsc::Receiver<T>>>;

#[derive(Debug)]
pub(super) struct InternalJobsClientPointer {
    internal_client: *const InternalJobsClient,
}

impl Deref for InternalJobsClientPointer {
    type Target = *const InternalJobsClient;
    fn deref(&self) -> &Self::Target {
        &self.internal_client
    }
}

unsafe impl Send for InternalJobsClientPointer {}
unsafe impl Sync for InternalJobsClientPointer {}

#[repr(C)]
pub(crate) struct InternalJobsClient {
    client: SharedPtr,
    interface: *const c_void,
}

extern "C" {
    fn internal_jobs_client(
        mqtt_client: *const InternalMqttClient,
        interface: *const c_void,
        qos: Qos,
        thing_name: *const c_char,
    ) -> *const InternalJobsClient;
    fn publish_get_pending_executions(
        client: *const InternalJobsClient,
        qos: Qos,
        callback: *const c_void,
    ) -> bool;
    fn publish_start_next_pending_execution(
        client: *const InternalJobsClient,
        qos: Qos,
        callback: *const c_void,
        request: NextPendingRequest,
    ) -> bool;
    fn drop_jobs_client(client: *const InternalJobsClient);
}

#[derive(Debug, Clone)]
pub struct JobsClient {
    _interface: Arc<JobsClientInterface>,
    mqtt_client: Arc<MqttClient>,
    internal_client: Arc<Mutex<InternalJobsClientPointer>>,
    get_pending_rx: ArcReceiver<JobsSummaryOwned>,
    get_pending_rejected_rx: ArcReceiver<RejectedOwned>,
    start_next_pending_rx: ArcReceiver<JobDescription>,
    start_next_pending_rejected_rx: ArcReceiver<RejectedOwned>,
    next_exec_changed_rx: ArcReceiver<JobDescription>,
    thing_name: String,
}

impl Drop for JobsClient {
    fn drop(&mut self) {
        unsafe {
            tokio::task::block_in_place(|| {
                let mut guard = self.internal_client.blocking_lock();
                drop_jobs_client(guard.internal_client);
                guard.internal_client = std::ptr::null();
            });
        }
    }
}

impl JobsClient {
    pub async fn new(mqtt_client: Arc<MqttClient>, qos: Qos, thing_name: &str) -> Result<Self> {
        let c_thing_name = CString::new(thing_name).map_err(Error::StringConversion)?;

        let (subscribe_completed_tx, mut subscribe_completed_rx) = mpsc::channel::<bool>(1);

        let (get_pending_tx, get_pending_rx) = mpsc::channel::<JobsSummaryOwned>(10);
        let (get_pending_rejected_tx, get_pending_rejected_rx) = mpsc::channel::<RejectedOwned>(10);
        let (start_next_pending_tx, start_next_pending_rx) = mpsc::channel::<JobDescription>(10);
        let (start_next_pending_rejected_tx, start_next_pending_rejected_rx) =
            mpsc::channel::<RejectedOwned>(10);
        let (next_exec_changed_tx, next_exec_changed_rx) = mpsc::channel::<JobDescription>(10);

        let jobs_interface = Arc::new(JobsClientInterface {
            subscribe_completed_jobs: Box::new(create_subscribe_completed_jobs(
                subscribe_completed_tx,
            )),
            publish_completed_jobs: Box::new(create_publish_completed_jobs()),
            get_pending_job_executions_accepted: Box::new(
                create_get_pending_job_executions_accepted(get_pending_tx),
            ),
            get_pending_job_executions_rejected: Box::new(
                create_get_pending_job_executions_rejected(get_pending_rejected_tx),
            ),
            start_next_pending_job_execution_accepted: Box::new(
                create_start_next_pending_job_execution_accepted(start_next_pending_tx),
            ),
            start_next_pending_job_execution_rejected: Box::new(
                create_start_next_pending_job_execution_rejected(start_next_pending_rejected_tx),
            ),
            job_executions_changed_events: Box::new(create_job_executions_changed_events()),
            next_job_execution_changed_events: Box::new(create_next_job_execution_changed_events(
                next_exec_changed_tx,
            )),
        });

        let internal_client = {
            let internal_client = mqtt_client.internal_client();
            let client = internal_client.lock();

            let internal_client = unsafe {
                internal_jobs_client(
                    **client,
                    (jobs_interface.as_ref() as *const JobsClientInterface).cast(),
                    qos,
                    c_thing_name.as_ptr(),
                )
            };

            if internal_client.is_null() {
                return Err(Error::JobsClientCreate);
            }

            Arc::new(Mutex::new(InternalJobsClientPointer { internal_client }))
        };

        if let Some(true) = subscribe_completed_rx.recv().await {
            Ok(Self {
                _interface: jobs_interface,
                mqtt_client,
                internal_client,
                get_pending_rx: Arc::new(Mutex::new(get_pending_rx)),
                get_pending_rejected_rx: Arc::new(Mutex::new(get_pending_rejected_rx)),
                start_next_pending_rx: Arc::new(Mutex::new(start_next_pending_rx)),
                start_next_pending_rejected_rx: Arc::new(Mutex::new(
                    start_next_pending_rejected_rx,
                )),
                next_exec_changed_rx: Arc::new(Mutex::new(next_exec_changed_rx)),
                thing_name: thing_name.to_owned(),
            })
        } else {
            Err(Error::JobsClientCreate)
        }
    }

    /// Retrieves a summary of pending job executions for the thing.
    /// Publishes a request via **MQTT** to get pending job executions.
    /// Waits on a response or rejection via internal channels.
    /// 
    /// # Arguments
    /// 
    /// - `qos` - specifies the Quality of Service level to use for the request.
    /// 
    /// # Returns
    /// [`Ok`] with the [`JobsSummaryOwned`] on success, or an [`Error`] on failure.
    pub async fn pending_jobs(&self, qos: Qos) -> Result<JobsSummaryOwned> {
        let (tx, rx) = oneshot::channel::<()>();
        {
            let client = self.internal_client.lock().await;
            let callback =
                Box::into_raw(Box::<Box<dyn FnOnce(i32)>>::new(Box::new(move |_io_err| {
                    let _ = tx.send(());
                })));

            unsafe { publish_get_pending_executions(**client, qos, callback.cast()) };
        }
        rx.await.map_err(Error::Publish)?;

        let mut rx = self.get_pending_rx.lock().await;
        let mut rejected_rx = self.get_pending_rejected_rx.lock().await;

        futures::select! {
            summary = rx.recv().fuse() => {
                Ok(summary.ok_or(Error::Receive)?)
            }
            rejected = rejected_rx.recv().fuse() => {
                Err(rejected.map(Error::Rejected).ok_or(Error::Receive)?)
            }
        }
    }

    /// Retrieves the next pending job execution for the thing and starts it.
    /// Publishes a request via MQTT to start the next pending job execution.
    /// Waits on a response or rejection via internal channels.
    /// 
    /// # Arguments
    /// 
    /// - `qos` - specifies the Quality of Service level to use for the request.
    /// - `timeout` - specifies a timeout duration to apply to the job step, in minutes.
    /// 
    /// # Returns
    /// [`Ok`] with the started [`Job`] on success, or an [`Error`] on failure.
    pub async fn start_next_pending_job(
        &self,
        qos: Qos,
        timeout: Option<chrono::Duration>,
    ) -> Result<Job> {
        let (tx, rx) = oneshot::channel::<()>();
        let step_timeout = timeout.map(|dur| dur.num_minutes()).unwrap_or(-1);

        let request = if step_timeout > 0 {
            NextPendingRequest {
                step_timeout: &step_timeout,
            }
        } else {
            NextPendingRequest {
                step_timeout: std::ptr::null(),
            }
        };

        {
            let client = self.internal_client.lock().await;
            let callback =
                Box::into_raw(Box::<Box<dyn FnOnce(i32)>>::new(Box::new(move |_io_err| {
                    let _ = tx.send(());
                })));

            unsafe {
                publish_start_next_pending_execution(**client, qos, callback.cast(), request)
            };
        }

        rx.await.map_err(Error::Publish)?;

        let mut rx = self.start_next_pending_rx.lock().await;
        let mut rejected_rx = self.start_next_pending_rejected_rx.lock().await;

        futures::select! {
            descr = rx.recv().fuse() => {
                description_to_job(self.mqtt_client.clone(), self.name().to_owned(), descr.ok_or(Error::Receive)?).await
            }
            rejected = rejected_rx.recv().fuse() => {
                Err(rejected.map(Error::Rejected).ok_or(Error::Receive)?)
            }
        }
    }

    /// Returns the next [`JobDescription`]. This allows
    /// detecting when the next pending job changes.
    pub async fn next_execution_changed(&self) -> Result<JobDescription> {
        self.next_exec_changed_rx
            .lock()
            .await
            .recv()
            .await
            .ok_or(Error::Receive)
    }

    pub fn name(&self) -> &str {
        &self.thing_name
    }
}

async fn description_to_job(
    client: Arc<MqttClient>,
    thing_name: String,
    description: JobDescription,
) -> Result<Job> {
    let JobDescription {
        data:
            Some(JobInfoOwned {
                job_id: Some(job_id),
                status: Some(status),
                execution_number: Some(execution_number),
                job_document: Some(document),
                ..
            }),
        ..
    } = description
    else {
        return Err(Error::NoPendingJobs);
    };

    Job::new(
        client,
        Qos::AtLeastOnce,
        Description {
            thing_name,
            job_id,
            status,
            execution_number,
            document: document.into(),
        },
    )
    .await
}
