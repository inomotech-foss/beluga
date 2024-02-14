use std::ffi::{c_char, c_void};
use std::fmt::Debug;

use aws_crt_cpp_sys::root::Aws::Crt::DateTime;
use tokio::sync::mpsc;

use super::common::{
    JobDescription, JobInfo, JobsSummary, JobsSummaryOwned, Rejected, RejectedOwned,
};
use super::{call, from_job_info, to_option_str};

#[no_mangle]
extern "C" fn on_subscribe_completed_jobs(interface: *const c_void, io_err: i32) {
    call::<JobsClientInterface>(interface, |interface| {
        interface.subscribe_completed_jobs.as_ref()(io_err);
    });
}

#[no_mangle]
extern "C" fn on_publish_completed_jobs(
    interface: *const c_void,
    callback: *const c_void,
    io_err: i32,
) {
    call::<JobsClientInterface>(interface, |interface| {
        interface.publish_completed_jobs.as_ref()(callback, io_err);
    });
}

#[no_mangle]
extern "C" fn on_next_job_execution_changed_events(
    interface: *const c_void,
    job_info: *const JobInfo,
    date_time: *const DateTime,
    io_err: i32,
) {
    call::<JobsClientInterface>(interface, |interface| {
        interface.next_job_execution_changed_events.as_ref()(job_info, date_time, io_err);
    });
}

#[no_mangle]
extern "C" fn on_get_pending_job_executions_accepted(
    interface: *const c_void,
    jobs_summary: JobsSummary,
    io_err: i32,
) {
    call::<JobsClientInterface>(interface, |interface| {
        interface.get_pending_job_executions_accepted.as_ref()(
            io_err,
            JobsSummaryOwned::from(jobs_summary),
        );
    });
}

#[no_mangle]
extern "C" fn on_get_pending_job_executions_rejected(
    interface: *const c_void,
    rejected: Rejected,
    io_err: i32,
) {
    call::<JobsClientInterface>(interface, |interface| {
        interface.get_pending_job_executions_rejected.as_ref()(rejected, io_err);
    });
}

#[no_mangle]
extern "C" fn on_start_next_pending_job_execution_accepted(
    interface: *const c_void,
    client_token: *const c_char,
    job_info: *const JobInfo,
    io_err: i32,
) {
    call::<JobsClientInterface>(interface, |interface| {
        interface.start_next_pending_job_execution_accepted.as_ref()(
            client_token,
            job_info,
            io_err,
        );
    });
}

#[no_mangle]
extern "C" fn on_start_next_pending_job_execution_rejected(
    interface: *const c_void,
    rejected: Rejected,
    io_err: i32,
) {
    call::<JobsClientInterface>(interface, |interface| {
        interface.start_next_pending_job_execution_rejected.as_ref()(rejected, io_err);
    });
}

#[no_mangle]
extern "C" fn on_job_executions_changed_events(interface: *const c_void, io_err: i32) {
    call::<JobsClientInterface>(interface, |interface| {
        interface.job_executions_changed_events.as_ref()(io_err);
    });
}

pub(super) fn create_subscribe_completed_jobs(tx: mpsc::Sender<bool>) -> impl Fn(i32) {
    move |io_err| {
        let _ = tx.try_send(io_err == 0);
    }
}

pub(super) fn create_publish_completed_jobs() -> impl Fn(*const c_void, i32) {
    move |callback, io_err| {
        let callback = unsafe { Box::from_raw(callback.cast::<Box<dyn FnOnce(i32)>>().cast_mut()) };
        callback(io_err);
    }
}

pub(super) fn create_get_pending_job_executions_accepted(
    tx: mpsc::Sender<JobsSummaryOwned>,
) -> impl Fn(i32, JobsSummaryOwned) {
    move |io_err, summary| {
        if io_err == 0 {
            let _ = tx.try_send(summary);
        }
    }
}

