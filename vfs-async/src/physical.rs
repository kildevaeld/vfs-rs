use super::traits::{OpenOptions, VFile, VMetadata, VPath, VFS};
use pathutils;
use std::borrow::Cow;
use std::fmt::{self, Debug};
use std::fs::{canonicalize, Metadata};
// use std::fs::{
//     canonicalize, remove_dir, remove_dir_all, remove_file, DirBuilder, File, Metadata,
//     OpenOptions as FSOpenOptions, ReadDir,
// };
use async_trait::async_trait;
use futures_core::Stream;
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite, IoSlice, IoSliceMut};
use pin_project::pin_project;
use std::io::{Result, SeekFrom};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::fs::{self, File, OpenOptions as FSOpenOptions, ReadDir};
use tokio::io::{AsyncRead as TAsyncRead, AsyncSeek as TAsyncSeek, AsyncWrite as TAsyncWrite};

#[pin_project]
pub struct PhysicalFile(#[pin] File);

impl VFile for PhysicalFile {}

impl AsyncRead for PhysicalFile {
    #[cfg(feature = "read-initializer")]
    unsafe fn initializer(&self) -> Initializer {
        self.0.initializer()
    }

    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        self.project().0.poll_read(cx, buf)
    }
}

impl AsyncWrite for PhysicalFile {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        self.project().0.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().0.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().0.poll_shutdown(cx)
    }
}

impl AsyncSeek for PhysicalFile {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<Result<u64>> {
        // match self.project().0.start_seek(cx, pos) {
        //     Poll::Pending => Poll::Pending,
        //     Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
        //     Poll::Ready(Ok(_)) => {
        //         loop {}
        //     }
        // }
        Poll::Pending
    }
}

/// A "physical" file system implementation using the underlying OS file system
#[derive(Debug, Clone)]
pub struct PhysicalFS {
    root: Arc<PathBuf>,
}

impl PhysicalFS {
    pub fn new<S: AsRef<Path>>(path: S) -> Result<PhysicalFS> {
        let path = path.as_ref();
        let meta = path.metadata()?;
        if !meta.is_dir() {}

        let abs = canonicalize(path)?;

        Ok(PhysicalFS {
            root: Arc::new(abs),
        })
    }
}

impl VMetadata for Metadata {
    fn is_dir(&self) -> bool {
        self.is_dir()
    }
    fn is_file(&self) -> bool {
        self.is_file()
    }
    fn len(&self) -> u64 {
        self.len()
    }
}

impl VFS for PhysicalFS {
    type Path = PhysicalPath;

    fn path(&self, path: &str) -> PhysicalPath {
        if path.is_empty() || path == "." {
            return PhysicalPath {
                root: self.root.clone(),
                path: "".to_owned(),
                full_path: self.root.as_path().to_path_buf(),
            };
        } else if path == ".." {
            panic!("invalid path");
        }
        let fp = pathutils::resolve(&self.root.to_string_lossy(), &path).unwrap();
        if path.chars().nth(0).unwrap() == '/' {
            return PhysicalPath {
                root: self.root.clone(),
                path: path.to_string(),
                full_path: PathBuf::from(fp),
            };
        }
        let path = pathutils::resolve("/", path).unwrap(); // format!("/{}", path);

        PhysicalPath {
            root: self.root.clone(),
            path: path,
            full_path: PathBuf::from(fp),
        }
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
pub struct PhysicalPath {
    root: Arc<PathBuf>,
    path: String,
    full_path: PathBuf,
}

#[async_trait]
impl VPath for PhysicalPath {
    type Metadata = Metadata;
    type File = PhysicalFile;
    type ReadDir = PhysicalReadDir;

    fn parent(&self) -> Option<Self> {
        match self.full_path.parent() {
            Some(path) => {
                let path = path.to_string_lossy();
                let replaced = path.replace(self.root.to_str().unwrap(), "");

                if String::from(path) == replaced {
                    return None;
                }

                Some(PhysicalPath {
                    root: self.root.clone(),
                    path: replaced.to_string(),
                    full_path: match pathutils::resolve(
                        self.root.to_string_lossy().as_ref(),
                        &replaced,
                    ) {
                        Ok(m) => PathBuf::from(m),
                        Err(_) => return None,
                    },
                })
            }
            None => None,
        }
    }

    fn file_name(&self) -> Option<String> {
        match pathutils::filename(&self.path) {
            Some(name) => Some(name),
            None => None,
        }
    }

    fn extension(&self) -> Option<String> {
        match pathutils::extname(&self.path) {
            Some(name) => Some(name),
            None => None,
        }
    }

    fn resolve(&self, path: &str) -> Self {
        let full_path =
            match pathutils::resolve(self.full_path.as_path().to_string_lossy().as_ref(), path) {
                Ok(s) => s,
                Err(_) => unimplemented!("resolve parent"),
            };

        let path = if self.root.as_path() == Path::new("/") {
            full_path.trim_start_matches("/").to_string()
        } else {
            full_path.replace(self.root.to_str().unwrap(), "")
        };

        return PhysicalPath {
            path: path,
            root: self.root.clone(),
            full_path: PathBuf::from(full_path),
        };
    }

    async fn exists(&self) -> bool {
        self.full_path.exists()
    }

    async fn metadata(&self) -> Result<Self::Metadata> {
        self.full_path.metadata()
    }

    fn to_path_buf(&self) -> Option<PathBuf> {
        Some(self.full_path.clone())
    }

    fn to_string(&self) -> Cow<str> {
        Cow::from(&self.path)
    }

    async fn open(&self, o: OpenOptions) -> Result<Self::File> {
        let file = FSOpenOptions::new()
            .write(o.write)
            .create(o.create)
            .read(o.read)
            .append(o.append)
            .truncate(o.truncate)
            .open(&self.full_path)
            .await?;
        Ok(PhysicalFile(file))
    }

    async fn read_dir(&self) -> Result<PhysicalReadDir> {
        let inner = tokio::fs::read_dir(&self.full_path).await?;
        Ok(PhysicalReadDir {
            inner: inner,
            root: self.root.clone(),
        })
    }

    // fn create(&self, options: OpenOptions) -> Result<File> {
    //     File::create(&self.full_path)
    // }

    // fn append(&self) -> Result<File> {
    //     FSOpenOptions::new()
    //         .write(true)
    //         .append(true)
    //         .open(&self.full_path)
    // }

    async fn mkdir(&self) -> Result<()> {
        fs::create_dir_all(&self.full_path).await?;
        Ok(())
    }

    async fn rm(&self) -> Result<()> {
        if self.full_path.is_dir() {
            fs::remove_dir(&self.full_path).await
        } else {
            fs::remove_file(&self.full_path).await
        }
    }

    async fn rm_all(&self) -> Result<()> {
        if self.full_path.is_dir() {
            fs::remove_dir_all(&self.full_path).await
        } else {
            fs::remove_file(&self.full_path).await
        }
    }
}

impl Debug for PhysicalPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "PhysicalPath<Root={:?}, Path={:?}, FullPath={:?}>",
            self.root, self.path, self.full_path
        )
    }
}

