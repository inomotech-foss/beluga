//! aws-c-io is an event driven framework for implementing application
//! protocols.

#![no_std]

extern crate aws_c_cal_sys;
extern crate aws_c_common_sys;
#[cfg(all(unix, not(target_vendor = "apple")))]
extern crate s2n_tls_sys;

mod bindings {
    #![allow(
        clippy::all,
        clippy::wildcard_imports,
        non_camel_case_types,
        non_snake_case,
        non_upper_case_globals,
        rustdoc::all
    )]

    use aws_c_common_sys::*;
    use libc::*;

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;
