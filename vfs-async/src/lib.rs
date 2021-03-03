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
    fn boxed(self) -> Box<dyn boxed::BVFS>
    where
        Self: Sized + 'static,
        <Self::Path as VPath>::ReadDir: Send,
    {
        boxed::vfs_box(self)
    }
}

impl<T> VFSExt for T where T: VFS {}
