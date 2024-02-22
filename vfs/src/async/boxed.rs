use super::{file::VAsyncFile, types::VAsyncPath};
use crate::{error::Error, Metadata, OpenOptions, SeekFrom, VAsyncFS, VMetadata};
use async_trait::async_trait;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures_core::{ready, Stream};
// use futures_io::{AsyncRead, AsyncSeek, AsyncWrite, SeekFrom};
use alloc::{boxed::Box, string::String};
use pin_project_lite::pin_project;

pub trait BVAsyncFS: Sync + Send {
    fn path(&self, path: &str) -> Result<VAsyncPathBox, Error>;
    fn box_clone(&self) -> VAsyncFSBox;
}

pub type VAsyncFSBox = Box<dyn BVAsyncFS>;

impl Clone for VAsyncFSBox {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

pub type VAsyncPathBox = Box<dyn BVAsyncPath>;

pub type VAsyncFileBox = Pin<Box<dyn VAsyncFile>>;

#[async_trait]
pub trait BVAsyncPath: Send + Sync {
    fn file_name(&self) -> Option<&str>;

    /// The extension of this filename
    fn extension(&self) -> Option<&str>;

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<VAsyncPathBox, Error>;

    /// Get the parent path
    fn parent(&self) -> Option<VAsyncPathBox>;

    /// Check if the file existst
    async fn exists(&self) -> bool;

    /// Get the file's metadata
    async fn metadata(&self) -> Result<Metadata, Error>;

    fn to_string(&self) -> String;

    // fn to_path_buf(&self) -> Option<PathBuf>;

    async fn open(&self, options: OpenOptions) -> Result<VAsyncFileBox, Error>;

