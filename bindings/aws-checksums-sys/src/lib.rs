//! Cross-Platform HW accelerated CRC32c and CRC32 with fallback to efficient SW
//! implementations.

#![no_std]

extern crate aws_c_common_sys;

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
