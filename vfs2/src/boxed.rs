use super::traits::*;
use std::fmt::Debug;
use std::io::{Read, Result, Write};
use std::path::PathBuf;

pub trait BVFS {
    fn path(&self, path: &str) -> Box<dyn BPath>;
}

pub trait BReadVFS: BVFS {
    fn read(&self, path: &str) -> Box<dyn BReadPath>;
}

pub trait BWriteVFS: BReadVFS {
    fn write(&self, path: &str) -> Box<dyn BWritePath>;
}

pub trait BPath: Debug + Send + Sync {
    fn file_name(&self) -> Option<String>;

    /// The extension of this filename
    fn extension(&self) -> Option<String>;

    /// append a segment to this path
    fn resolve(&self, path: &String) -> Box<dyn BPath>;

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BPath>>;

    /// Check if the file existst
    fn exists(&self) -> bool;

    /// Get the file's metadata
    fn metadata(&self) -> Result<Box<dyn VMetadata>>;

    /// Clone box
    fn box_clone(&self) -> Box<dyn BPath>;
}

pub trait BReadPath: Debug + Send + Sync {
    fn file_name(&self) -> Option<String>;

    /// The extension of this filename
    fn extension(&self) -> Option<String>;

    /// append a segment to this path
    fn resolve(&self, path: &String) -> Box<dyn BReadPath>;

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BReadPath>>;

    /// Check if the file existst
    fn exists(&self) -> bool;

    /// Get the file's metadata
    fn metadata(&self) -> Result<Box<dyn VMetadata>>;

    fn open(&self) -> Result<Box<dyn Read + Send>>;
    fn read_dir(&self) -> Result<Box<Iterator<Item = Result<Box<dyn BReadPath>>>>>;
    fn box_clone(&self) -> Box<dyn BReadPath>;
}

pub trait BFile: Read + Write {}

impl<T> BFile for T where T: Read + Write {}

pub trait BWritePath: Debug + Send + Sync {
    fn file_name(&self) -> Option<String>;

    /// The extension of this filename
    fn extension(&self) -> Option<String>;

    /// append a segment to this path
    fn resolve(&self, path: &String) -> Box<dyn BWritePath>;

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BWritePath>>;

    /// Check if the file existst
    fn exists(&self) -> bool;

    /// Get the file's metadata
    fn metadata(&self) -> Result<Box<dyn VMetadata>>;

    fn open(&self) -> Result<Box<dyn Read + Send>>;
    fn read_dir(&self) -> Result<Box<Iterator<Item = Result<Box<dyn BReadPath>>>>>;
    fn create(&self) -> Result<Box<dyn BFile>>;
    fn append(&self) -> Result<Box<dyn BFile>>;
    /// Create a directory at the location by this path
    fn mkdir(&self) -> Result<()>;
    /// Remove a file
    fn rm(&self) -> Result<()>;
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> Result<()>;
    fn box_clone(&self) -> Box<dyn BWritePath>;
}

#[derive(Debug)]
struct BPathWrapper<P> {
    inner: P,
}

impl<P> BPath for BPathWrapper<P>
where
    P: VPath + 'static,
{
    fn file_name(&self) -> Option<String> {
        self.inner.file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        self.inner.extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &String) -> Box<dyn BPath> {
        let ret = self.inner.resolve(path);
        Box::new(BPathWrapper { inner: ret })
    }

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BPath>> {
        match self.inner.parent() {
            Some(m) => Some(Box::new(BPathWrapper { inner: m })),
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
            Ok(m) => Ok(Box::new(m)),
            Err(e) => Err(e),
        }
    }

    fn box_clone(&self) -> Box<dyn BPath> {
        Box::new(BPathWrapper {
            inner: self.inner.clone(),
        })
    }
}

