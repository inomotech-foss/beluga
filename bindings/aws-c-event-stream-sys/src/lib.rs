//! C99 implementation of the vnd.amazon.event-stream content-type.

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
extern crate aws_c_io_sys;
extern crate aws_checksums_sys;

use aws_c_common_sys::*;
use aws_c_io_sys::*;
use libc::*;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