#[pin_project]
#[derive(Debug)]
pub struct PhysicalReadDir {
    #[pin]
    inner: ReadDir,
    root: Arc<PathBuf>,
}

impl Stream for PhysicalReadDir {
    type Item = Result<PhysicalPath>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.inner.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(s))) => {
                let fullp = s.path();
                let path = fullp
                    .to_str()
                    .unwrap()
                    .replace(this.root.to_str().unwrap(), "");

                Poll::Ready(Some(Ok(PhysicalPath {
                    root: this.root.clone(),
                    full_path: fullp,
                    path: path,
                })))
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Result};
    use std::path::PathBuf;

    use super::*;
    use super::{OpenOptions, VPath};
    use futures_util::io::AsyncReadExt;
    use futures_util::StreamExt;

    #[tokio::test]
    async fn to_string() {
        let vfs = PhysicalFS::new(".").unwrap();
        let path = vfs.path("./src/boxed.rs");
        assert_eq!(
            path.to_string(),
            std::borrow::Cow::Borrowed("/src/boxed.rs")
        );
    }

    #[tokio::test]
    async fn read_file() {
        let vfs = PhysicalFS::new(".").unwrap();
        let path = vfs.path("Cargo.toml");
        let mut file = path.open(OpenOptions::new().read(true)).await.unwrap();
        let mut string: String = "".to_owned();
        file.read_to_string(&mut string).await.unwrap();
        assert!(string.len() > 10);
        assert!(path.exists().await);
        assert!(path.metadata().await.unwrap().is_file());
        assert!(PathBuf::from(".").metadata().unwrap().is_dir());
    }
    #[tokio::test]
    async fn parent() {
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path("./src");
        let parent = vfs.path(".");
        assert_eq!(src.parent().unwrap().to_string(), parent.to_string());
        assert!(PathBuf::from("/").parent().is_none());
    }

    #[tokio::test]
    async fn parent_err() {
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path("");
        assert!(src.parent().is_none());
    }

    #[tokio::test]
    async fn read_dir() {
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path("./src");
        let _entries: Vec<Result<PhysicalPath>> = src.read_dir().await.unwrap().collect().await;
        //println!("{:#?}", entries);
    }

    #[tokio::test]
    async fn file_name() {
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path("./src/lib.rs");
        assert_eq!(src.file_name(), Some("lib.rs".to_owned()));
        assert_eq!(src.extension(), Some(".rs".to_owned()));
    }

    #[tokio::test]
    async fn resolve() {
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path("./src/test");
        let rel = src.resolve("../");
        assert_eq!(rel.to_string().as_ref(), "/src/");

        let rel = src.resolve("../../");
        assert_eq!(rel.to_string().as_ref(), "/");
    }

    // #[test]
    // fn to_path_buf() {
    //     let vfs = PhysicalFS::new(".").unwrap();
    //     let src = vfs.path("./src/lib.rs");
    //     //assert_eq!(Some(src.clone()), src.to_path_buf());
    // }
}