impl<P> BReadPath for BPathWrapper<P>
where
    P: ReadPath + 'static,
{
    fn file_name(&self) -> Option<String> {
        self.inner.file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        self.inner.extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &String) -> Box<dyn BReadPath> {
        let ret = self.inner.resolve(path);
        Box::new(BPathWrapper { inner: ret })
    }

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BReadPath>> {
        match self.inner.parent() {
            Some(m) => Some(Box::new(BPathWrapper { inner: m })),
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
            Ok(m) => Ok(Box::new(m)),
            Err(e) => Err(e),
        }
    }

    fn open(&self) -> Result<Box<dyn Read + Send>> {
        match self.inner.open() {
            Ok(m) => Ok(Box::new(m)),
            Err(e) => Err(e),
        }
    }

    fn read_dir(&self) -> Result<Box<Iterator<Item = Result<Box<dyn BReadPath>>>>> {
        match self.inner.read_dir() {
            Ok(m) => Ok(Box::new(BIterator::<P::Iterator, P>::new(m))),
            Err(e) => Err(e),
        }
    }

    fn box_clone(&self) -> Box<dyn BReadPath> {
        let path = Box::new(BPathWrapper {
            inner: self.inner.clone(),
        });
        path
    }
}

impl<P> BWritePath for BPathWrapper<P>
where
    P: WritePath + 'static,
{
    fn file_name(&self) -> Option<String> {
        self.inner.file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        self.inner.extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &String) -> Box<dyn BWritePath> {
        let ret = self.inner.resolve(path);
        Box::new(BPathWrapper { inner: ret })
    }

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BWritePath>> {
        match self.inner.parent() {
            Some(m) => Some(Box::new(BPathWrapper { inner: m })),
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
            Ok(m) => Ok(Box::new(m)),
            Err(e) => Err(e),
        }
    }

    fn open(&self) -> Result<Box<dyn Read + Send>> {
        match self.inner.open() {
            Ok(m) => Ok(Box::new(m)),
            Err(e) => Err(e),
        }
    }

    fn read_dir(&self) -> Result<Box<Iterator<Item = Result<Box<dyn BReadPath>>>>> {
        match self.inner.read_dir() {
            Ok(m) => Ok(Box::new(BIterator::<P::Iterator, P>::new(m))),
            Err(e) => Err(e),
        }
    }

    fn create(&self) -> Result<Box<dyn BFile>> {
        match self.inner.create() {
            Ok(m) => Ok(Box::new(m)),
            Err(e) => Err(e),
        }
    }
    fn append(&self) -> Result<Box<dyn BFile>> {
        match self.inner.append() {
            Ok(m) => Ok(Box::new(m)),
            Err(e) => Err(e),
        }
    }
    /// Create a directory at the location by this path
    fn mkdir(&self) -> Result<()> {
        self.inner.mkdir()
    }
    /// Remove a file
    fn rm(&self) -> Result<()> {
        self.inner.rm()
    }
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> Result<()> {
        self.inner.rm_all()
    }

    fn box_clone(&self) -> Box<dyn BWritePath> {
        let path = Box::new(BPathWrapper {
            inner: self.inner.clone(),
        });
        path
    }

    // fn box_read_clone(&self) -> Box<dyn BReadPath> {
    //     let path = Box::new(BPathWrapper {
    //         inner: self.inner.clone(),
    //     });
    //     path
    // }
}

impl Clone for Box<dyn BReadPath> {
    fn clone(&self) -> Box<dyn BReadPath> {
        self.box_clone()
    }
}

impl Clone for Box<dyn BWritePath> {
    fn clone(&self) -> Box<dyn BWritePath> {
        self.box_clone()
    }
}

impl Clone for Box<dyn BPath> {
    fn clone(&self) -> Box<dyn BPath> {
        self.box_clone()
    }
}

struct BIterator<Iter, I> {
    inner: Iter,
    _i: std::marker::PhantomData<I>,
}

impl<Iter, I> BIterator<Iter, I> {
    pub fn new(inner: Iter) -> BIterator<Iter, I> {
        BIterator {
            inner,
            _i: std::marker::PhantomData,
        }
    }
}

impl<Iter, I> Iterator for BIterator<Iter, I>
where
    Iter: Iterator<Item = Result<I>>,
    I: ReadPath + 'static,
{
    type Item = Result<Box<dyn BReadPath>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(m) => match m {
                Ok(m) => Some(Ok(Box::new(BPathWrapper { inner: m }))),
                Err(e) => Some(Err(e)),
            },
            None => None,
        }
    }
}

