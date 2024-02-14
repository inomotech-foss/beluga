use std::ffi::{c_char, CStr};

use aws_crt_cpp_sys::root::Aws::Crt::DateTime;
pub use aws_iot_device_sdk_sys::root::Aws::Iotjobs::{JobStatus, RejectedErrorCode};
use itertools::Itertools;

use crate::common::Buffer;

#[repr(C)]
pub(super) struct JobInfo {
    pub(super) job_id: *const c_char,
    pub(super) job_document: Buffer,
    pub(super) status: *const JobStatus,
    pub(super) version_number: *const i32,
    pub(super) queue_at: *const DateTime,
    pub(super) thing_name: *const c_char,
    pub(super) execution_number: *const i64,
    pub(super) last_updated_at: *const DateTime,
    pub(super) started_at: *const DateTime,
}

#[derive(Debug, Clone)]
pub struct JobInfoOwned {
    pub job_id: Option<String>,
    pub job_document: Option<serde_json::Value>,
    pub status: Option<JobStatus>,
    pub version_number: Option<i32>,
    pub queue_at: Option<chrono::NaiveDateTime>,
    pub thing_name: Option<String>,
    pub execution_number: Option<i64>,
    pub last_updated_at: Option<chrono::NaiveDateTime>,
    pub started_at: Option<chrono::NaiveDateTime>,
}

impl From<JobInfo> for JobInfoOwned {
    fn from(value: JobInfo) -> Self {
        Self {
            job_id: unsafe {
                value.job_id.as_ref().map(|msg| {
                    CStr::from_ptr(msg as *const _)
                        .to_string_lossy()
                        .to_string()
                })
            },
            job_document: serde_json::from_slice(&Vec::<u8>::from(value.job_document)).ok(),
            status: unsafe { value.status.as_ref().cloned() },
            version_number: unsafe { value.version_number.as_ref().cloned() },
            queue_at: covert_date_time(value.queue_at),
            thing_name: unsafe {
                value.thing_name.as_ref().map(|msg| {
                    CStr::from_ptr(msg as *const _)
                        .to_string_lossy()
                        .to_string()
                })
            },
            execution_number: unsafe { value.execution_number.as_ref().cloned() },
            last_updated_at: covert_date_time(value.queue_at),
            started_at: covert_date_time(value.started_at),
        }
    }
}

#[repr(C)]
pub(super) struct Rejected {
    pub(super) timestamp: *const DateTime,
    pub(super) code: *const RejectedErrorCode,
    pub(super) message: *const c_char,
    pub(super) client_token: *const c_char,
}

#[derive(Debug, Clone)]
pub struct RejectedOwned {
    pub timestamp: Option<chrono::NaiveDateTime>,
    pub code: Option<RejectedErrorCode>,
    pub message: Option<String>,
    pub client_token: Option<String>,
}

impl From<Rejected> for RejectedOwned {
    fn from(value: Rejected) -> Self {
        let timestamp = covert_date_time(value.timestamp);
        let client_token = unsafe {
            value.client_token.as_ref().map(|token| {
                CStr::from_ptr(token as *const _)
                    .to_string_lossy()
                    .to_string()
            })
        };

        let message = unsafe {
            value.message.as_ref().map(|msg| {
                CStr::from_ptr(msg as *const _)
                    .to_string_lossy()
                    .to_string()
            })
        };

        let code = unsafe { value.code.as_ref().cloned() };

        Self {
            timestamp,
            code,
            message,
            client_token,
        }
    }
}

#[repr(C)]
pub(super) struct JobExecutionSummary {
    pub(super) job_id: *const c_char,
    pub(super) version_number: *const i32,
    pub(super) execution_number: *const i64,
    pub(super) started_at: *const DateTime,
    pub(super) queued_at: *const DateTime,
}

#[derive(Debug, Clone)]
pub struct JobExecutionSummaryOwned {
    pub job_id: Option<String>,
    pub version_number: Option<i32>,
    pub execution_number: Option<i64>,
    pub started_at: Option<chrono::NaiveDateTime>,
    pub queued_at: Option<chrono::NaiveDateTime>,
}

impl From<&JobExecutionSummary> for JobExecutionSummaryOwned {
    fn from(value: &JobExecutionSummary) -> Self {
        Self {
            job_id: unsafe {
                value.job_id.as_ref().map(|msg| {
                    CStr::from_ptr(msg as *const _)
                        .to_string_lossy()
                        .to_string()
                })
            },
            version_number: unsafe { value.version_number.as_ref().cloned() },
            execution_number: unsafe { value.execution_number.as_ref().cloned() },
            started_at: covert_date_time(value.started_at),
            queued_at: covert_date_time(value.queued_at),
        }
    }
}

#[repr(C)]
pub(super) struct JobsSummary {
    pub(super) queued_jobs: *const JobExecutionSummary,
    pub(super) progres_jobs: *const JobExecutionSummary,
    pub(super) queued_size: usize,
    pub(super) progres_size: usize,
}

#[derive(Debug, Clone)]
pub struct JobsSummaryOwned {
    pub queued_jobs: Vec<JobExecutionSummaryOwned>,
    pub progres_jobs: Vec<JobExecutionSummaryOwned>,
}

impl From<JobsSummary> for JobsSummaryOwned {
    fn from(
        JobsSummary {
            queued_jobs,
            progres_jobs,
            queued_size,
            progres_size,
        }: JobsSummary,
    ) -> Self {
        let queued_jobs = unsafe { std::slice::from_raw_parts(queued_jobs, queued_size) }
            .iter()
            .map(JobExecutionSummaryOwned::from)
            .collect_vec();

        let progres_jobs = unsafe { std::slice::from_raw_parts(progres_jobs, progres_size) }
            .iter()
            .map(JobExecutionSummaryOwned::from)
            .collect_vec();

        Self {
            queued_jobs,
            progres_jobs,
        }
    }
}

pub(super) fn covert_date_time(date_time: *const DateTime) -> Option<chrono::NaiveDateTime> {
    unsafe {
        date_time
            .as_ref()
            .and_then(|date_time| {
                chrono::NaiveDate::from_ymd_opt(
                    date_time.GetYear(false).into(),
                    date_time.GetMonth(false) as u32,
                    date_time.GetDay(false).into(),
                )
                .zip(chrono::NaiveTime::from_hms_opt(
                    date_time.GetHour(false).into(),
                    date_time.GetMinute(false).into(),
                    date_time.GetSecond(false).into(),
                ))
            })
            .map(|(date, time)| chrono::NaiveDateTime::new(date, time))
    }
}

#[repr(C)]
pub(super) struct NextPendingRequest {
    /// Specifies the amount of time this device has to finish execution of this
    /// job in minutes.
    pub(super) step_timeout: *const i64,
}

#[derive(Debug, Clone)]
pub struct JobDescription {
    pub client_token: Option<String>,
    pub data: Option<JobInfoOwned>,
}

#[derive(Debug, Clone)]
pub enum JobTag {
    Execution,
    NextChangedExecution,
    Pending,
    Update,
}
