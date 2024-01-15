#![cfg_attr(not(any(test, feature = "std")), no_std)]

extern crate alloc;

pub use self::api::ApiHandle;
pub use self::core::*;

#[macro_use]
mod macros;
mod api;
mod core;
mod future;
pub mod io;
pub mod iot;
pub mod mqtt;
