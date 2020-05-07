use crate::{OpenOptions, VFile, VMetadata, VPath, VFS};
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite, SeekFrom};
use futures_util::{
    future::{BoxFuture, FutureExt, TryFuture, TryFutureExt},
    pin_mut,
};
use pin_project::pin_project;
use std::borrow::Cow;
use std::io::Result;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait BVFS: Sync + Send {
    fn path(&self, path: &str) -> Box<dyn BVPath>;
}

pub trait BVPath: Send + Sync {
    fn file_name(&self) -> Option<String>;

    /// The extension of this filename
    fn extension(&self) -> Option<String>;

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Box<dyn BVPath>;

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BVPath>>;

    /// Check if the file existst
    fn exists(&self) -> BoxFuture<'static, bool>;

    /// Get the file's metadata
    fn metadata(&self) -> BoxFuture<'static, Result<Box<dyn VMetadata>>>;

    fn to_string(&self) -> Cow<str>;

    fn to_path_buf(&self) -> Option<PathBuf>;

    fn open(&self, options: OpenOptions) -> BoxFuture<'static, Result<Pin<Box<dyn VFile>>>>;
    fn read_dir(
        &self,
    ) -> BoxFuture<'static, Result<Pin<Box<dyn Stream<Item = Result<Box<dyn BVPath>>> + Send>>>>;

    /// Create a directory at the location by this path
    fn mkdir(&self) -> BoxFuture<'static, Result<()>>;
    /// Remove a file
    fn rm(&self) -> BoxFuture<'static, Result<()>>;
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> BoxFuture<'static, Result<()>>;

    fn create(&self) -> BoxFuture<'static, Result<Pin<Box<dyn VFile>>>> {
        self.open(OpenOptions::new().write(true).create(true).truncate(true))
    }
    fn append(&self) -> BoxFuture<'static, Result<Pin<Box<dyn VFile>>>> {
        self.open(OpenOptions::new().write(true).create(true).append(true))
    }

    fn box_clone(&self) -> Box<dyn BVPath>;
}

// pub trait BVMetadata {}

// pub trait BVFile {}

struct VFSBox<V>(V);

impl<V: VFS> BVFS for VFSBox<V>
where
    V::Path: 'static + Send + Sync,
    <V::Path as VPath>::ReadDir: Send,
{
    fn path(&self, path: &str) -> Box<dyn BVPath> {
        Box::new(VPathBox(self.0.path(path)))
    }
}

struct VPathBox<P>(P);

impl<P> BVPath for VPathBox<P>
where
    P: Send + Sync + VPath + 'static,
    P::ReadDir: Send,
{
    fn file_name(&self) -> Option<String> {
        self.0.file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        self.0.extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Box<dyn BVPath> {
        Box::new(VPathBox(self.0.resolve(path)))
    }

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BVPath>> {
        self.0
            .parent()
            .map(|p| Box::new(VPathBox(p)) as Box<dyn BVPath>)
    }

    /// Check if the file existst
    fn exists(&self) -> BoxFuture<'static, bool> {
        self.0.exists()
    }

    /// Get the file's metadata
    fn metadata(&self) -> BoxFuture<'static, Result<Box<dyn VMetadata>>> {
        let req = self.0.metadata();
        let fut = async move {
            match req.await {
                Ok(meta) => Ok(Box::new(VMetadataBox(meta)) as Box<dyn VMetadata>),
                Err(err) => Err(err),
            }
        };

        Box::pin(fut)
        // pin_mut!(req);
        // Box::pin(req.map_ok(|meta| Box::new(VMetadataBox(meta)) as Box<dyn BVMetadata>))
    }

    fn to_string(&self) -> Cow<str> {
        self.0.to_string()
    }

    fn to_path_buf(&self) -> Option<PathBuf> {
        self.0.to_path_buf()
    }

    fn open(&self, options: OpenOptions) -> BoxFuture<'static, Result<Pin<Box<dyn VFile>>>> {
        let req = self.0.open(options);
        Box::pin(async move {
            match req.await {
                Ok(file) => Ok(Box::pin(VFileBox(file)) as Pin<Box<dyn VFile>>),
                Err(err) => Err(err),
            }
        })
    }

    fn read_dir(
        &self,
    ) -> BoxFuture<'static, Result<Pin<Box<dyn Stream<Item = Result<Box<dyn BVPath>>> + Send>>>>
    {
        let req = self.0.read_dir();
        Box::pin(async move {
            match req.await {
                Ok(p) => Ok(Box::pin(ReadDirBox(p))
                    as Pin<Box<dyn Stream<Item = Result<Box<dyn BVPath>>> + Send>>),
                Err(err) => Err(err),
            }
        })
    }

    /// Create a directory at the location by this path
    fn mkdir(&self) -> BoxFuture<'static, Result<()>> {
        self.0.mkdir()
    }
    /// Remove a file
    fn rm(&self) -> BoxFuture<'static, Result<()>> {
        self.0.rm()
    }
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> BoxFuture<'static, Result<()>> {
        self.0.rm_all()
    }

    fn box_clone(&self) -> Box<dyn BVPath> {
        Box::new(VPathBox(self.0.clone()))
    }
}

