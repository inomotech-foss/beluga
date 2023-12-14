use std::ffi::{c_char, CStr};

use tracing::{debug, error, info};

#[no_mangle]
extern "C" fn info(msg: *const c_char) {
    info!("{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
}

#[no_mangle]
extern "C" fn debug(msg: *const c_char) {
    debug!("{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
}

#[no_mangle]
extern "C" fn error(msg: *const c_char) {
    error!("{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
}
