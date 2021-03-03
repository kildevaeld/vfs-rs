use crate::{OpenOptions, VFile, VMetadata, VPath, VFS};
use async_trait::async_trait;
use futures_lite::{
    io::{AsyncRead, AsyncSeek, AsyncWrite, SeekFrom},
    ready, Stream,
};
use pin_project::pin_project;
use std::borrow::Cow;
use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait BVFS: Sync + Send {
    fn path(&self, path: &str) -> Box<dyn BVPath>;
}

#[async_trait]
pub trait BVPath: Send + Sync {
    fn file_name(&self) -> Option<String>;

    /// The extension of this filename
    fn extension(&self) -> Option<String>;

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Box<dyn BVPath>;

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BVPath>>;

    /// Check if the file existst
    async fn exists(&self) -> bool;

    /// Get the file's metadata
    async fn metadata(&self) -> Result<Box<dyn VMetadata>>;

    fn to_string(&self) -> Cow<str>;

    // fn to_path_buf(&self) -> Option<PathBuf>;

    async fn open(&self, options: OpenOptions) -> Result<Pin<Box<dyn VFile>>>;

    async fn read_dir(&self)
        -> Result<Pin<Box<dyn Stream<Item = Result<Box<dyn BVPath>>> + Send>>>;

    /// Create a directory at the location by this path
    async fn create_dir(&self) -> Result<()>;
    /// Remove a file
    async fn rm(&self) -> Result<()>;
    /// Remove a file or directory and all its contents
    async fn rm_all(&self) -> Result<()>;

    fn box_clone(&self) -> Box<dyn BVPath>;
}

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

#[async_trait]
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
    async fn exists(&self) -> bool {
        self.0.exists().await
    }

    /// Get the file's metadata
    async fn metadata(&self) -> Result<Box<dyn VMetadata>> {
        let req = self.0.metadata();
        match req.await {
            Ok(meta) => Ok(Box::new(VMetadataBox(meta)) as Box<dyn VMetadata>),
            Err(err) => Err(err),
        }
        // pin_mut!(req);
        // Box::pin(req.map_ok(|meta| Box::new(VMetadataBox(meta)) as Box<dyn BVMetadata>))
    }

    fn to_string(&self) -> Cow<str> {
        self.0.to_string()
    }

    async fn open(&self, options: OpenOptions) -> Result<Pin<Box<dyn VFile>>> {
        let req = self.0.open(options);
        match req.await {
            Ok(file) => Ok(Box::pin(VFileBox(file)) as Pin<Box<dyn VFile>>),
            Err(err) => Err(err),
        }
    }

    async fn read_dir(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Box<dyn BVPath>>> + Send>>> {
        let req = self.0.read_dir();
        match req.await {
            Ok(p) => Ok(Box::pin(ReadDirBox(p))
                as Pin<Box<dyn Stream<Item = Result<Box<dyn BVPath>>> + Send>>),
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

#[pin_project]
struct VFileBox<F>(#[pin] F);

impl<F> AsyncRead for VFileBox<F>
where
    F: VFile,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        self.project().0.poll_read(cx, buf)
    }
}

impl<F> AsyncWrite for VFileBox<F>
where
    F: VFile,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.project().0.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().0.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().0.poll_close(cx)
    }
}

impl<F> AsyncSeek for VFileBox<F>
where
    F: VFile,
{
    fn poll_seek(self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<Result<u64>> {
        self.project().0.poll_seek(cx, pos)
    }
}

impl<F> VFile for VFileBox<F> where F: VFile {}

#[pin_project]
struct ReadDirBox<S>(#[pin] S);

impl<S, P> Stream for ReadDirBox<S>
where
    S: Stream<Item = Result<P>> + Send,
    P: VPath + 'static,
    P::ReadDir: Send,
{
    type Item = Result<Box<dyn BVPath>>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match ready!(self.project().0.poll_next(cx)) {
            Some(Ok(item)) => Poll::Ready(Some(Ok(Box::new(VPathBox(item))))),
            Some(Err(err)) => Poll::Ready(Some(Err(err))),
            None => Poll::Ready(None),
        }
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

#[async_trait]
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
    async fn exists(&self) -> bool {
        self.as_ref().exists().await
    }

    /// Get the file's metadata
    async fn metadata(&self) -> Result<Self::Metadata> {
        self.as_ref().metadata().await
    }

    fn to_string(&self) -> Cow<str> {
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
