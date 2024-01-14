//! C99 implementation of the HTTP/1.1 and HTTP/2 specifications.

#![no_std]
#![allow(
    clippy::all,
    clippy::wildcard_imports,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    rustdoc::all
)]

extern crate aws_c_cal_sys;
extern crate aws_c_common_sys;
extern crate aws_c_compression_sys;
extern crate aws_c_io_sys;

use aws_c_common_sys::*;
use aws_c_io_sys::*;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
