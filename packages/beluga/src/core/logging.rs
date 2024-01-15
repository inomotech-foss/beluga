use alloc::borrow::Cow;
use alloc::string::String;
use core::ffi::{c_int, c_void, CStr};
use core::mem::MaybeUninit;
use core::ops::RangeInclusive;

use aws_c_auth_sys::AWS_C_AUTH_PACKAGE_ID;
use aws_c_cal_sys::AWS_C_CAL_PACKAGE_ID;
use aws_c_common_sys::{
    aws_log_level, aws_log_subject_name, aws_log_subject_t, aws_logger, aws_logger_set,
    aws_logger_vtable, aws_string, AWS_C_COMMON_PACKAGE_ID, AWS_LL_DEBUG, AWS_LL_ERROR,
    AWS_LL_FATAL, AWS_LL_INFO, AWS_LL_TRACE, AWS_LL_WARN, AWS_LOG_SUBJECT_STRIDE_BITS,
    AWS_OP_SUCCESS,
};
use aws_c_compression_sys::AWS_C_COMPRESSION_PACKAGE_ID;
use aws_c_event_stream_sys::AWS_C_EVENT_STREAM_PACKAGE_ID;
use aws_c_http_sys::AWS_C_HTTP_PACKAGE_ID;
use aws_c_io_sys::AWS_C_IO_PACKAGE_ID;
use aws_c_iot_sys::AWS_C_IOTDEVICE_PACKAGE_ID;
use aws_c_mqtt_sys::AWS_C_MQTT_PACKAGE_ID;
use aws_c_s3_sys::AWS_C_S3_PACKAGE_ID;
use aws_c_sdkutils_sys::AWS_C_SDKUTILS_PACKAGE_ID;
use beluga_glue::logging::{beluga_logger, beluga_logging_log};

use crate::{AllocatorRef, AwsString};

mod subject_tree;

/// # Safety
///
/// Must only be called once.
pub unsafe fn init_unchecked_racy(allocator: AllocatorRef) {
    static mut LOGGER: MaybeUninit<aws_logger> = MaybeUninit::uninit();
    LOGGER.write(create_logger(allocator));
    unsafe { aws_logger_set(LOGGER.as_mut_ptr()) };
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Subject(aws_log_subject_t);

impl Subject {
    const STRIDE: u32 = 1 << AWS_LOG_SUBJECT_STRIDE_BITS;

    #[inline]
    pub const fn package_id(self) -> PackageId {
        PackageId(self.0 / Self::STRIDE)
    }

    #[inline]
    pub fn subject_name(self) -> &'static CStr {
        unsafe { CStr::from_ptr(aws_log_subject_name(self.0)) }
    }

    fn dynamic_target(self) -> String {
        alloc::format!("aws::{}", self.subject_name().to_bytes().escape_ascii())
    }

    pub fn target(self) -> Cow<'static, str> {
        self.static_target()
            .map_or_else(|| self.dynamic_target().into(), Cow::from)
    }
}

impl From<aws_log_subject_t> for Subject {
    fn from(value: aws_log_subject_t) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PackageId(u32);

impl PackageId {
    // 0
    pub const COMMON: Self = Self(AWS_C_COMMON_PACKAGE_ID);
    // 1
    pub const IO: Self = Self(AWS_C_IO_PACKAGE_ID);
    // 2
    pub const HTTP: Self = Self(AWS_C_HTTP_PACKAGE_ID);
    // 3
    pub const COMPRESSION: Self = Self(AWS_C_COMPRESSION_PACKAGE_ID);
    // 4
    pub const EVENT_STREAM: Self = Self(AWS_C_EVENT_STREAM_PACKAGE_ID);
    // 5
    pub const MQTT: Self = Self(AWS_C_MQTT_PACKAGE_ID);
    // 6
    pub const AUTH: Self = Self(AWS_C_AUTH_PACKAGE_ID);
    // 7
    pub const CAL: Self = Self(AWS_C_CAL_PACKAGE_ID);
    // 13
    pub const IOTDEVICE: Self = Self(AWS_C_IOTDEVICE_PACKAGE_ID);
    // 14
    pub const S3: Self = Self(AWS_C_S3_PACKAGE_ID);
    // 15
    pub const SDKUTILS: Self = Self(AWS_C_SDKUTILS_PACKAGE_ID);

    #[inline]
    pub const fn subject_range(self) -> RangeInclusive<Subject> {
        let begin = Subject(self.0 * Subject::STRIDE);
        let end = Subject((self.0 + 1) * Subject::STRIDE - 1);
        begin..=end
    }
}

fn create_logger(allocator: AllocatorRef) -> aws_logger {
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

    static VTABLE: aws_logger_vtable = aws_logger_vtable {
        log: Some(beluga_logging_log),
        get_log_level: Some(get_log_level),
        set_log_level: Some(set_log_level),
        clean_up: Some(clean_up),
    };

    extern "C" fn log_enabled(
        _p_impl: *mut c_void,
        log_level: aws_log_level,
        subject: aws_log_subject_t,
    ) -> bool {
        let subject = Subject(subject);
        let Some(target) = subject.static_target() else {
            // if we don't know this subject just treat it as enabled and let the logger
            // figure it out. global filters are already checked before.
            return true;
        };
        log::log_enabled!(target: target, aws_level_to_log(log_level))
    }

    unsafe extern "C" fn log(
        _p_impl: *mut c_void,
        log_level: aws_log_level,
        subject: aws_log_subject_t,
        message: *mut aws_string,
    ) -> c_int {
        let subject = Subject(subject);
        let message = AwsString::from_raw_unchecked(message);

        let target = subject.target();
        log::log!(target: &target, aws_level_to_log(log_level), "{}", message);
        AWS_OP_SUCCESS as _
    }

    static BELUGA_LOGGER: beluga_logger = beluga_logger {
        p_impl: core::ptr::null_mut(),
        log_enabled,
        log,
    };

    aws_logger {
        vtable: core::ptr::addr_of!(VTABLE).cast_mut(),
        allocator: allocator.as_ptr(),
        p_impl: core::ptr::addr_of!(BELUGA_LOGGER)
            .cast::<c_void>()
            .cast_mut(),
    }
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
