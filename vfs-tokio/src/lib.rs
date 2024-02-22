// use async_trait::async_trait;
use async_compat::Compat;
use futures_core::{ready, Stream};
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};
use relative_path::RelativePathBuf;
use std::{
    fmt::{self, Debug},
    io::{self},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::fs::{canonicalize, File, OpenOptions as FSOpenOptions};
use vfs::{
    async_trait,
    error::{ErrorKind, Result},
    Error, FileType, Metadata, OpenOptions, SeekFrom, VAsyncFS, VAsyncFile, VAsyncPath,
};

pin_project_lite::pin_project! {
    pub struct PhysicalFile {
        #[pin]
        file: Compat<File>
    }

}

impl VAsyncFile for PhysicalFile {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        self.project()
            .file
            .poll_read(cx, buf)
            .map_err(|err| err.into())
    }

    fn poll_seek(self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<Result<u64>> {
        self.project()
            .file
            .poll_seek(cx, pos.into())
            .map_err(|err| err.into())
    }

    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.project()
            .file
            .poll_write(cx, buf)
            .map_err(|err| err.into())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().file.poll_flush(cx).map_err(|err| err.into())
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().file.poll_close(cx).map_err(|err| err.into())
    }
}

/// A "physical" file system implementation using the underlying OS file system
#[derive(Debug, Clone)]
pub struct PhysicalFS {
    root: Arc<PathBuf>,
}

impl PhysicalFS {
    pub async fn new<S: AsRef<Path>>(path: S) -> Result<PhysicalFS> {
        let path = path.as_ref();
        let meta = path.metadata()?;
        if !meta.is_dir() {}

        let abs = canonicalize(path).await?;

        Ok(PhysicalFS {
            root: Arc::new(abs),
        })
    }

    pub fn root(&self) -> &Path {
        self.root.as_ref()
    }
}

pub struct PhysicalMetadata(Metadata);

impl VAsyncFS for PhysicalFS {
    type Path = PhysicalPath;

    fn path(&self, path: &str) -> Result<PhysicalPath> {
        let path = RelativePathBuf::from(path).normalize();

        let fullpath = path.to_logical_path(self.root.as_path());

        if !fullpath.starts_with(self.root.as_path()) {
            return Err(Error::new_const(
                ErrorKind::PermissionDenied,
                "path out of bounds",
            ));
        }

        // if path.is_empty() || path == "." {
        //     return PhysicalPath {
        //         root: self.root.clone(),
        //         path: "".to_owned(),
        //         full_path: self.root.as_path().to_path_buf(),
        //     };
        // } else if path == ".." {
        //     panic!("invalid path");
        // }
        // let fp = pathutils::resolve(&self.root.to_string_lossy(), &path).unwrap();
        // if path.chars().nth(0).unwrap() == '/' {
        //     return PhysicalPath {
        //         root: self.root.clone(),
        //         path: path.to_string(),
        //         full_path: PathBuf::from(fp),
        //     };
        // }
        // let path = pathutils::resolve("/", path).unwrap();

        Ok(PhysicalPath {
            root: self.root.clone(),
            path,
            fullpath,
        })
    }

    fn from_path(path: &Self::Path) -> Result<Self> {
        Ok(PhysicalFS {
            root: path.fullpath.clone().into(),
        })
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalPath {
    root: Arc<PathBuf>,
    path: RelativePathBuf,
    fullpath: PathBuf,
}

impl PhysicalPath {
    pub fn path(&self) -> &Path {
        &self.fullpath
    }
}

#[async_trait]
impl VAsyncPath for PhysicalPath {
    type FS = PhysicalFS;
    type File = PhysicalFile;
    type ReadDir = PhysicalReadDir;

    fn parent(&self) -> Option<Self> {
        match self.path.parent() {
            Some(path) => {
                let fullpath = path.to_logical_path(self.root.as_ref());

                if !fullpath.starts_with(self.root.as_path()) {
                    return None;
                }

                Some(PhysicalPath {
                    path: path.to_owned(),
                    root: self.root.clone(),
                    fullpath,
                })
            }
            None => None,
        }
    }

    fn file_name(&self) -> Option<&str> {
        self.path.file_name()
    }

    fn extension(&self) -> Option<&str> {
        self.path.extension()
    }

    fn resolve(&self, path: &str) -> Result<Self> {
        let path = RelativePathBuf::from(path);
        let fullpath = path.to_logical_path(self.root.as_path());
        if !fullpath.starts_with(self.root.as_path()) {
            return Err(Error::new_const(
                ErrorKind::PermissionDenied,
                "path out of bounds",
            ));
        }

        Ok(PhysicalPath {
            root: self.root.clone(),
            path,
            fullpath,
        })
    }

    async fn exists(&self) -> bool {
        let path = self.fullpath.clone();
        tokio::task::spawn_blocking(move || path.exists())
            .await
            .unwrap_or_default()
    }

    async fn metadata(&self) -> Result<Metadata> {
        let meta = tokio::fs::metadata(&self.fullpath).await?;

        let meta = Metadata {
            kind: FileType::File,
            size: meta.len(),
        };

        Ok(meta)
    }

    fn to_string(&self) -> String {
        self.path.to_string()
    }

    async fn open(&self, o: OpenOptions) -> Result<Self::File> {
        let file = FSOpenOptions::new()
            .write(o.write)
            .create(o.create)
            .read(o.read)
            .append(o.append)
            .truncate(o.truncate)
            .open(&self.fullpath)
            .await?;

        Ok(PhysicalFile {
            file: Compat::new(file),
        })
    }

    async fn read_dir(&self) -> Result<PhysicalReadDir> {
        let root = self.root.clone();

        let inner = tokio::fs::read_dir(&self.fullpath).await?;

        Ok(PhysicalReadDir { inner, root: root })
    }

    async fn create_dir(&self) -> Result<()> {
        Ok(tokio::fs::create_dir_all(&self.fullpath).await?)
    }

    async fn rm(&self) -> Result<()> {
        if self.fullpath.is_dir() {
            tokio::fs::remove_dir(&self.fullpath).await?
        } else {
            tokio::fs::remove_file(&self.fullpath).await?
        }

        Ok(())
    }

    async fn rm_all(&self) -> Result<()> {
        if self.fullpath.is_dir() {
            tokio::fs::remove_dir(&self.fullpath).await?
        } else {
            tokio::fs::remove_file(&self.fullpath).await?
        }

        Ok(())
    }

    fn into_path_buf(&self) -> Option<std::path::PathBuf> {
        Some(self.fullpath.clone())
    }
}

impl Debug for PhysicalPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "PhysicalPath<Root={:?}, Path={:?}, FullPath={:?}>",
            self.root, self.path, self.fullpath
        )
    }
}

