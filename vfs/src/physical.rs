use super::traits::{OpenOptions, VFile, VMetadata, VPath, VFS};
use async_fs::{File, OpenOptions as FSOpenOptions};
use async_trait::async_trait;
use blocking::unblock;
use futures_lite::Stream;
use pathutils;
use pin_project::pin_project;
use std::borrow::Cow;
use std::fmt::{self, Debug};
use std::fs::{canonicalize, Metadata};
use std::io::Result;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

impl VFile for File {}

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

    pub fn root(&self) -> &PathBuf {
        self.root.as_ref()
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
        let path = pathutils::resolve("/", path).unwrap();

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
    type File = File;
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
            Some(name) => Some(name.to_string()),
            None => None,
        }
    }

    fn extension(&self) -> Option<String> {
        match pathutils::extname(&self.path) {
            Some(name) => Some(name.to_string()),
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
        let path = self.full_path.clone();
        unblock(move || path.exists()).await
    }

    async fn metadata(&self) -> Result<Self::Metadata> {
        let path = self.full_path.clone();
        unblock(move || path.metadata()).await
    }

    fn to_string(&self) -> Cow<str> {
        Cow::from(&self.path)
    }

    async fn open(&self, o: OpenOptions) -> Result<File> {
        let file = FSOpenOptions::new()
            .write(o.write)
            .create(o.create)
            .read(o.read)
            .append(o.append)
            .truncate(o.truncate)
            .open(&self.full_path)
            .await?;

        Ok(file)
    }

    async fn read_dir(&self) -> Result<PhysicalReadDir> {
        let root = self.root.clone();

        let inner = async_fs::read_dir(&self.full_path).await?;

        Ok(PhysicalReadDir { inner, root: root })
    }

    async fn create_dir(&self) -> Result<()> {
        async_fs::create_dir_all(self.full_path.clone()).await
    }

    async fn rm(&self) -> Result<()> {
        if self.full_path.is_dir() {
            async_fs::remove_dir(self.full_path.clone()).await
        } else {
            async_fs::remove_file(self.full_path.clone()).await
        }
    }

    async fn rm_all(&self) -> Result<()> {
        if self.full_path.is_dir() {
            async_fs::remove_dir_all(self.full_path.clone()).await
        } else {
            async_fs::remove_file(self.full_path.clone()).await
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
    inner: async_fs::ReadDir,
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
    use std::io::Result;
    use std::path::PathBuf;

    use super::*;
    use super::{OpenOptions, VPath};
    use crate::VFSExt;
    use futures::{executor::block_on, io::AsyncReadExt, StreamExt};

    #[test]
    fn to_string() {
        let vfs = PhysicalFS::new(".").unwrap();
        let path = vfs.path("./src/boxed.rs");
        assert_eq!(
            path.to_string(),
            std::borrow::Cow::Borrowed("/src/boxed.rs")
        );
    }

    #[test]
    fn read_file() {
        block_on(async {
            let vfs = PhysicalFS::new(".").unwrap();
            let path = vfs.path("Cargo.toml");
            let mut file = path.open(OpenOptions::new().read(true)).await.unwrap();
            let mut string: String = "".to_owned();
            file.read_to_string(&mut string).await.unwrap();
            assert!(string.len() > 10);
            assert!(path.exists().await);
            assert!(path.metadata().await.unwrap().is_file());
            assert!(PathBuf::from(".").metadata().unwrap().is_dir());
        });
    }
    #[test]
    fn parent() {
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path("./src");
        let parent = vfs.path(".");
        assert_eq!(src.parent().unwrap().to_string(), parent.to_string());
        assert!(PathBuf::from("/").parent().is_none());
    }

    #[test]
    fn parent_err() {
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path("");
        assert!(src.parent().is_none());
    }

    #[test]
    fn read_dir() {
        block_on(async {
            let vfs = PhysicalFS::new(".").unwrap();
            let src = vfs.path("./src");
            let _entries: Vec<Result<PhysicalPath>> = src.read_dir().await.unwrap().collect().await;
        })
    }

    #[test]
    fn readfile() {
        block_on(async {
            let vfs = PhysicalFS::new(".").unwrap();
            let src = vfs.read("./src/lib.rs").await.unwrap();
        })
    }

    #[test]
    fn file_name() {
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path("./src/lib.rs");
        assert_eq!(src.file_name(), Some("lib.rs".to_owned()));
        assert_eq!(src.extension(), Some(".rs".to_owned()));
    }

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
