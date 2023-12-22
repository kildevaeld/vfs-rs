use crate::{error::Error, vfs_box, VFSBox, VPath};

pub trait VFS: Send + Sync {
    type Path: VPath;
    fn path(&self, path: &str) -> Result<Self::Path, Error>;
}

pub trait VFSExt: VFS {
    fn boxed(self) -> VFSBox
    where
        Self: Sized + 'static + Clone,
        Self::Path: Send + Sync + Clone,
    {
        vfs_box(self)
    }
}

impl<T> VFSExt for T where T: VFS {}