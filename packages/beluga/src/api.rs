use aws_c_event_stream_sys::{aws_event_stream_library_clean_up, aws_event_stream_library_init};
use aws_c_mqtt_sys::{aws_mqtt_library_clean_up, aws_mqtt_library_init};
use aws_c_s3_sys::{aws_s3_library_clean_up, aws_s3_library_init};
use aws_c_sdkutils_sys::{aws_sdkutils_library_clean_up, aws_sdkutils_library_init};

use crate::allocator::AllocatorRef;
use crate::Allocator;

pub struct ApiHandle {
    _p: (),
}

impl ApiHandle {
    pub fn get() -> &'static Self {
        static INIT: std::sync::OnceLock<ApiHandle> = std::sync::OnceLock::new();
        INIT.get_or_init(|| unsafe { Self::new_unchecked_racy(Allocator::default()) })
    }

    /// # Safety
    ///
    /// Must only be called once.
    #[must_use]
    pub unsafe fn new_unchecked_racy(allocator: AllocatorRef) -> Self {
        crate::logging::init_unchecked_racy(allocator);
        aws_mqtt_library_init(allocator.as_ptr());
        aws_s3_library_init(allocator.as_ptr());
        aws_event_stream_library_init(allocator.as_ptr());
        aws_sdkutils_library_init(allocator.as_ptr());
        Self { _p: () }
    }
}

impl Drop for ApiHandle {
    fn drop(&mut self) {
        unsafe {
            aws_s3_library_clean_up();
            aws_mqtt_library_clean_up();
            aws_event_stream_library_clean_up();
            aws_sdkutils_library_clean_up();
        }
    }
}
