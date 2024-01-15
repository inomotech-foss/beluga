#![allow(clippy::wildcard_imports, non_camel_case_types, non_snake_case)]

use core::ffi::{c_char, c_int, c_void};

use aws_c_common_sys::*;

extern "C" {
    /// Log function for [aws_logger_vtable].
    ///
    /// The logger's p_impl must point to a [beluga_logger].
    pub fn beluga_logging_log(
        logger: *mut aws_logger,
        log_level: aws_log_level,
        subject: aws_log_subject_t,
        format: *const c_char,
        ...
    ) -> c_int;
}

// this struct needs to match the one defined in `logging.c`.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct beluga_logger {
    pub p_impl: *mut c_void,
    /// This function is called to determine whether a message should be logged
    /// before doing any allocation.
    pub log_enabled: unsafe extern "C" fn(
        p_impl: *mut c_void,
        log_level: aws_log_level,
        subject: aws_log_subject_t,
    ) -> bool,
    pub log: unsafe extern "C" fn(
        p_impl: *mut c_void,
        log_level: aws_log_level,
        subject: aws_log_subject_t,
        message: *mut aws_string,
    ) -> c_int,
}

unsafe impl Send for beluga_logger {}
unsafe impl Sync for beluga_logger {}
