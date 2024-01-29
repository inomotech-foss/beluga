use std::ffi::{c_char, c_void};
use std::fmt::Debug;

use tokio::sync::mpsc;

use super::common::{JobDescription, JobInfo, Rejected, RejectedOwned};
use super::{call, from_job_info, to_option_str};

#[no_mangle]
extern "C" fn on_describe_job_execution_accepted(
    interface: *const c_void,
    client_token: *const c_char,
    job_info: *const JobInfo,
    io_err: i32,
) {
    call::<JobInterface>(interface, |interface| {
        interface.describe_job_execution_accepted.as_ref()(client_token, job_info, io_err);
    });
}

#[no_mangle]
extern "C" fn on_describe_job_execution_rejected(
    interface: *const c_void,
    rejected: Rejected,
    io_err: i32,
) {
    call::<JobInterface>(interface, |interface| {
        interface.describe_job_execution_rejected.as_ref()(RejectedOwned::from(rejected), io_err);
    });
}

#[no_mangle]
extern "C" fn on_update_job_execution_accepted(
    interface: *const c_void,
    client_token: *const c_char,
    job_info: *const JobInfo,
    io_err: i32,
) {
    call::<JobInterface>(interface, |interface| {
        interface.update_job_execution_accepted.as_ref()(client_token, job_info, io_err);
    });
}

#[no_mangle]
extern "C" fn on_update_job_execution_rejected(
    interface: *const c_void,
    rejected: Rejected,
    io_err: i32,
) {
    call::<JobInterface>(interface, |interface| {
        interface.update_job_execution_rejected.as_ref()(RejectedOwned::from(rejected), io_err);
    });
}

#[no_mangle]
extern "C" fn on_subscribe_completed_job(interface: *const c_void, io_err: i32) {
    call::<JobInterface>(interface, |interface| {
        interface.subscribe_completed_job.as_ref()(io_err);
    });
}

#[no_mangle]
extern "C" fn on_publish_completed_job(
    interface: *const c_void,
    callback: *const c_void,
    io_err: i32,
) {
    call::<JobInterface>(interface, |interface| {
        interface.publish_completed_job.as_ref()(callback, io_err);
    });
}

pub(super) fn create_describe_job_execution_accepted(
    tx: mpsc::Sender<JobDescription>,
) -> impl Fn(*const c_char, *const JobInfo, i32) {
    move |token, info, io_err| {
        if io_err == 0 {
            let _ = tx.try_send(JobDescription {
                client_token: to_option_str(token),
                data: from_job_info(info),
            });
        }
    }
}

pub(super) fn create_describe_job_execution_rejected(
    tx: mpsc::Sender<RejectedOwned>,
) -> impl Fn(RejectedOwned, i32) {
    move |rejected, _io_err| {
        let _ = tx.try_send(rejected);
    }
}

pub(super) fn create_update_job_execution_accepted(
    tx: mpsc::Sender<JobDescription>,
) -> impl Fn(*const c_char, *const JobInfo, i32) {
    move |token, info, io_err| {
        if io_err == 0 {
            let _ = tx.try_send(JobDescription {
                client_token: to_option_str(token),
                data: from_job_info(info),
            });
        }
    }
}

pub(super) fn create_update_job_execution_rejected(
    tx: mpsc::Sender<RejectedOwned>,
) -> impl Fn(RejectedOwned, i32) {
    move |rejected, _io_err| {
        let _ = tx.try_send(rejected);
    }
}

pub(super) fn create_subscribe_completed_job(tx: mpsc::Sender<bool>) -> impl Fn(i32) {
    move |io_err| {
        let _ = tx.try_send(io_err == 0);
    }
}

pub(super) fn create_publish_completed_job() -> impl Fn(*const c_void, i32) {
    move |callback, io_err| {
        let callback = unsafe { Box::from_raw(callback.cast::<Box<dyn FnOnce(i32)>>().cast_mut()) };
        callback(io_err);
    }
}

pub(super) struct JobInterface {
    pub(super) describe_job_execution_accepted: Box<dyn Fn(*const c_char, *const JobInfo, i32)>,
    pub(super) describe_job_execution_rejected: Box<dyn Fn(RejectedOwned, i32)>,
    pub(super) update_job_execution_accepted: Box<dyn Fn(*const c_char, *const JobInfo, i32)>,
    pub(super) update_job_execution_rejected: Box<dyn Fn(RejectedOwned, i32)>,
    pub(super) subscribe_completed_job: Box<dyn Fn(i32)>,
    pub(super) publish_completed_job: Box<dyn Fn(*const c_void, i32)>,
}

impl Debug for JobInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobInterface")
            .field(
                "describe_job_execution_accepted",
                &format!(
                    "{:?}",
                    std::ptr::addr_of!(self.describe_job_execution_accepted)
                ),
            )
            .field(
                "describe_job_execution_rejected",
                &format!(
                    "{:?}",
                    std::ptr::addr_of!(self.describe_job_execution_rejected)
                ),
            )
            .field(
                "update_job_execution_accepted",
                &format!(
                    "{:?}",
                    std::ptr::addr_of!(self.update_job_execution_accepted)
                ),
            )
            .field(
                "update_job_execution_rejected",
                &format!(
                    "{:?}",
                    std::ptr::addr_of!(self.update_job_execution_rejected)
                ),
            )
            .field(
                "subscribe_completed_job",
                &format!("{:?}", std::ptr::addr_of!(self.subscribe_completed_job)),
            )
            .field(
                "publish_completed_job",
                &format!("{:?}", std::ptr::addr_of!(self.publish_completed_job)),
            )
            .finish()
    }
}

unsafe impl Send for JobInterface {}
unsafe impl Sync for JobInterface {}
