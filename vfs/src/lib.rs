#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod boxed;
pub mod error;
mod file;
mod fs;
mod path;
mod types;

#[cfg(feature = "async")]
pub mod r#async;

pub use self::{
    boxed::*,
    error::Error,
    error::Result,
    file::{VFile, VFileExt},
    fs::*,
    path::VPath,
    types::*,
};

#[cfg(feature = "async")]
pub use r#async::*;

#[cfg(feature = "async")]
pub use async_trait::async_trait;
