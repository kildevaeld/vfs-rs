#![no_std]

extern crate alloc;

mod boxed;
mod error;
mod file;
mod fs;
mod path;
mod types;

#[cfg(feature = "async")]
pub mod r#async;

pub use self::{
    boxed::*,
    file::{VFile, VFileExt},
    fs::*,
    path::VPath,
    types::*,
};

#[cfg(feature = "async")]
pub use r#async::*;