pin_project_lite::pin_project! {

    #[derive(Debug)]
    pub struct PhysicalReadDir {
        #[pin]
        inner: tokio::fs::ReadDir,
        root: Arc<PathBuf>,
    }
}

impl Stream for PhysicalReadDir {
    type Item = Result<PhysicalPath>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match ready!(this.inner.poll_next_entry(cx)) {
            Ok(None) => Poll::Ready(None),
            Ok(Some(s)) => {
                let fullpath = s.path();
                let path = pathdiff::diff_paths(&fullpath, this.root.as_path()).unwrap();
                let path = RelativePathBuf::from_path(path).expect("relative path");

                Poll::Ready(Some(Ok(PhysicalPath {
                    root: this.root.clone(),
                    fullpath,
                    path,
                })))
            }
            Err(err) => Poll::Ready(Some(Err(err.into()))),
        }
    }
}

impl<'a> TryFrom<&'a Path> for PhysicalPath {
    type Error = io::Error;
    fn try_from(path: &'a Path) -> io::Result<Self> {
        let mut path = path.to_path_buf();
        if !path.is_absolute() {
            path = std::fs::canonicalize(path)?;
        }

        if path.is_dir() {
            Ok(PhysicalPath {
                root: Arc::new(path.clone()),
                path: RelativePathBuf::from("."),
                fullpath: path,
            })
        } else {
            let filename = path.file_name().expect("filename");
            let parent = path.parent().unwrap_or_else(|| Path::new("/"));

            Ok(PhysicalPath {
                root: Arc::new(parent.to_path_buf()),
                path: RelativePathBuf::from(filename.to_str().expect("filename")),
                fullpath: path,
            })
        }
    }
}

impl TryFrom<PathBuf> for PhysicalPath {
    type Error = io::Error;
    fn try_from(path: PathBuf) -> io::Result<Self> {
        path.as_path().try_into()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use vfs::{error::Result, OpenOptions, VAsyncFileExt};

    use futures_util::StreamExt;

    #[tokio::test]
    async fn to_string() {
        let vfs = PhysicalFS::new(".").await.expect("open fs");
        let path = vfs.path("./src/boxed.rs").expect("open path");
        assert_eq!(path.to_string(), String::from("src/boxed.rs"));
    }

    #[tokio::test]
    async fn read_file() {
        let vfs = PhysicalFS::new(".").await.expect("open fs");
        let path = vfs.path("Cargo.toml").expect("open path");
        let mut file = path.open(OpenOptions::new().read(true)).await.unwrap();
        let mut string = Vec::default();
        file.read_to_end(&mut string).await.unwrap();
        assert!(string.len() > 10);
        assert!(path.exists().await);
        assert!(path.metadata().await.unwrap().is_file());
        // assert!(PathBuf::from(".").metadata().unwrap().is_dir());
    }

    #[tokio::test]
    async fn parent() {
        let vfs = PhysicalFS::new(".").await.expect("open fs");
        let src = vfs.path("./src").expect("open path");
        let parent = vfs.path(".").expect("open path");
        assert_eq!(src.parent().unwrap().to_string(), parent.to_string());
        // assert!(PathBuf::from("/").parent().is_none());
    }

    #[tokio::test]
    async fn parent_err() {
        let vfs = PhysicalFS::new(".").await.expect("open fs");
        let src = vfs.path("").expect("open path");
        assert!(src.parent().is_none());
    }

    #[tokio::test]
    async fn read_dir() {
        let vfs = PhysicalFS::new(".").await.expect("open fs");
        let src = vfs.path("./src").expect("open path");
        let _entries: Vec<Result<PhysicalPath>> = src.read_dir().await.unwrap().collect().await;
    }

    /*#[tokio::test]
    fn readfile() {
        block_on(async {
            let vfs = PhysicalFS::new(".").unwrap();
            let src = vfs.read("./src/lib.rs").await.unwrap();
        })
    }

    #[tokio::test]
    fn file_name() {
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path("./src/lib.rs");
        assert_eq!(src.file_name(), Some("lib.rs".to_owned()));
        assert_eq!(src.extension(), Some(".rs".to_owned()));
    }

    */

    /*
    #[tokio::test]
    async fn resolve() {
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path("./src/test");
        let rel = src.resolve("../");
        assert_eq!(rel.to_string().as_ref(), "/src/");

        let rel = src.resolve("../../");
        assert_eq!(rel.to_string().as_ref(), "/");
    }*/
}