pub(super) fn create_get_pending_job_executions_rejected(
    tx: mpsc::Sender<RejectedOwned>,
) -> impl Fn(Rejected, i32) {
    move |rejected, _io_err| {
        let _ = tx.try_send(RejectedOwned::from(rejected));
    }
}

pub(super) fn create_start_next_pending_job_execution_accepted(
    tx: mpsc::Sender<JobDescription>,
) -> impl Fn(*const c_char, *const JobInfo, i32) {
    move |client_token, job_info, io_err| {
        if io_err == 0 {
            let _ = tx.try_send(JobDescription {
                client_token: to_option_str(client_token),
                data: from_job_info(job_info),
            });
        }
    }
}

pub(super) fn create_start_next_pending_job_execution_rejected(
    tx: mpsc::Sender<RejectedOwned>,
) -> impl Fn(Rejected, i32) {
    move |rejected, _io_err| {
        let _ = tx.try_send(RejectedOwned::from(rejected));
    }
}

pub(super) fn create_job_executions_changed_events() -> impl Fn(i32) {
    move |_io_err| {}
}

pub(super) fn create_next_job_execution_changed_events(
    tx: mpsc::Sender<JobDescription>,
) -> impl Fn(*const JobInfo, *const DateTime, i32) {
    move |job_info, _date_time, io_err| {
        if io_err == 0 {
            let _ = tx.try_send(JobDescription {
                client_token: None,
                data: from_job_info(job_info),
            });
        }
    }
}

pub(super) struct JobsClientInterface {
    pub(super) subscribe_completed_jobs: Box<dyn Fn(i32)>,
    pub(super) publish_completed_jobs: Box<dyn Fn(*const c_void, i32)>,
    pub(super) get_pending_job_executions_accepted: Box<dyn Fn(i32, JobsSummaryOwned)>,
    pub(super) get_pending_job_executions_rejected: Box<dyn Fn(Rejected, i32)>,
    pub(super) start_next_pending_job_execution_accepted:
        Box<dyn Fn(*const c_char, *const JobInfo, i32)>,
    pub(super) start_next_pending_job_execution_rejected: Box<dyn Fn(Rejected, i32)>,
    pub(super) job_executions_changed_events: Box<dyn Fn(i32)>,
    pub(super) next_job_execution_changed_events: Box<dyn Fn(*const JobInfo, *const DateTime, i32)>,
}

impl Debug for JobsClientInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobsClientInterface")
            .field(
                "subscribe_completed_jobs",
                &format!("{:?}", std::ptr::addr_of!(self.subscribe_completed_jobs)),
            )
            .field(
                "publish_completed_jobs",
                &format!("{:?}", std::ptr::addr_of!(self.publish_completed_jobs)),
            )
            .field(
                "get_pending_job_executions_accepted",
                &format!(
                    "{:?}",
                    std::ptr::addr_of!(self.get_pending_job_executions_accepted)
                ),
            )
            .field(
                "get_pending_job_executions_rejected",
                &format!(
                    "{:?}",
                    std::ptr::addr_of!(self.get_pending_job_executions_rejected)
                ),
            )
            .field(
                "start_next_pending_job_execution_accepted",
                &format!(
                    "{:?}",
                    std::ptr::addr_of!(self.start_next_pending_job_execution_accepted)
                ),
            )
            .field(
                "start_next_pending_job_execution_rejected",
                &format!(
                    "{:?}",
                    std::ptr::addr_of!(self.start_next_pending_job_execution_rejected)
                ),
            )
            .field(
                "job_executions_changed_events",
                &format!(
                    "{:?}",
                    std::ptr::addr_of!(self.job_executions_changed_events)
                ),
            )
            .field(
                "next_job_execution_changed_events",
                &format!(
                    "{:?}",
                    std::ptr::addr_of!(self.next_job_execution_changed_events)
                ),
            )
            .finish()
    }
}

unsafe impl Send for JobsClientInterface {}
unsafe impl Sync for JobsClientInterface {}
