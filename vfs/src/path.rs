use std::path::PathBuf;

use alloc::string::String;

use crate::{
    error::Error,
    file::VFile,
    types::{Metadata, OpenOptions},
};

pub trait VPath: Sized {
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
}
