use super::traits::*;
use std::borrow::Cow;
use std::fmt::Debug;
use std::io::{Read, Result, Write};
use std::path::PathBuf;

pub trait BVFS: Sync + Send + Debug {
    fn path(&self, path: &str) -> Box<dyn BPath>;
}



pub trait BFile: Read + Write + Send {}

impl<T> BFile for T where T: Read + Write + Send {}

pub trait BPath: Debug + Send + Sync {
    fn file_name(&self) -> Option<String>;

    /// The extension of this filename
    fn extension(&self) -> Option<String>;

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Box<dyn BPath>;

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BPath>>;

    /// Check if the file existst
    fn exists(&self) -> bool;

    /// Get the file's metadata
    fn metadata(&self) -> Result<Box<dyn VMetadata>>;

    fn open(&self, options: OpenOptions) -> Result<Box<dyn BFile>>;
    fn read_dir(&self) -> Result<Box<dyn Iterator<Item = Result<Box<dyn BPath>>>>>;

    // fn create(&self) -> Result<Box<dyn BFile>>;
    // fn append(&self) -> Result<Box<dyn BFile>>;
    /// Create a directory at the location by this path
    fn mkdir(&self) -> Result<()>;
    /// Remove a file
    fn rm(&self) -> Result<()>;
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> Result<()>;
    fn box_clone(&self) -> Box<dyn BPath>;
    fn to_string(&self) -> Cow<str>;
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
    fn resolve(&self, path: &str) -> Box<dyn BPath> {
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

    fn open(&self, options: OpenOptions) -> Result<Box<dyn BFile>> {
        match self.inner.open(options) {
            Ok(m) => Ok(Box::new(m)),
            Err(e) => Err(e),
        }
    }

    fn read_dir(&self) -> Result<Box<dyn Iterator<Item = Result<Box<dyn BPath>>>>> {
        match self.inner.read_dir() {
            Ok(m) => Ok(Box::new(BPathIterator::<P::Iterator, P>::new(m))),
            Err(e) => Err(e),
        }
    }

    // fn create(&self) -> Result<Box<dyn BFile>> {
    //     match self.inner.create() {
    //         Ok(m) => Ok(Box::new(m)),
    //         Err(e) => Err(e),
    //     }
    // }
    // fn append(&self) -> Result<Box<dyn BFile>> {
    //     match self.inner.append() {
    //         Ok(m) => Ok(Box::new(m)),
    //         Err(e) => Err(e),
    //     }
    // }
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

    fn box_clone(&self) -> Box<dyn BPath> {
        let path = Box::new(BPathWrapper {
            inner: self.inner.clone(),
        });
        path
    }

    fn to_string(&self) -> Cow<str> {
        self.inner.to_string()
    }
}

impl Clone for Box<dyn BPath> {
    fn clone(&self) -> Box<dyn BPath> {
        self.box_clone()
    }
}



struct BPathIterator<Iter, I> {
    inner: Iter,
    _i: std::marker::PhantomData<I>,
}

impl<Iter, I> BPathIterator<Iter, I> {
    pub fn new(inner: Iter) -> BPathIterator<Iter, I> {
        BPathIterator {
            inner,
            _i: std::marker::PhantomData,
        }
    }
}

impl<Iter, I> Iterator for BPathIterator<Iter, I>
where
    Iter: Iterator<Item = Result<I>>,
    I: VPath + 'static,
{
    type Item = Result<Box<dyn BPath>>;
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

#[derive(Debug)]
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



pub fn vfs_box<V: VFS + 'static>(v: V) -> Box<dyn BVFS>

{
    Box::new(BVFSWrapper { inner: v })
}




impl VPath for Box<dyn BPath> {
    type Metadata = Box<dyn VMetadata + 'static>;
    type File = Box<dyn BFile>;
    type Iterator = Box<dyn Iterator<Item = Result<Box<dyn BPath + 'static>>>>;

    fn file_name(&self) -> Option<String> {
        self.as_ref().file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        self.as_ref().extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Box<dyn BPath> {
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
        self.as_ref().to_string()
    }

    fn to_path_buf(&self) -> Option<PathBuf> {
        None
    }

     fn open(&self, options: OpenOptions) -> Result<Self::File> {
        self.as_ref().open(options)
    }

    fn read_dir(&self) -> Result<Self::Iterator> {
        self.as_ref().read_dir()
    }

    // fn create(&self) -> Result<Self::File> {
    //     self.as_ref().create()
    // }
    // fn append(&self) -> Result<Self::File> {
    //     self.as_ref().append()
    // }
    /// Create a directory at the location by this path
    fn mkdir(&self) -> Result<()> {
        self.as_ref().mkdir()
    }
    /// Remove a file
    fn rm(&self) -> Result<()> {
        self.as_ref().rm()
    }
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> Result<()> {
        self.as_ref().rm_all()
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

impl VFile for Box<dyn BFile> {}

#[cfg(test)]
mod tests {

    use super::super::memory::*;
    use super::*;

    #[test]
    fn it_works() {
        let m = MemoryFS::new();
        let b = vfs_box(m);
        let mut f = b.path("/test.txt").create().unwrap();
        f.write(b"Hello, World!");
        f.flush();
    }
}