struct VMetadataBox<M>(M);

impl<M> VMetadata for VMetadataBox<M>
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

struct VFileBox<F>(F);

impl<F> AsyncRead for VFileBox<F>
where
    F: VFile,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        Poll::Pending
    }
}

impl<F> AsyncWrite for VFileBox<F>
where
    F: VFile,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        Poll::Pending
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Pending
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Pending
    }
}

impl<F> AsyncSeek for VFileBox<F>
where
    F: VFile,
{
    fn poll_seek(self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<Result<u64>> {
        Poll::Pending
    }
}

impl<F> VFile for VFileBox<F> where F: VFile {}

#[pin_project]
struct ReadDirBox<S>(#[pin] S);

impl<S, P> Stream for ReadDirBox<S>
where
    S: Stream<Item = Result<P>> + Send,
    P: VPath,
{
    type Item = Result<Box<dyn BVPath>>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Pending
    }
}

/* ***************

VPath

****************/

impl Clone for Box<dyn BVPath> {
    fn clone(&self) -> Self {
        self.as_ref().box_clone()
    }
}

impl VPath for Box<dyn BVPath> {
    type Metadata = Box<dyn VMetadata>;
    type File = Pin<Box<dyn VFile>>;
    type ReadDir = Pin<Box<dyn Stream<Item = Result<Box<dyn BVPath>>> + Send>>;

    fn file_name(&self) -> Option<String> {
        self.as_ref().file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        self.as_ref().extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Self {
        self.as_ref().resolve(path)
    }

    /// Get the parent path
    fn parent(&self) -> Option<Self> {
        self.as_ref().parent()
    }

    /// Check if the file existst
    fn exists(&self) -> BoxFuture<'static, bool> {
        self.as_ref().exists()
    }

    /// Get the file's metadata
    fn metadata(&self) -> BoxFuture<'static, Result<Self::Metadata>> {
        self.as_ref().metadata()
    }

    fn to_string(&self) -> Cow<str> {
        self.as_ref().to_string()
    }

    fn to_path_buf(&self) -> Option<PathBuf> {
        self.as_ref().to_path_buf()
    }

    fn open(&self, options: OpenOptions) -> BoxFuture<'static, Result<Self::File>> {
        self.as_ref().open(options)
    }

    fn read_dir(&self) -> BoxFuture<'static, Result<Self::ReadDir>> {
        self.as_ref().read_dir()
    }

    /// Create a directory at the location by this path
    fn mkdir(&self) -> BoxFuture<'static, Result<()>> {
        self.as_ref().mkdir()
    }
    /// Remove a file
    fn rm(&self) -> BoxFuture<'static, Result<()>> {
        self.as_ref().rm()
    }
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> BoxFuture<'static, Result<()>> {
        self.as_ref().rm_all()
    }
}

impl VFile for Pin<Box<dyn VFile>> {}

impl VMetadata for Box<dyn VMetadata> {
    fn is_dir(&self) -> bool {
        self.as_ref().is_dir()
    }
    /// Returns true iff this path is a file
    fn is_file(&self) -> bool {
        self.as_ref().is_file()
    }
    /// Returns the length of the file at this path
    fn len(&self) -> u64 {
        self.as_ref().len()
    }
}

impl VFS for Box<dyn BVFS> {
    type Path = Box<dyn BVPath>;
    fn path(&self, path: &str) -> Self::Path {
        self.as_ref().path(path)
    }
}

pub fn vfs_box<V: VFS + 'static + Send>(v: V) -> Box<dyn BVFS>
where
    <V::Path as VPath>::ReadDir: Send,
{
    Box::new(VFSBox(v))
}

pub fn path_box<V: VPath + 'static>(path: V) -> Box<dyn BVPath>
where
    V::ReadDir: Send,
{
    Box::new(VPathBox(path))
}
