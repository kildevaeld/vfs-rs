use super::{file::VFile, path::VPath};
use crate::{error::Error, Metadata, OpenOptions, SeekFrom, VMetadata, VFS};
use alloc::{boxed::Box, string::String};
use async_trait::async_trait;
use pin_project_lite::pin_project;

pub trait BVFS: Sync + Send {
    fn path(&self, path: &str) -> Result<VPathBox, Error>;
    fn box_clone(&self) -> VFSBox;
}

pub type VFSBox = Box<dyn BVFS>;

pub type VPathBox = Box<dyn BVPath>;

pub type VFileBox = Box<dyn VFile>;

pub trait BVPath: Send + Sync {
    fn file_name(&self) -> Option<&str>;

    /// The extension of this filename
    fn extension(&self) -> Option<&str>;

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<VPathBox, Error>;

    /// Get the parent path
    fn parent(&self) -> Option<VPathBox>;

    /// Check if the file existst
    fn exists(&self) -> bool;

    /// Get the file's metadata
    fn metadata(&self) -> Result<Metadata, Error>;

    fn to_string(&self) -> String;

    // fn to_path_buf(&self) -> Option<PathBuf>;

    fn open(&self, options: OpenOptions) -> Result<VFileBox, Error>;

    fn read_dir(&self) -> Result<Box<dyn Iterator<Item = Result<VPathBox, Error>>>, Error>;

    /// Create a directory at the location by this path
    fn create_dir(&self) -> Result<(), Error>;
    /// Remove a file
    fn rm(&self) -> Result<(), Error>;
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> Result<(), Error>;

    fn box_clone(&self) -> VPathBox;
}

struct BVFSBox<V>(V);

impl<V: VFS> BVFS for BVFSBox<V>
where
    V: Send + Clone + 'static,
    V::Path: 'static + Send + Sync + Clone,
{
    fn path(&self, path: &str) -> Result<VPathBox, Error> {
        Ok(Box::new(BVPathBox(self.0.path(path)?)))
    }

    fn box_clone(&self) -> VFSBox {
        Box::new(BVFSBox(self.0.clone()))
    }
}

struct BVPathBox<P>(P);

#[async_trait]
impl<P: Clone> BVPath for BVPathBox<P>
where
    P: Send + Sync + VPath + 'static,
{
    fn file_name(&self) -> Option<&str> {
        self.0.file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<&str> {
        self.0.extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<VPathBox, Error> {
        Ok(Box::new(BVPathBox(self.0.resolve(path)?)))
    }

    /// Get the parent path
    fn parent(&self) -> Option<VPathBox> {
        self.0.parent().map(|p| Box::new(BVPathBox(p)) as VPathBox)
    }

    /// Check if the file existst
    fn exists(&self) -> bool {
        self.0.exists()
    }

    /// Get the file's metadata
    fn metadata(&self) -> Result<Metadata, Error> {
        let req = self.0.metadata();
        match req {
            Ok(meta) => Ok(meta),
            Err(err) => Err(err),
        }
        // pin_mut!(req);
        // Box::pin(req.map_ok(|meta| Box::new(VMetadataBox(meta)) as Box<dyn BVMetadata>))
    }

    fn to_string(&self) -> String {
        self.0.to_string()
    }

    fn open(&self, options: OpenOptions) -> Result<VFileBox, Error> {
        let req = self.0.open(options);
        match req {
            Ok(file) => Ok(Box::new(file) as VFileBox),
            Err(err) => Err(err),
        }
    }

    fn read_dir(&self) -> Result<Box<dyn Iterator<Item = Result<VPathBox, Error>>>, Error> {
        let req = self.0.read_dir();
        match req {
            Ok(p) => {
                Ok(Box::new(ReadDirBox::new(p))
                    as Box<dyn Iterator<Item = Result<VPathBox, Error>>>)
            }
            Err(err) => Err(err),
        }
    }

    /// Create a directory at the location by this path
    fn create_dir(&self) -> Result<(), Error> {
        self.0.create_dir()
    }
    /// Remove a file
    fn rm(&self) -> Result<(), Error> {
        self.0.rm()
    }
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> Result<(), Error> {
        self.0.rm_all()
    }

    fn box_clone(&self) -> VPathBox {
        Box::new(BVPathBox(self.0.clone()))
    }
}

struct BVMetadataBox<M>(M);

impl<M> VMetadata for BVMetadataBox<M>
where
    M: VMetadata,
{
    fn is_dir(&self) -> bool {
        self.0.is_dir()
    }
    /// Returns true iff this path is a file
    fn is_file(&self) -> bool {
        self.0.is_file()
    }
    /// Returns the length of the file at this path
    fn len(&self) -> u64 {
        self.0.len()
    }
}

impl VFile for Box<dyn VFile> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        (**self).read(buf)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        (**self).write(buf)
    }

    fn flush(&mut self) -> Result<(), Error> {
        (**self).flush()
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        (**self).seek(pos)
    }

    fn close(&mut self) -> Result<(), Error> {
        (**self).close()
    }
}

pin_project! {
    struct ReadDirBox<S> {
        #[pin]
        stream: S
    }
}

impl<S> ReadDirBox<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
}

impl<S, P> Iterator for ReadDirBox<S>
where
    S: Iterator<Item = Result<P, Error>>,
    P: VPath + 'static,
{
    type Item = Result<VPathBox, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        //self.stream.next().map(|m| Box::new())
        todo!()
    }
}

/* ***************

VPath

****************/

impl Clone for VPathBox {
    fn clone(&self) -> Self {
        self.as_ref().box_clone()
    }
}

impl VPath for VPathBox {
    type File = VFileBox;
    type ReadDir = Box<dyn Iterator<Item = Result<VPathBox, Error>>>;

    fn file_name(&self) -> Option<&str> {
        self.as_ref().file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<&str> {
        self.as_ref().extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<Self, Error> {
        self.as_ref().resolve(path)
    }

    /// Get the parent path
    fn parent(&self) -> Option<Self> {
        self.as_ref().parent()
    }

    /// Check if the file existst
    fn exists(&self) -> bool {
        self.as_ref().exists()
    }

    /// Get the file's metadata
    fn metadata(&self) -> Result<Metadata, Error> {
        self.as_ref().metadata()
    }

    fn to_string(&self) -> String {
        self.as_ref().to_string()
    }

    fn open(&self, options: OpenOptions) -> Result<Self::File, Error> {
        self.as_ref().open(options)
    }

    fn read_dir(&self) -> Result<Self::ReadDir, Error> {
        self.as_ref().read_dir()
    }

    /// Create a directory at the location by this path
    fn create_dir(&self) -> Result<(), Error> {
        self.as_ref().create_dir()
    }
    /// Remove a file
    fn rm(&self) -> Result<(), Error> {
        self.as_ref().rm()
    }
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> Result<(), Error> {
        self.as_ref().rm_all()
    }
}

impl VFS for Box<dyn BVFS> {
    type Path = VPathBox;
    fn path(&self, path: &str) -> Result<Self::Path, Error> {
        self.as_ref().path(path)
    }
}

impl Clone for Box<dyn BVFS> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

pub fn vfs_box<V: VFS + 'static + Send>(v: V) -> VFSBox
where
    V: Clone,
    V::Path: Send + Sync + Clone,
{
    Box::new(BVFSBox(v))
}

pub fn vpath_box<V: VPath + 'static + Clone>(path: V) -> VPathBox
where
    V: Send + Sync + Clone,
{
    Box::new(BVPathBox(path))
}
