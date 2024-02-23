use core::fmt::Debug;

use crate::{
    error::Error,
    types::{Metadata, OpenOptions},
    vafs_box, VAsyncFSBox,
};
use alloc::{boxed::Box, string::String, vec::Vec};
use async_trait::async_trait;
use futures_core::Stream;

use super::{file::VAsyncFile, path_ext::VAsyncPathExt};

pub trait VAsyncFS: Send + Sync + Sized {
    type Path: VAsyncPath<FS = Self>;
    fn path(&self, path: &str) -> Result<Self::Path, Error>;

    fn from_path(path: &Self::Path) -> Result<Self, Error>;
}

#[async_trait]
pub trait VAsyncFSExt: VAsyncFS {
    fn boxed(self) -> VAsyncFSBox
    where
        Self: Sized + 'static + Clone,
        Self::Path: Send + Sync + Clone,
        <Self::Path as VAsyncPath>::ReadDir: Send,
    {
        vafs_box(self)
    }

    async fn read(&self, path: &str) -> Result<Vec<u8>, Error>
    where
        <Self::Path as VAsyncPath>::File: Unpin,
    {
        self.path(path)?.read().await
    }

    async fn read_to_string(&self, path: &str) -> Result<String, Error>
    where
        <Self::Path as VAsyncPath>::File: Unpin,
    {
        self.path(path)?.read_to_string().await
    }
}

#[async_trait]
impl<T> VAsyncFSExt for T where T: VAsyncFS {}

#[async_trait]
pub trait VAsyncPath: Debug + Clone + Send + Sync {
    type FS: VAsyncFS<Path = Self>;
    type File: VAsyncFile;
    type ReadDir: Stream<Item = Result<Self, Error>>;

    fn file_name(&self) -> Option<&str>;

    /// The extension of this filename
    fn extension(&self) -> Option<&str>;

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<Self, Error>;

    /// Get the parent path
    fn parent(&self) -> Option<Self>;

    /// Check if the file existst
    async fn exists(&self) -> bool;

    /// Get the file's metadata
    async fn metadata(&self) -> Result<Metadata, Error>;

    fn to_string(&self) -> String;

    async fn open(&self, options: OpenOptions) -> Result<Self::File, Error>;
    async fn read_dir(&self) -> Result<Self::ReadDir, Error>;

    /// Create a directory at the location by this path
    async fn create_dir(&self) -> Result<(), Error>;
    /// Remove a file
    async fn rm(&self) -> Result<(), Error>;
    /// Remove a file or directory and all its contents
    async fn rm_all(&self) -> Result<(), Error>;

    #[cfg(feature = "std")]
    fn into_path_buf(&self) -> Option<std::path::PathBuf> {
        None
    }
}

pub trait VMetadata {
    fn is_dir(&self) -> bool;
    /// Returns true iff this path is a file
    fn is_file(&self) -> bool;
    /// Returns the length of the file at this path
    fn len(&self) -> u64;
}