struct BVFSWrapper<V> {
    inner: V,
}

impl<V> BVFS for BVFSWrapper<V>
where
    V: VFS,
    <V as VFS>::Path: 'static,
{
    fn path(&self, path: &str) -> Box<dyn BPath> {
        return Box::new(BPathWrapper {
            inner: self.inner.path(path),
        });
    }
}

impl<V> BReadVFS for BVFSWrapper<V>
where
    V: VFS,
    <V as VFS>::Path: ReadPath + 'static,
{
    fn read(&self, path: &str) -> Box<dyn BReadPath> {
        return Box::new(BPathWrapper {
            inner: self.inner.path(path),
        });
    }
}

impl<V> BWriteVFS for BVFSWrapper<V>
where
    V: VFS,
    <V as VFS>::Path: WritePath + 'static,
{
    fn write(&self, path: &str) -> Box<dyn BWritePath> {
        return Box::new(BPathWrapper {
            inner: self.inner.path(path),
        });
    }
}

pub fn read_box<V: VFS + 'static>(v: V) -> Box<dyn BReadVFS>
where
    <V as VFS>::Path: ReadPath,
{
    Box::new(BVFSWrapper { inner: v })
}

pub fn write_box<V: VFS + 'static>(v: V) -> Box<dyn BWriteVFS>
where
    <V as VFS>::Path: WritePath,
{
    Box::new(BVFSWrapper { inner: v })
}

// VPath support
impl VPath for Box<dyn BPath> {
    type Metadata = Box<dyn VMetadata + 'static>;

    fn file_name(&self) -> Option<String> {
        self.as_ref().file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        self.as_ref().extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &String) -> Box<dyn BPath> {
        self.as_ref().resolve(path)
    }

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BPath>> {
        self.as_ref().parent()
    }

    /// Check if the file existst
    fn exists(&self) -> bool {
        self.as_ref().exists()
    }

    /// Get the file's metadata
    fn metadata(&self) -> Result<Self::Metadata> {
        self.as_ref().metadata()
    }

    fn to_string(&self) -> std::borrow::Cow<str> {
        std::borrow::Cow::default()
    }

    fn to_path_buf(&self) -> Option<PathBuf> {
        None
    }
}

impl VPath for Box<dyn BReadPath> {
    type Metadata = Box<dyn VMetadata + 'static>;

    fn file_name(&self) -> Option<String> {
        self.as_ref().file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        self.as_ref().extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &String) -> Box<dyn BReadPath> {
        self.as_ref().resolve(path)
    }

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BReadPath>> {
        self.as_ref().parent()
    }

    /// Check if the file existst
    fn exists(&self) -> bool {
        self.as_ref().exists()
    }

    /// Get the file's metadata
    fn metadata(&self) -> Result<Self::Metadata> {
        self.as_ref().metadata()
    }

    fn to_string(&self) -> std::borrow::Cow<str> {
        std::borrow::Cow::default()
    }

    fn to_path_buf(&self) -> Option<PathBuf> {
        None
    }
}

impl ReadPath for Box<dyn BReadPath> {
    type Read = Box<dyn Read + Send>;
    type Iterator = Box<dyn Iterator<Item = Result<Box<dyn BReadPath + 'static>>>>;

    fn open(&self) -> Result<Self::Read> {
        self.as_ref().open()
    }

    fn read_dir(&self) -> Result<Self::Iterator> {
        self.as_ref().read_dir()
    }
}

impl VMetadata for Box<dyn VMetadata> {
    fn is_dir(&self) -> bool {
        self.as_ref().is_dir()
    }
    fn is_file(&self) -> bool {
        self.as_ref().is_file()
    }
    fn len(&self) -> u64 {
        self.as_ref().len()
    }
}

#[cfg(test)]
mod tests {

    use super::super::memory::*;
    use super::*;

    #[test]
    fn it_works() {
        let m = MemoryFS::new();
        let b = write_box(m);
        let mut f = b.write("/test.txt").create().unwrap();
        f.write(b"Hello, World!");
        f.flush();
    }
}
