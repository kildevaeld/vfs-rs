use futures_core::Stream;

use crate::{
    error::Error,
    file::{OpenOptions, VFile},
    fs::VFS,
    metadata::Metadata,
};

pub trait VPath: Sized {
    type FS: VFS<Path = Self>;
    type File: VFile;
    type ListDir: Stream<Item = Result<Self, Error>>;

    type Metadata: Future<Output = Result<Metadata, Error>>;
    type Open: Future<Output = Result<Self::File, Error>>;
    type CreateDir: Future<Output = Result<(), Error>>;
    type Remove: Future<Output = Result<(), Error>>;
    type ReadDir: Future<Output = Result<Self::ListDir, Error>>;

    fn file_name(&self) -> Option<&str>;

    /// The extension of this filename
    fn extension(&self) -> Option<&str>;

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<Self, Error>;

    /// Get the parent path
    fn parent(&self) -> Option<Self>;

    /// Get the file's metadata
    fn metadata(&self) -> Self::Metadata;

    fn open(&self, options: OpenOptions) -> Self::Open;
    fn read_dir(&self) -> Self::ReadDir;

    /// Create a directory at the location by this path
    fn create_dir(&self) -> Self::CreateDir;

    /// Remove a file or directory and all its contents
    fn rm(&self) -> Self::Remove;
}
