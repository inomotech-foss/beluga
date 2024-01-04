use std::ffi::{c_char, CStr};

use aws_c_common_sys::{
    aws_log_level, AWS_LL_DEBUG, AWS_LL_ERROR, AWS_LL_FATAL, AWS_LL_INFO, AWS_LL_WARN,
};

/// # Safety
///
/// All char pointers must be valid nul-terminated strings.
#[no_mangle]
unsafe extern "C" fn rust_log(
    level: aws_log_level,
    file: *const c_char,
    target: *const c_char,
    line: u32,
    msg: *const c_char,
) {
    let level = match level {
        AWS_LL_DEBUG => log::Level::Debug,
        AWS_LL_INFO => log::Level::Info,
        AWS_LL_WARN => log::Level::Warn,
        AWS_LL_ERROR | AWS_LL_FATAL => log::Level::Error,
        _ => log::Level::Trace,
    };

    if level > log::STATIC_MAX_LEVEL || level > log::max_level() {
        // log level not enabled
        return;
    }

    let logger = log::logger();

    let name = unsafe { CStr::from_ptr(target).to_string_lossy() };
    let metadata = log::Metadata::builder().level(level).target(&name).build();
    if !logger.enabled(&metadata) {
        return;
    }

    let file = (!file.is_null()).then(|| unsafe { CStr::from_ptr(file).to_string_lossy() });
    let msg = unsafe { CStr::from_ptr(msg).to_string_lossy() };
    logger.log(
        &log::Record::builder()
            .metadata(metadata)
            .file(file.as_deref())
            .line(file.as_ref().map(|_| line))
            .args(format_args!("{msg}"))
            .build(),
    );
}
