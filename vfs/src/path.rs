use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    error::{Error, ErrorKind},
    file::VFile,
    types::{Metadata, OpenOptions},
    VFileExt, VFS,
};

pub trait VPath: Sized {
    type FS: VFS<Path = Self>;
    type File: VFile;
    type ReadDir: Iterator<Item = Result<Self, Error>>;

    fn file_name(&self) -> Option<&str>;

    /// The extension of this filename
    fn extension(&self) -> Option<&str>;

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<Self, Error>;

    /// Get the parent path
    fn parent(&self) -> Option<Self>;

    /// Check if the file existst
    fn exists(&self) -> bool;

    /// Get the file's metadata
    fn metadata(&self) -> Result<Metadata, Error>;

    fn to_string(&self) -> String;

    fn open(&self, options: OpenOptions) -> Result<Self::File, Error>;
    fn read_dir(&self) -> Result<Self::ReadDir, Error>;

    /// Create a directory at the location by this path
    fn create_dir(&self) -> Result<(), Error>;
    /// Remove a file
    fn rm(&self) -> Result<(), Error>;
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> Result<(), Error>;

    #[cfg(feature = "std")]
    fn into_path_buf(&self) -> Option<std::path::PathBuf> {
        None
    }
}

pub trait VPathExt: VPath {
    fn read(&self) -> Result<Vec<u8>, Error> {
        let mut file = self.open(OpenOptions::default().read(true))?;

        let mut buffer = Vec::default();
        file.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    fn read_to_string(&self) -> Result<String, Error> {
        let buffer = self.read()?;
        String::from_utf8(buffer).map_err(|err| Error::new(ErrorKind::InvalidData, err.to_string()))
    }
}

impl<V> VPathExt for V where V: VPath {}
