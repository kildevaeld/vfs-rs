use super::{OpenOptions, VFile, VMetadata, VPath, VFS};
use std::borrow::Cow;
use std::fmt;
use std::io::{self, Result};
use std::iter::Iterator;
use std::path::PathBuf;

pub trait GVFS: Send + Sync {
    fn path(&self, path: &str) -> Box<dyn GPath>;
}

pub trait GPath: std::marker::Send + std::marker::Sync {
    fn open_with_options(&self, open_options: &OpenOptions) -> Result<Box<dyn VFile>>;
    /// Open the file at this path for reading
    fn open(&self) -> Result<Box<dyn VFile>> {
        self.open_with_options(OpenOptions::new().read(true))
    }
    /// Open the file at this path for writing, truncating it if it exists already
    fn create(&self) -> Result<Box<dyn VFile>> {
        self.open_with_options(OpenOptions::new().write(true).create(true).truncate(true))
    }
    /// Open the file at this path for appending, creating it if necessary
    fn append(&self) -> Result<Box<dyn VFile>> {
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
    fn resolve(&self, path: &String) -> Box<GPath>;

    /// Get the parent path
    fn parent(&self) -> Option<Box<GPath>>;

    /// Check if the file existst
    fn exists(&self) -> bool;

    /// Get the file's metadata
    fn metadata(&self) -> Result<Box<dyn VMetadata>>;

    /// Retrieve the path entries in this path
    fn read_dir(&self) -> Result<Box<dyn Iterator<Item = Result<Box<dyn GPath>>>>>;

    /// Retrieve a string representation
    fn to_string(&self) -> Cow<str>;

    /// Retrieve a standard PathBuf, if available (usually only for PhysicalFS)
    fn to_path_buf(&self) -> Option<PathBuf>;

    fn box_clone(&self) -> Box<GPath>;
}

struct GFVSWrapper<V> {
    inner: V,
}

impl<V> GVFS for GFVSWrapper<V>
where
    V: VFS,
    <V as VFS>::Path: 'static,
{
    fn path(&self, path: &str) -> Box<dyn GPath> {
        Box::new(GPathWrapper {
            inner: self.inner.path(path),
        })
    }
}

struct GPathWrapper<P> {
    inner: P,
}

impl<P> fmt::Debug for GFVSWrapper<P>
where
    P: VPath + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner.to_string())
    }
}

impl<P> GPath for GPathWrapper<P>
where
    P: VPath + 'static,
{
    fn open_with_options(&self, open_options: &OpenOptions) -> Result<Box<dyn VFile>> {
        match self.inner.open_with_options(open_options) {
            Ok(m) => Ok(Box::new(m)),
            Err(e) => Err(e),
        }
    }
    /// Open the file at this path for reading
    fn open(&self) -> Result<Box<dyn VFile>> {
        self.open_with_options(OpenOptions::new().read(true))
    }
    /// Open the file at this path for writing, truncating it if it exists already
    fn create(&self) -> Result<Box<dyn VFile>> {
        self.open_with_options(OpenOptions::new().write(true).create(true).truncate(true))
    }
    /// Open the file at this path for appending, creating it if necessary
    fn append(&self) -> Result<Box<dyn VFile>> {
        self.open_with_options(OpenOptions::new().write(true).create(true).append(true))
    }
    /// Create a directory at the location by this path
    fn mkdir(&self) -> Result<()> {
        self.mkdir()
    }

    /// Remove a file
    fn rm(&self) -> Result<()> {
        self.inner.rm()
    }

    /// Remove a file or directory and all its contents
    fn rmrf(&self) -> Result<()> {
        self.inner.rmrf()
    }

    /// The file name of this path
    fn file_name(&self) -> Option<String> {
        self.inner.file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        self.inner.extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &String) -> Box<dyn GPath> {
        Box::new(GPathWrapper {
            inner: self.inner.resolve(path),
        })
    }

    /// Get the parent path
    fn parent(&self) -> Option<Box<GPath>> {
        match self.inner.parent() {
            Some(res) => Some(Box::new(GPathWrapper { inner: res })),
            None => None,
        }
    }

    /// Check if the file existst
    fn exists(&self) -> bool {
        self.inner.exists()
    }

    /// Get the file's metadata
    fn metadata(&self) -> Result<Box<dyn VMetadata>> {
        match self.inner.metadata() {
            Ok(res) => Ok(Box::new(res)),
            Err(e) => Err(e),
        }
    }

    /// Retrieve the path entries in this path
    fn read_dir(&self) -> Result<Box<dyn Iterator<Item = Result<Box<dyn GPath>>>>> {
        match self.inner.read_dir() {
            Ok(res) => Ok(Box::new(GPathIteratorWrapper::<P::Iterator, P>::new(res))),
            Err(e) => Err(e),
        }
    }

    /// Retrieve a string representation
    fn to_string(&self) -> Cow<str> {
        self.inner.to_string()
    }

    /// Retrieve a standard PathBuf, if available (usually only for PhysicalFS)
    fn to_path_buf(&self) -> Option<PathBuf> {
        self.inner.to_path_buf()
    }

    fn box_clone(&self) -> Box<GPath> {
        let path = Box::new(GPathWrapper {
            inner: self.inner.clone(),
        });
        path
    }
}

impl Clone for Box<GPath> {
    fn clone(&self) -> Box<GPath> {
        self.box_clone()
    }
}

struct GPathIteratorWrapper<Iter: Iterator<Item = Result<Item>>, Item> {
    inner: Iter,
}

impl<Iter: Iterator<Item = Result<Item>>, Item> GPathIteratorWrapper<Iter, Item> {
    pub fn new(iter: Iter) -> GPathIteratorWrapper<Iter, Item> {
        GPathIteratorWrapper { inner: iter }
    }
}

impl<Iter: Iterator<Item = Result<Item>>, Item> Iterator for GPathIteratorWrapper<Iter, Item>
where
    Item: VPath + 'static,
{
    type Item = Result<Box<GPath>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(m) => match m {
                Ok(m) => Some(Ok(Box::new(GPathWrapper { inner: m }))),
                Err(e) => Some(Err(e)),
            },
            None => None,
        }
    }
}

pub fn boxed_vfs<V: VFS + 'static>(vfs: V) -> Box<dyn GVFS> {
    Box::new(GFVSWrapper { inner: vfs })
}
