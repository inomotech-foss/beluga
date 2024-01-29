//! Core c99 package for AWS SDK for C.
//!
//! Includes cross-platform primitives, configuration, data structures, and
//! error handling.

#![no_std]

mod bindings {
    #![allow(
        clippy::all,
        clippy::wildcard_imports,
        non_camel_case_types,
        non_snake_case,
        non_upper_case_globals,
        rustdoc::all
    )]

    #[cfg(target_os = "macos")]
    use core_foundation::base::CFAllocatorRef;
    use libc::*;

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;
