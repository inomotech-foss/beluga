//! aws-c-io is an event driven framework for implementing application
//! protocols.

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

use aws_c_common_sys::*;
use libc::*;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
