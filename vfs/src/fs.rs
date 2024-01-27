use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    error::{Error, ErrorKind},
    vfs_box, OpenOptions, VFSBox, VFileExt, VPath,
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
        let path = self.path(path)?;

        let mut file = path.open(OpenOptions::default().read(true))?;

        let mut buffer = Vec::default();
        file.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    fn read_to_string(&self, path: &str) -> Result<String, Error> {
        let buffer = self.read(path)?;
        String::from_utf8(buffer).map_err(|err| Error::new(ErrorKind::InvalidData, err.to_string()))
    }
}

impl<T> VFSExt for T where T: VFS {}
