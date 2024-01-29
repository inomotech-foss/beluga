//! Next generation AWS IoT Client SDK for C++ using the AWS Common Runtime

#![no_std]

extern crate aws_c_iot_sys;
extern crate aws_crt_cpp_sys;

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
