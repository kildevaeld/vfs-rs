use alloc::{
    string::{String},
    vec::Vec,
};

use crate::{
    error::{Error},
    path::VPathExt,
    vfs_box, VFSBox, VPath,
};

pub trait VFS: Send + Sync + Sized {
    type Path: VPath<FS = Self>;
    fn path(&self, path: &str) -> Result<Self::Path, Error>;

    fn from_path(path: &Self::Path) -> Result<Self, Error>;
}

pub trait VFSExt: VFS {
    fn boxed(self) -> VFSBox
    where
        Self: Sized + 'static + Clone,
        Self::Path: Send + Sync + Clone,
    {
        vfs_box(self)
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, Error> {
        self.path(path)?.read()
    }

    fn read_to_string(&self, path: &str) -> Result<String, Error> {
        self.path(path)?.read_to_string()
    }
}

impl<T> VFSExt for T where T: VFS {}
