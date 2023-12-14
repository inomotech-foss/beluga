use std::ffi::{c_char, CStr};

use tracing::{debug, error, info, trace, warn};

#[no_mangle]
extern "C" fn rust_info(file: *const c_char, name: *const c_char, line: i32, msg: *const c_char) {
    let (source, name) = fields(file, name);
    if !source.is_empty() && !name.is_empty() {
        info!(name: "", target: "aws_sdk", %source, %name, %line, "{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
    } else {
        info!(name: "", target: "aws_sdk", "{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
    }
}

#[no_mangle]
extern "C" fn rust_error(file: *const c_char, name: *const c_char, line: i32, msg: *const c_char) {
    let (source, name) = fields(file, name);
    if !source.is_empty() && !name.is_empty() {
        error!(name: "", target: "aws_sdk", %source, %name, %line, "{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
    } else {
        error!(name: "", target: "aws_sdk", "{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
    }
}

#[no_mangle]
extern "C" fn rust_debug(file: *const c_char, name: *const c_char, line: i32, msg: *const c_char) {
    let (source, name) = fields(file, name);
    if !source.is_empty() && !name.is_empty() {
        debug!(name: "", target: "aws_sdk", %source, %name, %line, "{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
    } else {
        debug!(name: "", target: "aws_sdk", "{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
    }
}

#[no_mangle]
extern "C" fn rust_warn(file: *const c_char, name: *const c_char, line: i32, msg: *const c_char) {
    let (source, name) = fields(file, name);
    if !source.is_empty() && !name.is_empty() {
        warn!(name: "", target: "aws_sdk", %source, %name, %line, "{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
    } else {
        warn!(name: "", target: "aws_sdk", "{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
    }
}

#[no_mangle]
extern "C" fn rust_trace(file: *const c_char, name: *const c_char, line: i32, msg: *const c_char) {
    let (source, name) = fields(file, name);
    if !source.is_empty() && !name.is_empty() {
        trace!(name: "", target: "aws_sdk", %source, %name, %line, "{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
    } else {
        trace!(name: "", target: "aws_sdk", "{}", unsafe { CStr::from_ptr(msg) }.to_string_lossy());
    }
}

#[inline]
#[must_use]
fn fields<'a>(file: *const c_char, name: *const c_char) -> (&'a str, &'a str) {
    (
        unsafe { CStr::from_ptr(file) }.to_str().unwrap_or_default(),
        unsafe { CStr::from_ptr(name) }.to_str().unwrap_or_default(),
    )
}
