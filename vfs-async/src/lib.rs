mod traits;
// pub mod util;
pub mod boxed;
#[cfg(feature = "glob")]
mod glob;
mod memory;
mod physical;
mod walk_dir;
mod walkdir;

#[cfg(feature = "glob")]
pub use glob::*;
pub use memory::*;
pub use physical::*;
pub use traits::*;
pub use walk_dir::*;
pub use walkdir::*;

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
