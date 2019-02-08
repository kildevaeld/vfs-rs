use std::borrow::Cow;
use std::convert::AsRef;
use std::fmt::Debug;
use std::io::{Read, Result, Seek, Write};
use std::path::{Path, PathBuf};

pub mod boxed;
pub mod physical;
pub mod utils;
/// A abstract path to a location in a filesystem
pub trait VPath: Debug + std::marker::Send + std::marker::Sync + Sized + Clone {
    type File: VFile;
    type Metadata: VMetadata;
    type Iterator: Iterator<Item = Result<Self>>;

    /// Open the file at this path with the given options
    fn open_with_options(&self, open_options: &OpenOptions) -> Result<Self::File>;
    /// Open the file at this path for reading
    fn open(&self) -> Result<Self::File> {
        self.open_with_options(OpenOptions::new().read(true))
    }
    /// Open the file at this path for writing, truncating it if it exists already
    fn create(&self) -> Result<Self::File> {
        self.open_with_options(OpenOptions::new().write(true).create(true).truncate(true))
    }
    /// Open the file at this path for appending, creating it if necessary
    fn append(&self) -> Result<Self::File> {
        self.open_with_options(OpenOptions::new().write(true).create(true).append(true))
    }
    /// Create a directory at the location by this path
    fn mkdir(&self) -> Result<()>;

    /// Remove a file
    fn rm(&self) -> Result<()>;

    /// Remove a file or directory and all its contents
    fn rmrf(&self) -> Result<()>;

    /// The file name of this path
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

    /// Retrieve the path entries in this path
    fn read_dir(&self) -> Result<Self::Iterator>;

    /// Retrieve a string representation
    fn to_string(&self) -> Cow<str>;

    /// Retrieve a standard PathBuf, if available (usually only for PhysicalFS)
    fn to_path_buf(&self) -> Option<PathBuf>;

    // fn box_clone(&self) -> Box<Self>;
}

// /// Resolve the path relative to the given base returning a new path
// pub fn resolve<S: Into<String>>(base: &VPath, path: S) -> Box<VPath> {
//     base.resolve(&path.into())
// }

/// An abstract file object
pub trait VFile: Read + Write + Seek + Debug {}

impl<T> VFile for T where T: Read + Write + Seek + Debug {}

/// File metadata abstraction
pub trait VMetadata {
    /// Returns true iff this path is a directory
    fn is_dir(&self) -> bool;
    /// Returns true iff this path is a file
    fn is_file(&self) -> bool;
    /// Returns the length of the file at this path
    fn len(&self) -> u64;
}

/// An abstract virtual file system
pub trait VFS: Send + Sync {
    /// The type of file objects
    type Path: VPath;
    // /// The type of path objects
    // type File: VFile;
    // /// The type of metadata objects
    // type Metadata: VMetadata;

    /// Create a new path within this filesystem
    fn path(&self, path: &str) -> Self::Path;
}

/// Options for opening files
#[derive(Debug, Default)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    create: bool,
    append: bool,
    truncate: bool,
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let p = super::physical::PhysicalFS::new(".").unwrap();
        let b = super::boxed::boxed_vfs(p);
    }
}
