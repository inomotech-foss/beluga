use alloc::borrow::Cow;
use alloc::string::String;
use core::ffi::CStr;
use core::mem::MaybeUninit;
use core::ops::RangeInclusive;

use aws_c_auth_sys::AWS_C_AUTH_PACKAGE_ID;
use aws_c_cal_sys::AWS_C_CAL_PACKAGE_ID;
use aws_c_common_sys::{
    aws_log_subject_name, aws_log_subject_t, aws_logger, aws_logger_set, AWS_C_COMMON_PACKAGE_ID,
    AWS_LOG_SUBJECT_STRIDE_BITS,
};
use aws_c_compression_sys::AWS_C_COMPRESSION_PACKAGE_ID;
use aws_c_event_stream_sys::AWS_C_EVENT_STREAM_PACKAGE_ID;
use aws_c_http_sys::AWS_C_HTTP_PACKAGE_ID;
use aws_c_io_sys::AWS_C_IO_PACKAGE_ID;
use aws_c_iot_sys::AWS_C_IOTDEVICE_PACKAGE_ID;
use aws_c_mqtt_sys::AWS_C_MQTT_PACKAGE_ID;
use aws_c_s3_sys::AWS_C_S3_PACKAGE_ID;
use aws_c_sdkutils_sys::AWS_C_SDKUTILS_PACKAGE_ID;

use crate::AllocatorRef;

mod subject_tree;

/// # Safety
///
/// Must only be called once.
pub unsafe fn init_unchecked_racy(allocator: AllocatorRef) {
    static mut LOGGER: MaybeUninit<aws_logger> = MaybeUninit::uninit();
    LOGGER.write(crate::glue::logging::create_logger(allocator));
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
