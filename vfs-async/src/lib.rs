#[cfg(feature = "boxed")]
pub mod boxed;
#[cfg(feature = "memory")]
mod memory;
#[cfg(feature = "fs")]
mod physical;
mod traits;

#[cfg(feature = "memory")]
pub use memory::*;
#[cfg(feature = "fs")]
pub use physical::*;

pub use traits::*;

pub trait VFSExt: VFS {
    #[cfg(feature = "boxed")]
    fn boxed(self) -> Box<dyn boxed::BVFS>
    where
        Self: Sized + 'static,
        <Self::Path as VPath>::ReadDir: Send,
    {
        boxed::vfs_box(self)
    }
}

impl<T> VFSExt for T where T: VFS {}

pub trait VPathExt: VPath {
    #[cfg(feature = "boxed")]
    fn boxed(self) -> Box<dyn boxed::BVPath>
    where
        Self: Sized + 'static,
        <Self as VPath>::ReadDir: Send,
    {
        boxed::path_box(self)
    }
}

impl<T> VPathExt for T where T: VPath {}
