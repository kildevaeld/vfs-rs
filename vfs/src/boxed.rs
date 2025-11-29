use core::pin::Pin;

use dyn_clone::DynClone;
use futures::{StreamExt, TryStreamExt};
use futures_core::{future::BoxFuture, stream::BoxStream};
use std::boxed::Box;

use crate::{Error, Metadata, OpenOptions, VFS, VFile, VPath};

pub type BoxVPath = Box<dyn VPathBox + Send + Sync>;

pub type BoxVFile = Pin<Box<dyn VFile + Send + Sync>>;

pub type BoxVFS = Box<dyn VFSBox + Send + Sync>;

pub fn path_box<T>(path: T) -> BoxVPath
where
    T: Clone + 'static,
    T: VPath + Send + Sync,
    T::File: Send + Sync + 'static,
    T::Metadata: Send + 'static,
    T::Open: Send + 'static,
    T::CreateDir: Send + 'static,
    T::Remove: Send + 'static,
    T::ReadDir: Send + 'static,
    T::ListDir: Send + 'static,
{
    Box::new(BoxedVPath(path))
}

pub trait VFSBox: DynClone {
    fn path(&self, path: &str) -> Result<BoxVPath, Error>;
}

dyn_clone::clone_trait_object!(VFSBox);

pub trait VPathBox: DynClone {
    fn file_name(&self) -> Option<&str>;

    /// The extension of this filename
    fn extension(&self) -> Option<&str>;

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<BoxVPath, Error>;

    /// Get the parent path
    fn parent(&self) -> Option<BoxVPath>;

    /// Get the file's metadata
    fn metadata(&self) -> BoxFuture<'static, Result<Metadata, Error>>;

    fn open(&self, options: OpenOptions) -> BoxFuture<'static, Result<BoxVFile, Error>>;
    fn read_dir(
        &self,
    ) -> BoxFuture<'static, Result<BoxStream<'static, Result<BoxVPath, Error>>, Error>>;

    /// Create a directory at the location by this path
    fn create_dir(&self) -> BoxFuture<'static, Result<(), Error>>;

    /// Remove a file or directory and all its contents
    fn rm(&self) -> BoxFuture<'static, Result<(), Error>>;
}

dyn_clone::clone_trait_object!(VPathBox);

#[derive(Clone)]
struct BoxedVPath<T>(T);

impl<T> VPathBox for BoxedVPath<T>
where
    T: Clone + 'static,
    T: VPath + Send + Sync,
    T::File: Send + Sync + 'static,
    T::Metadata: Send + 'static,
    T::Open: Send + 'static,
    T::CreateDir: Send + 'static,
    T::Remove: Send + 'static,
    T::ReadDir: Send + 'static,
    T::ListDir: Send + 'static,
{
    fn file_name(&self) -> Option<&str> {
        self.0.file_name()
    }

    fn extension(&self) -> Option<&str> {
        self.0.extension()
    }

    fn resolve(&self, path: &str) -> Result<BoxVPath, Error> {
        self.0
            .resolve(path)
            .map(|m| Box::new(BoxedVPath(m)) as BoxVPath)
    }

    fn parent(&self) -> Option<BoxVPath> {
        self.0.parent().map(|m| Box::new(BoxedVPath(m)) as BoxVPath)
    }

    fn metadata(&self) -> BoxFuture<'static, Result<Metadata, Error>> {
        let future = self.0.metadata();
        Box::pin(future)
    }

    fn open(&self, options: OpenOptions) -> BoxFuture<'static, Result<BoxVFile, Error>> {
        let future = self.0.open(options);
        Box::pin(async move {
            let ret = future.await?;
            Ok(Box::pin(ret) as BoxVFile)
        })
    }

    fn read_dir(
        &self,
    ) -> BoxFuture<'static, Result<BoxStream<'static, Result<BoxVPath, Error>>, Error>> {
        let future = self.0.read_dir();
        Box::pin(async move {
            let read_dir = future.await?;
            let stream = read_dir
                .map_ok(|m| Box::new(BoxedVPath(m)) as BoxVPath)
                .boxed();
            Ok(stream)
        })
    }

    fn create_dir(&self) -> BoxFuture<'static, Result<(), Error>> {
        let future = self.0.create_dir();
        Box::pin(future)
    }

    fn rm(&self) -> BoxFuture<'static, Result<(), Error>> {
        let future = self.0.rm();
        Box::pin(future)
    }
}

impl VFS for BoxVFS {
    type Path = BoxVPath;

    fn path(&self, path: &str) -> Result<Self::Path, Error> {
        (**self).path(path)
    }
}

impl VPath for BoxVPath {
    type FS = BoxVFS;

    type File = BoxVFile;

    type ListDir = BoxStream<'static, Result<BoxVPath, Error>>;

    type Metadata = BoxFuture<'static, Result<Metadata, Error>>;

    type Open = BoxFuture<'static, Result<BoxVFile, Error>>;

    type CreateDir = BoxFuture<'static, Result<(), Error>>;

    type Remove = BoxFuture<'static, Result<(), Error>>;

    type ReadDir = BoxFuture<'static, Result<Self::ListDir, Error>>;

    fn file_name(&self) -> Option<&str> {
        (**self).file_name()
    }

    fn extension(&self) -> Option<&str> {
        (**self).extension()
    }

    fn resolve(&self, path: &str) -> Result<Self, Error> {
        (**self).resolve(path)
    }

    fn parent(&self) -> Option<Self> {
        (**self).parent()
    }

    fn metadata(&self) -> Self::Metadata {
        (**self).metadata()
    }

    fn open(&self, options: OpenOptions) -> Self::Open {
        (**self).open(options)
    }

    fn read_dir(&self) -> Self::ReadDir {
        (**self).read_dir()
    }

    fn create_dir(&self) -> Self::CreateDir {
        (**self).create_dir()
    }

    fn rm(&self) -> Self::Remove {
        (**self).rm()
    }
}
