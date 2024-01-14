//! AWS Crypto Abstraction Layer: Cross-Platform, C99 wrapper for cryptography
//! primitives.

#![no_std]
#![allow(
    clippy::all,
    clippy::wildcard_imports,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    rustdoc::all
)]

extern crate aws_c_common_sys;
#[cfg(all(unix, not(target_vendor = "apple")))]
extern crate aws_lc_sys;
#[cfg(target_vendor = "apple")]
extern crate security_framework_sys;
#[cfg(windows)]
extern crate windows_sys;

use aws_c_common_sys::*;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
