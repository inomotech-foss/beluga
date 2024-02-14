//! C++ wrapper around the aws-c-* libraries. Provides Cross-Platform Transport
//! Protocols and SSL/TLS implementations for C++.

#![no_std]

extern crate aws_c_auth_sys;
extern crate aws_c_cal_sys;
extern crate aws_c_common_sys;
extern crate aws_c_compression_sys;
extern crate aws_c_event_stream_sys;
extern crate aws_c_http_sys;
extern crate aws_c_io_sys;
extern crate aws_c_mqtt_sys;
extern crate aws_c_s3_sys;
extern crate aws_c_sdkutils_sys;
extern crate aws_checksums_sys;

mod bindings {
    #![allow(
        clippy::all,
        clippy::wildcard_imports,
        non_camel_case_types,
        non_snake_case,
        non_upper_case_globals,
        rustdoc::all
    )]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;
