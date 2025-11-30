use std::{
    os::unix::fs::MetadataExt,
    path::PathBuf,
    task::{Poll, ready},
};

use async_compat::Compat;
use futures_core::{Stream, future::BoxFuture};
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};
use pin_project_lite::pin_project;
use relative_path::RelativePath;
use vfs::{Error, ErrorKind, FileType, Metadata, VFS, VFile, VPath};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FS(PathBuf);

impl FS {
    pub async fn new(path: PathBuf) -> Result<FS, Error> {
        let meta = tokio::fs::metadata(&path).await?;

        if !meta.is_dir() {
            return Err(Error::from(ErrorKind::NotADirectory));
        }

        Ok(FS(path))
    }
}

impl VFS for FS {
    type Path = Path;

    fn path(&self, path: &str) -> Result<Self::Path, vfs::Error> {
        Ok(Path(RelativePath::new(path).to_logical_path(&self.0)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path(PathBuf);

impl Path {
    pub const fn new(path: PathBuf) -> Path {
        Path(path)
    }

    pub fn real_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl VPath for Path {
    type FS = FS;

    type File = File;

    type ListDir = ListDir;

    type Metadata = PathWork<Metadata>;

    type Open = BoxFuture<'static, Result<File, Error>>;

    type CreateDir = PathWork<()>;

    type Remove = PathWork<()>;

    type ReadDir = BoxFuture<'static, Result<ListDir, Error>>;

    fn file_name(&self) -> Option<&str> {
        self.0.file_name().and_then(|m| m.to_str())
    }

    fn to_string(&self) -> String {
        self.0.display().to_string()
    }

    fn extension(&self) -> Option<&str> {
        self.0.extension().and_then(|m| m.to_str())
    }

    fn resolve(&self, path: &str) -> Result<Self, vfs::Error> {
        let path = RelativePath::new(path).to_logical_path(&self.0);
        Ok(Self(path))
    }

    fn parent(&self) -> Option<Self> {
        self.0.parent().map(|m| Self(m.to_path_buf()))
    }

    fn metadata(&self) -> Self::Metadata {
        let path = self.0.clone();
        PathWork {
            inner: tokio::task::spawn_blocking(move || {
                let metadata = std::fs::metadata(path)?;

                let ty = if metadata.is_dir() {
                    FileType::Dir
                } else {
                    FileType::File
                };

                vfs::Result::Ok(vfs::Metadata {
                    size: metadata.size(),
                    kind: ty,
                })
            }),
        }
    }

    fn open(&self, options: vfs::OpenOptions) -> Self::Open {
        let path = self.0.clone();
        Box::pin(async move {
            let mut ops = tokio::fs::OpenOptions::new();

            ops.append(options.append)
                .read(options.read)
                .write(options.write)
                .truncate(options.truncate)
                .create(options.create);

            let file = ops.open(path).await?;

            Ok(File {
                file: Compat::new(file),
            })
        })
    }

    fn read_dir(&self) -> Self::ReadDir {
        let path = self.0.clone();
        Box::pin(async move {
            let readdir = tokio::fs::read_dir(path).await?;
            Ok(ListDir { inner: readdir })
        })
    }

    fn create_dir(&self) -> Self::CreateDir {
        let path = self.0.clone();
        PathWork {
            inner: tokio::task::spawn_blocking(move || {
                std::fs::create_dir_all(path)?;
                vfs::Result::Ok(())
            }),
        }
    }

    fn rm(&self) -> Self::Remove {
        let path = self.0.clone();
        PathWork {
            inner: tokio::task::spawn_blocking(move || {
                std::fs::remove_dir_all(path)?;
                vfs::Result::Ok(())
            }),
        }
    }
}

pin_project! {
    pub struct PathWork<T> {
        #[pin]
        inner: tokio::task::JoinHandle<Result<T, Error>>
    }
}

impl<T> Future for PathWork<T> {
    type Output = Result<T, Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match ready!(self.project().inner.poll(cx)) {
            Ok(ret) => std::task::Poll::Ready(ret),
            Err(err) => Poll::Ready(Err(vfs::Error::new(vfs::ErrorKind::Other, err))),
        }
    }
}

pin_project! {
    pub struct ListDir {
        #[pin]
        inner: tokio::fs::ReadDir
    }
}

impl Stream for ListDir {
    type Item = Result<Path, Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match ready!(self.project().inner.poll_next_entry(cx)) {
            Ok(Some(ret)) => {
                let path = ret.path();
                Poll::Ready(Some(Ok(Path(path))))
            }
            Ok(None) => Poll::Ready(None),
            Err(err) => Poll::Ready(Some(Err(err.into()))),
        }
    }
}

pin_project! {
    pub struct File {
        #[pin]
        file: Compat<tokio::fs::File>
    }
}

impl VFile for File {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<Result<usize, vfs::Error>> {
        self.project()
            .file
            .poll_read(cx, buf)
            .map_err(|err| err.into())
    }

    fn poll_seek(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        pos: vfs::SeekFrom,
    ) -> std::task::Poll<Result<u64, vfs::Error>> {
        self.project()
            .file
            .poll_seek(cx, pos.into())
            .map_err(|err| err.into())
    }

    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, vfs::Error>> {
        self.project()
            .file
            .poll_write(cx, buf)
            .map_err(|err| err.into())
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), vfs::Error>> {
        self.project().file.poll_flush(cx).map_err(|err| err.into())
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), vfs::Error>> {
        self.project().file.poll_close(cx).map_err(|err| err.into())
    }
}
