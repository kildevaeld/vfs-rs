use crate::{OpenOptions, VFile, VMetadata, VPath, VFS};
use async_trait::async_trait;
use futures_core::{ready, Stream};
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite, SeekFrom};
use pin_project_lite::pin_project;
use std::{
    io::Result,
    pin::Pin,
    task::{Context, Poll},
};

pub trait BVFS: Sync + Send {
    fn path(&self, path: &str) -> Result<VPathBox>;
}

pub type VFSBox = Box<dyn BVFS>;

pub type VPathBox = Box<dyn BVPath>;

pub type VMetadataBox = Box<dyn VMetadata + Send>;

pub type VFileBox = Pin<Box<dyn VFile>>;

#[async_trait]
pub trait BVPath: Send + Sync {
    fn file_name(&self) -> Option<&str>;

    /// The extension of this filename
    fn extension(&self) -> Option<&str>;

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<VPathBox>;

    /// Get the parent path
    fn parent(&self) -> Option<VPathBox>;

    /// Check if the file existst
    async fn exists(&self) -> bool;

    /// Get the file's metadata
    async fn metadata(&self) -> Result<VMetadataBox>;

    fn to_string(&self) -> String;

    // fn to_path_buf(&self) -> Option<PathBuf>;

    async fn open(&self, options: OpenOptions) -> Result<VFileBox>;

    async fn read_dir(&self) -> Result<Pin<Box<dyn Stream<Item = Result<VPathBox>> + Send>>>;

    /// Create a directory at the location by this path
    async fn create_dir(&self) -> Result<()>;
    /// Remove a file
    async fn rm(&self) -> Result<()>;
    /// Remove a file or directory and all its contents
    async fn rm_all(&self) -> Result<()>;

    fn box_clone(&self) -> VPathBox;
}

struct BVFSBox<V>(V);

impl<V: VFS> BVFS for BVFSBox<V>
where
    V: Send,
    V::Path: 'static + Send + Sync,
    <V::Path as VPath>::ReadDir: Send,
    <V::Path as VPath>::Metadata: Send,
{
    fn path(&self, path: &str) -> Result<VPathBox> {
        Ok(Box::new(BVPathBox(self.0.path(path)?)))
    }
}

struct BVPathBox<P>(P);

#[async_trait]
impl<P> BVPath for BVPathBox<P>
where
    P: Send + Sync + VPath + 'static,
    P::ReadDir: Send,
    P::Metadata: Send,
{
    fn file_name(&self) -> Option<&str> {
        self.0.file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<&str> {
        self.0.extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<VPathBox> {
        Ok(Box::new(BVPathBox(self.0.resolve(path)?)))
    }

    /// Get the parent path
    fn parent(&self) -> Option<VPathBox> {
        self.0.parent().map(|p| Box::new(BVPathBox(p)) as VPathBox)
    }

    /// Check if the file existst
    async fn exists(&self) -> bool {
        self.0.exists().await
    }

    /// Get the file's metadata
    async fn metadata(&self) -> Result<VMetadataBox> {
        let req = self.0.metadata();
        match req.await {
            Ok(meta) => Ok(Box::new(BVMetadataBox(meta)) as VMetadataBox),
            Err(err) => Err(err),
        }
        // pin_mut!(req);
        // Box::pin(req.map_ok(|meta| Box::new(VMetadataBox(meta)) as Box<dyn BVMetadata>))
    }

    fn to_string(&self) -> String {
        self.0.to_string()
    }

    async fn open(&self, options: OpenOptions) -> Result<VFileBox> {
        let req = self.0.open(options);
        match req.await {
            Ok(file) => Ok(Box::pin(BVFileBox::new(file)) as VFileBox),
            Err(err) => Err(err),
        }
    }

    async fn read_dir(&self) -> Result<Pin<Box<dyn Stream<Item = Result<VPathBox>> + Send>>> {
        let req = self.0.read_dir();
        match req.await {
            Ok(p) => Ok(Box::pin(ReadDirBox::new(p))
                as Pin<Box<dyn Stream<Item = Result<VPathBox>> + Send>>),
            Err(err) => Err(err),
        }
    }

    /// Create a directory at the location by this path
    async fn create_dir(&self) -> Result<()> {
        self.0.create_dir().await
    }
    /// Remove a file
    async fn rm(&self) -> Result<()> {
        self.0.rm().await
    }
    /// Remove a file or directory and all its contents
    async fn rm_all(&self) -> Result<()> {
        self.0.rm_all().await
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

pin_project! {
    struct BVFileBox<F> {
        #[pin]
        future:F
    }
}

impl<F> BVFileBox<F> {
    pub fn new(future: F) -> Self {
        Self { future }
    }
}

impl<F> AsyncRead for BVFileBox<F>
where
    F: VFile,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        self.project().future.poll_read(cx, buf)
    }
}

impl<F> AsyncWrite for BVFileBox<F>
where
    F: VFile,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.project().future.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().future.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().future.poll_close(cx)
    }
}

impl<F> AsyncSeek for BVFileBox<F>
where
    F: VFile,
{
    fn poll_seek(self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<Result<u64>> {
        self.project().future.poll_seek(cx, pos)
    }
}

impl<F> VFile for BVFileBox<F> where F: VFile {}

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
    S: Stream<Item = Result<P>> + Send,
    P: VPath + 'static,
    P::ReadDir: Send,
    P::Metadata: Send,
{
    type Item = Result<VPathBox>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match ready!(self.project().stream.poll_next(cx)) {
            Some(Ok(item)) => Poll::Ready(Some(Ok(Box::new(BVPathBox(item))))),
            Some(Err(err)) => Poll::Ready(Some(Err(err))),
            None => Poll::Ready(None),
        }
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

#[async_trait]
impl VPath for VPathBox {
    type Metadata = Box<dyn VMetadata + Send>;
    type File = VFileBox;
    type ReadDir = Pin<Box<dyn Stream<Item = Result<VPathBox>> + Send>>;

    fn file_name(&self) -> Option<&str> {
        self.as_ref().file_name()
    }

    /// The extension of this filename
    fn extension(&self) -> Option<&str> {
        self.as_ref().extension()
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Result<Self> {
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
    async fn metadata(&self) -> Result<Self::Metadata> {
        self.as_ref().metadata().await
    }

    fn to_string(&self) -> String {
        self.as_ref().to_string()
    }

    async fn open(&self, options: OpenOptions) -> Result<Self::File> {
        self.as_ref().open(options).await
    }

    async fn read_dir(&self) -> Result<Self::ReadDir> {
        self.as_ref().read_dir().await
    }

    /// Create a directory at the location by this path
    async fn create_dir(&self) -> Result<()> {
        self.as_ref().create_dir().await
    }
    /// Remove a file
    async fn rm(&self) -> Result<()> {
        self.as_ref().rm().await
    }
    /// Remove a file or directory and all its contents
    async fn rm_all(&self) -> Result<()> {
        self.as_ref().rm_all().await
    }
}

impl VFile for VFileBox {}

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

impl VFS for Box<dyn BVFS> {
    type Path = VPathBox;
    fn path(&self, path: &str) -> Result<Self::Path> {
        self.as_ref().path(path)
    }
}

pub fn vfs_box<V: VFS + 'static + Send>(v: V) -> VFSBox
where
    <V::Path as VPath>::ReadDir: Send,
    <V::Path as VPath>::Metadata: Send,
{
    Box::new(BVFSBox(v))
}

pub fn vpath_box<V: VPath + 'static>(path: V) -> VPathBox
where
    V::ReadDir: Send,
    V::Metadata: Send,
{
    Box::new(BVPathBox(path))
}
