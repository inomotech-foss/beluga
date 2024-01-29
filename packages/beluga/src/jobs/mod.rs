use std::ffi::{c_char, c_void, CStr};

pub use common::{
    JobDescription, JobExecutionSummaryOwned, JobInfoOwned, JobStatus, JobTag, JobsSummaryOwned,
    RejectedErrorCode, RejectedOwned,
};
pub use job::{Description, Job};
pub use job_client::JobsClient;
use tracing::error;

use self::common::JobInfo;

mod common;
mod job;
mod job_callbacks;
mod job_client;
mod job_client_callbacks;

fn call<T>(interface: *const c_void, functor: impl FnOnce(&T) + std::panic::UnwindSafe) {
    let res = std::panic::catch_unwind(|| {
        // Safely calls `functor` on `interface` if it can be cast to `T`.
        // This allows passing a generic Rust closure into a C callback
        // that expects a concrete type without unsafe code.
        if let Some(interface) = unsafe { interface.cast::<T>().as_ref() } {
            functor(interface);
        }
    });

    if let Err(err) = res {
        error!(?err, "call to interface panicked");
    }
}

/// Converts a C string pointer to a Rust `String`, returning `None` if the
/// pointer is null.
///
/// This handles converting the raw C string pointer to a Rust string, handling
/// potential NULL pointer and encoding issues safely.
fn to_option_str(client_token: *const c_char) -> Option<String> {
    unsafe {
        client_token.as_ref().map(|msg| {
            CStr::from_ptr(msg as *const _)
                .to_string_lossy()
                .to_string()
        })
    }
}

/// Converts a C JobInfo pointer to a Rust JobInfoOwned, returning None if the
/// pointer is null.
///
/// This handles converting the raw C JobInfo pointer to a Rust JobInfoOwned,
/// handling potential NULL pointer and encoding issues safely.
fn from_job_info(job_info: *const JobInfo) -> Option<JobInfoOwned> {
    if !job_info.is_null() {
        JobInfoOwned::from(unsafe { job_info.read() }).into()
    } else {
        None
    }
}