    async fn read_dir(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<VAsyncPathBox, Error>> + Send>>, Error>;

    /// Create a directory at the location by this path
    async fn create_dir(&self) -> Result<(), Error>;
    /// Remove a file
    async fn rm(&self) -> Result<(), Error>;
    /// Remove a file or directory and all its contents
    async fn rm_all(&self) -> Result<(), Error>;

    fn box_clone(&self) -> VAsyncPathBox;

    fn into_fs(&self) -> Result<VAsyncFSBox, Error>;

    #[cfg(feature = "std")]
    fn into_path_buf(&self) -> Option<std::path::PathBuf> {
        None
    }
}

struct BVAsyncFSBox<V>(V);

impl<V: VAsyncFS> BVAsyncFS for BVAsyncFSBox<V>
where
    V: Send + Clone + 'static,
    V::Path: 'static + Send + Sync,
    <V::Path as VAsyncPath>::ReadDir: Send,
{
    fn path(&self, path: &str) -> Result<VAsyncPathBox, Error> {
        Ok(Box::new(BVAsyncPathBox(self.0.path(path)?)))
    }

    fn box_clone(&self) -> VAsyncFSBox {
        Box::new(BVAsyncFSBox(self.0.clone()))
    }
}

struct BVAsyncPathBox<P>(P);

#[async_trait]
impl<P> BVAsyncPath for BVAsyncPathBox<P>
where
    P: Send + Sync + VAsyncPath + 'static,
    P::FS: Clone,
    P::ReadDir: Send,
{
    fn file_name(&self) -> Option<&str> {
        self.0.file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<&str> {
        self.0.extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<VAsyncPathBox, Error> {
        Ok(Box::new(BVAsyncPathBox(self.0.resolve(path)?)))
    }

    /// Get the parent path
    fn parent(&self) -> Option<VAsyncPathBox> {
        self.0
            .parent()
            .map(|p| Box::new(BVAsyncPathBox(p)) as VAsyncPathBox)
    }

    /// Check if the file existst
    async fn exists(&self) -> bool {
        self.0.exists().await
    }

    /// Get the file's metadata
    async fn metadata(&self) -> Result<Metadata, Error> {
        let req = self.0.metadata();
        match req.await {
            Ok(meta) => Ok(meta),
            Err(err) => Err(err),
        }
        // pin_mut!(req);
        // Box::pin(req.map_ok(|meta| Box::new(VMetadataBox(meta)) as Box<dyn BVMetadata>))
    }

    fn to_string(&self) -> String {
        self.0.to_string()
    }

    async fn open(&self, options: OpenOptions) -> Result<VAsyncFileBox, Error> {
        let req = self.0.open(options);
        match req.await {
            Ok(file) => Ok(Box::pin(BVAsyncFileBox::new(file)) as VAsyncFileBox),
            Err(err) => Err(err),
        }
    }

    async fn read_dir(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<VAsyncPathBox, Error>> + Send>>, Error> {
        let req = self.0.read_dir();
        match req.await {
            Ok(p) => Ok(Box::pin(ReadDirBox::new(p))
                as Pin<Box<dyn Stream<Item = Result<VAsyncPathBox, Error>> + Send>>),
            Err(err) => Err(err),
        }
    }

    /// Create a directory at the location by this path
    async fn create_dir(&self) -> Result<(), Error> {
        self.0.create_dir().await
    }
    /// Remove a file
    async fn rm(&self) -> Result<(), Error> {
        self.0.rm().await
    }
    /// Remove a file or directory and all its contents
    async fn rm_all(&self) -> Result<(), Error> {
        self.0.rm_all().await
    }

    fn box_clone(&self) -> VAsyncPathBox {
        Box::new(BVAsyncPathBox(self.0.clone()))
    }

    fn into_fs(&self) -> Result<VAsyncFSBox, Error> {
        let path = P::FS::from_path(&self.0)?;
        Ok(Box::new(BVAsyncFSBox(path)))
    }

    #[cfg(feature = "std")]
    fn into_path_buf(&self) -> Option<std::path::PathBuf> {
        self.0.into_path_buf()
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

pin_project! {
    struct BVAsyncFileBox<F> {
        #[pin]
        future:F
    }
}

impl<F> BVAsyncFileBox<F> {
    pub fn new(future: F) -> Self {
        Self { future }
    }
}

impl<F> VAsyncFile for BVAsyncFileBox<F>
where
    F: VAsyncFile,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        self.project().future.poll_read(cx, buf)
    }

    fn poll_seek(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<Result<u64, Error>> {
        self.project().future.poll_seek(cx, pos)
    }

    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        self.project().future.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.project().future.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.project().future.poll_close(cx)
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

impl<S, P> Stream for ReadDirBox<S>
where
    S: Stream<Item = Result<P, Error>> + Send,
    P: VAsyncPath + 'static,
    P::ReadDir: Send,
    P::FS: Clone,
{
    type Item = Result<VAsyncPathBox, Error>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match ready!(self.project().stream.poll_next(cx)) {
            Some(Ok(item)) => Poll::Ready(Some(Ok(Box::new(BVAsyncPathBox(item))))),
            Some(Err(err)) => Poll::Ready(Some(Err(err))),
            None => Poll::Ready(None),
        }
    }
}

/* ***************

VPath

****************/

impl Clone for VAsyncPathBox {
    fn clone(&self) -> Self {
        self.as_ref().box_clone()
    }
}

#[async_trait]
impl VAsyncPath for VAsyncPathBox {
    type FS = VAsyncFSBox;
    type File = VAsyncFileBox;
    type ReadDir = Pin<Box<dyn Stream<Item = Result<VAsyncPathBox, Error>> + Send>>;

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
    async fn exists(&self) -> bool {
        self.as_ref().exists().await
    }

    /// Get the file's metadata
    async fn metadata(&self) -> Result<Metadata, Error> {
        self.as_ref().metadata().await
    }

    fn to_string(&self) -> String {
        self.as_ref().to_string()
    }

    async fn open(&self, options: OpenOptions) -> Result<Self::File, Error> {
        self.as_ref().open(options).await
    }

    async fn read_dir(&self) -> Result<Self::ReadDir, Error> {
        self.as_ref().read_dir().await
    }

    /// Create a directory at the location by this path
    async fn create_dir(&self) -> Result<(), Error> {
        self.as_ref().create_dir().await
    }
    /// Remove a file
    async fn rm(&self) -> Result<(), Error> {
        self.as_ref().rm().await
    }
    /// Remove a file or directory and all its contents
    async fn rm_all(&self) -> Result<(), Error> {
        self.as_ref().rm_all().await
    }

    #[cfg(feature = "std")]
    fn into_path_buf(&self) -> Option<std::path::PathBuf> {
        self.as_ref().into_path_buf()
    }
}

impl VMetadata for Box<dyn VMetadata + Send> {
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

impl VAsyncFS for Box<dyn BVAsyncFS> {
    type Path = VAsyncPathBox;
    fn path(&self, path: &str) -> Result<Self::Path, Error> {
        self.as_ref().path(path)
    }

    fn from_path(path: &Self::Path) -> Result<Self, Error> {
        path.into_fs()
    }
}

pub fn vafs_box<V: VAsyncFS + 'static + Send>(v: V) -> VAsyncFSBox
where
    V: Clone,
    <V::Path as VAsyncPath>::ReadDir: Send,
{
    Box::new(BVAsyncFSBox(v))
}

pub fn vapath_box<V: VAsyncPath + 'static>(path: V) -> VAsyncPathBox
where
    V::ReadDir: Send,
    V::FS: Clone,
{
    Box::new(BVAsyncPathBox(path))
}
