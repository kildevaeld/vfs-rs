#![no_std]

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

#[cfg(all(feature = "std", not(feature = "alloc")))]
extern crate std;

#[cfg(any(feature = "alloc", feature = "std"))]
pub mod boxed;
mod error;
mod ext;
mod file;
mod fs;
mod metadata;
mod path;

pub use self::{error::*, ext::*, file::*, fs::*, metadata::*, path::*};

pub mod prelude {
    pub use super::{
        ext::{VFileExt, VPathExt},
        file::VFile,
        fs::VFS,
        path::VPath,
    };
}
