use async_trait::async_trait;
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};
use std::io::Result;

pub trait VFile: AsyncRead + AsyncSeek + AsyncWrite + Send {}

pub trait VFS: Send + Sync {
    type Path: VPath;
    fn path(&self, path: &str) -> Result<Self::Path>;
}

#[async_trait]
pub trait VPath: Clone + Send + Sync {
    type Metadata: VMetadata;
    type File: VFile;
    type ReadDir: Stream<Item = Result<Self>>;

    fn file_name(&self) -> Option<&str>;

    /// The extension of this filename
    fn extension(&self) -> Option<&str>;

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<Self>;

    /// Get the parent path
    fn parent(&self) -> Option<Self>;

    /// Check if the file existst
    async fn exists(&self) -> bool;

    /// Get the file's metadata
    async fn metadata(&self) -> Result<Self::Metadata>;

    fn to_string(&self) -> String;

    async fn open(&self, options: OpenOptions) -> Result<Self::File>;
    async fn read_dir(&self) -> Result<Self::ReadDir>;

    /// Create a directory at the location by this path
    async fn create_dir(&self) -> Result<()>;
    /// Remove a file
    async fn rm(&self) -> Result<()>;
    /// Remove a file or directory and all its contents
    async fn rm_all(&self) -> Result<()>;
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
    pub read: bool,
    pub write: bool,
    pub create: bool,
    pub append: bool,
    pub truncate: bool,
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
