// #![no_std]

extern crate alloc;

pub use self::allocator::{Allocator, AllocatorRef};
pub use self::api::ApiHandle;
pub use self::error::{Error, Result};
pub use self::types::*;

#[macro_use]
mod macros;
mod allocator;
mod api;
mod error;
mod future;
mod glue;
pub mod io;
mod logging;
pub mod mqtt;
pub mod secure_tunnel;
mod types;
