#[cfg(feature = "boxed")]
pub mod boxed;
mod memory;
mod traits;
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
