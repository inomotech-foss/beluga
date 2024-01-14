use core::ffi::{c_int, CStr};
use core::fmt::{Debug, Display};

use aws_c_common_sys::{
    aws_common_error, aws_error_lib_name, aws_error_name, aws_error_str, aws_last_error,
    aws_reset_error, AWS_ERROR_OVERFLOW_DETECTED, AWS_ERROR_SUCCESS, AWS_ERROR_UNKNOWN,
};

pub type Result<T> = core::result::Result<T, Error>;

#[must_use]
#[derive(Clone, Copy)]
pub struct Error(c_int);

impl Error {
    pub const UNKNOWN: Self = Self(AWS_ERROR_UNKNOWN as _);
    pub const OVERFLOW_DETECTED: Self = Self(AWS_ERROR_OVERFLOW_DETECTED as _);

    #[inline]
    pub fn last_in_current_thread() -> Self {
        let err = Self(unsafe { aws_last_error() });
        if err.is_success() {
            // we expect an error status but apparently not from aws
            Self::UNKNOWN
        } else {
            err
        }
    }

    #[inline]
    pub fn check_rc(rc: c_int) -> Result<()> {
        if rc == 0 {
            return Ok(());
        }
        Err(Self::last_in_current_thread())
    }

    #[inline]
    pub fn reset() {
        unsafe { aws_reset_error() };
    }

    #[inline]
    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self.0 as _, AWS_ERROR_SUCCESS)
    }

    #[inline]
    #[must_use]
    pub fn message(self) -> &'static CStr {
        unsafe { CStr::from_ptr(aws_error_str(self.0)) }
    }

    #[inline]
    #[must_use]
    pub fn name(self) -> &'static CStr {
        unsafe { CStr::from_ptr(aws_error_name(self.0)) }
    }

    #[inline]
    #[must_use]
    pub fn lib_name(self) -> &'static CStr {
        unsafe { CStr::from_ptr(aws_error_lib_name(self.0)) }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Error")
            .field("lib_name", &self.lib_name())
            .field("message", &self.message())
            .finish()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message().to_bytes().escape_ascii())
    }
}

impl std::error::Error for Error {}

impl From<aws_common_error> for Error {
    fn from(value: aws_common_error) -> Self {
        Self(value as _)
    }
}

#[cfg(test)]
mod tests {
    use aws_c_common_sys::{AWS_ERROR_INVALID_UTF8, AWS_ERROR_OOM};

    use super::*;
    use crate::ApiHandle;

    #[test]
    fn debug_impl() {
        // load error messages
        let _handle = ApiHandle::get();
        let err = Error::from(AWS_ERROR_OOM);
        assert_eq!(
            format!("{err:?}"),
            "Error { lib_name: \"aws-c-common\", message: \"Out of memory.\" }"
        );
    }

    #[test]
    fn display_impl() {
        // load error messages
        let _handle = ApiHandle::get();
        let err = Error::from(AWS_ERROR_INVALID_UTF8);
        assert_eq!(format!("{err}"), "Invalid UTF-8.");
    }
}
