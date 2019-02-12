use std::borrow::Cow;
use std::fmt::Debug;
use std::io::{Read, Result, Write};
use std::path::PathBuf;

pub trait VFS: Sync + Send {
    type Path: VPath;
    fn path(&self, path: &str) -> Self::Path;
}

pub trait VPath: Debug + Sized + Sync + Send + Clone {
    type Metadata: VMetadata;

    fn file_name(&self) -> Option<String>;

    /// The extension of this filename
    fn extension(&self) -> Option<String>;

    /// append a segment to this path
    fn resolve(&self, path: &String) -> Self;

    /// Get the parent path
    fn parent(&self) -> Option<Self>;

    /// Check if the file existst
    fn exists(&self) -> bool;

    /// Get the file's metadata
    fn metadata(&self) -> Result<Self::Metadata>;

    fn to_string(&self) -> Cow<str>;

    fn to_path_buf(&self) -> Option<PathBuf>;
}

pub trait ReadPath: VPath {
    type Read: Read + Send;
    type Iterator: Iterator<Item = Result<Self>>;

    fn open(&self) -> Result<Self::Read>;
    fn read_dir(&self) -> Result<Self::Iterator>;
}

pub trait WritePath: ReadPath {
    type Write: Write + Read + Send;

    fn create(&self) -> Result<Self::Write>;
    fn append(&self) -> Result<Self::Write>;
    /// Create a directory at the location by this path
    fn mkdir(&self) -> Result<()>;
    /// Remove a file
    fn rm(&self) -> Result<()>;
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> Result<()>;
}

pub trait VMetadata {
    fn is_dir(&self) -> bool;
    /// Returns true iff this path is a file
    fn is_file(&self) -> bool;
    /// Returns the length of the file at this path
    fn len(&self) -> u64;
}

#[derive(Debug, Default)]
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
    pub fn read(&mut self, read: bool) -> &mut OpenOptions {
        self.read = read;
        self
    }

    /// Open for writing
    pub fn write(&mut self, write: bool) -> &mut OpenOptions {
        self.write = write;
        self
    }

    /// Create the file if it does not exist yet
    pub fn create(&mut self, create: bool) -> &mut OpenOptions {
        self.create = create;
        self
    }

    /// Append at the end of the file
    pub fn append(&mut self, append: bool) -> &mut OpenOptions {
        self.append = append;
        self
    }

    /// Truncate the file to 0 bytes after opening
    pub fn truncate(&mut self, truncate: bool) -> &mut OpenOptions {
        self.truncate = truncate;
        self
    }
}
