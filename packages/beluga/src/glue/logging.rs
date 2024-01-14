use core::ffi::{c_char, c_int, c_void};

use aws_c_common_sys::{
    aws_log_level, aws_log_subject_t, aws_logger, aws_logger_vtable, AWS_LL_DEBUG, AWS_LL_ERROR,
    AWS_LL_FATAL, AWS_LL_INFO, AWS_LL_TRACE, AWS_LL_WARN, AWS_OP_SUCCESS,
};

use crate::logging::Subject;
use crate::{AllocatorRef, AwsString};

pub fn create_logger(allocator: AllocatorRef) -> aws_logger {
    static VTABLE: aws_logger_vtable = create_vtable();
    aws_logger {
        vtable: core::ptr::addr_of!(VTABLE).cast_mut(),
        allocator: allocator.as_ptr(),
        p_impl: core::ptr::null_mut(),
    }
}

const fn create_vtable() -> aws_logger_vtable {
    extern "C" {
        // handled in C glue because of the printf formatting mechanism.
        fn acglu_log(
            logger: *mut aws_logger,
            log_level: aws_log_level,
            subject: aws_log_subject_t,
            format: *const c_char,
            ...
        ) -> c_int;
    }

    extern "C" fn get_log_level(
        _logger: *mut aws_logger,
        _subject: aws_log_subject_t,
    ) -> aws_log_level {
        let level_filter = log::STATIC_MAX_LEVEL.min(log::max_level());
        log_level_filter_to_aws(level_filter)
    }

    extern "C" fn set_log_level(_logger: *mut aws_logger, log_level: aws_log_level) -> c_int {
        let level = aws_level_to_log(log_level);
        log::set_max_level(level.to_level_filter());
        AWS_OP_SUCCESS as _
    }

    extern "C" fn clean_up(_logger: *mut aws_logger) {}

    aws_logger_vtable {
        log: Some(acglu_log),
        get_log_level: Some(get_log_level),
        set_log_level: Some(set_log_level),
        clean_up: Some(clean_up),
    }
}

#[no_mangle]
extern "C" fn arglu_log_enabled(
    _p_impl: *mut c_void,
    log_level: aws_log_level,
    subject: Subject,
) -> bool {
    let Some(target) = subject.static_target() else {
        // if we don't know this subject just treat it as enabled and let the logger
        // figure it out. global filters are already checked before.
        return true;
    };
    log::log_enabled!(target: target, aws_level_to_log(log_level))
}

#[no_mangle]
extern "C" fn arglu_log(
    _p_impl: *mut c_void,
    log_level: aws_log_level,
    subject: Subject,
    message: AwsString,
) -> c_int {
    let target = subject.target();
    log::log!(target: &target, aws_level_to_log(log_level), "{}", message);
    AWS_OP_SUCCESS as _
}

const fn aws_level_to_log(level: aws_log_level) -> log::Level {
    use log::Level::{Debug, Error, Info, Trace, Warn};
    match level {
        AWS_LL_FATAL | AWS_LL_ERROR => Error,
        AWS_LL_WARN => Warn,
        AWS_LL_INFO => Info,
        AWS_LL_DEBUG => Debug,
        // trace and anything else
        _ => Trace,
    }
}

const fn log_level_filter_to_aws(level: log::LevelFilter) -> aws_log_level {
    use log::LevelFilter::{Debug, Error, Info, Off, Trace, Warn};
    match level {
        Off => AWS_LL_FATAL,
        Error => AWS_LL_ERROR,
        Warn => AWS_LL_WARN,
        Info => AWS_LL_INFO,
        Debug => AWS_LL_DEBUG,
        Trace => AWS_LL_TRACE,
    }
}
