mod traits;
// mod util;
// mod memory;
pub mod boxed;
#[cfg(feature = "glob")]
mod glob;
mod physical;
mod walk_dir;
mod walkdir;

#[cfg(feature = "glob")]
pub use glob::*;
pub use physical::*;
pub use traits::*;
pub use walk_dir::*;

pub trait VFSExt: VFS {
    fn boxed(self) -> Box<dyn boxed::BVFS>
    where
        Self: Sized + 'static,
    {
        boxed::vfs_box(self)
    }
}
