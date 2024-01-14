//! Core c99 package for AWS SDK for C.
//!
//! Includes cross-platform primitives, configuration, data structures, and
//! error handling.

#![no_std]
#![allow(
    clippy::all,
    clippy::wildcard_imports,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    rustdoc::all
)]

#[cfg(feature = "enable-tracing")]
extern crate ittapi_sys;

#[cfg(target_vendor = "apple")]
use core_foundation_sys::base::CFAllocatorRef;
use libc::*;

#[cfg(target_os = "android")]
#[link(name = "log")]
extern "C" {}

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// needs to be done manually because it's defined in an unnamed enum
pub const AWS_LOG_SUBJECT_STRIDE_BITS: u32 = 10;
