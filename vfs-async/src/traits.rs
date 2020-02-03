use std::io::{Result};
use futures_core::Stream;
use async_trait::async_trait;
use std::borrow::Cow;
use std::path::PathBuf;
use std::pin::Pin;
use std::future::Future;
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};

use futures_util::future::BoxFuture;

pub trait VFile: AsyncRead + AsyncSeek + AsyncWrite {}

pub trait VFS {
    type Path: VPath;
    fn path(&self, path: &str) -> Self::Path;
}

pub trait VPath: Clone {
    type Metadata: VMetadata;
    type File: VFile;
    type ReadDir: Stream<Item = Result<Self>>;

    fn file_name(&self) -> Option<String>;

    /// The extension of this filename
    fn extension(&self) -> Option<String>;

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Self;

    /// Get the parent path
    fn parent(&self) -> Option<Self>;

    /// Check if the file existst
    fn exists(&self) -> BoxFuture<'static, bool>;

    /// Get the file's metadata
    fn metadata(&self) -> BoxFuture<'static, Result<Self::Metadata>>;

    fn to_string(&self) -> Cow<str>;

    fn to_path_buf(&self) -> Option<PathBuf>;

    fn open(&self, options: OpenOptions) ->  BoxFuture<'static, Result<Self::File>>;
    fn read_dir(&self) ->  BoxFuture<'static, Result<Self::ReadDir>>;

    /// Create a directory at the location by this path
    fn mkdir(&self) ->  BoxFuture<'static, Result<()>>;
    /// Remove a file
    fn rm(&self) -> BoxFuture<'static, Result<()>>;
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> BoxFuture<'static, Result<()>>;

    fn create(&self) -> BoxFuture<'static, Result<Self::File>> {
        self.open(OpenOptions::new().write(true).create(true).truncate(true))
    }
    
    fn append(&self) -> BoxFuture<'static, Result<Self::File>> {
        self.open(OpenOptions::new().write(true).create(true).append(true))
    }

}

pub trait VMetadata {
    fn is_dir(&self) -> bool;
    /// Returns true iff this path is a file
    fn is_file(&self) -> bool;
    /// Returns the length of the file at this path
    fn len(&self) -> u64;
}


#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct OpenOptions {
    pub(crate) read: bool,
    pub(crate) write: bool,
    pub(crate) create: bool,
    pub(crate) append: bool,
    pub(crate) truncate: bool,
}

impl OpenOptions {
    /// Create a new instance
    pub fn new() -> OpenOptions {
        Default::default()
    }

    /// Open for reading
    pub fn read(mut self, read: bool) -> Self {
        self.read = read;
        self
    }

    /// Open for writing
    pub fn write(mut self, write: bool) -> Self {
        self.write = write;
        self
    }

    /// Create the file if it does not exist yet
    pub fn create(mut self, create: bool) -> Self {
        self.create = create;
        self
    }

    /// Append at the end of the file
    pub fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    /// Truncate the file to 0 bytes after opening
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }
}
