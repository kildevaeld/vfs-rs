use super::traits::{ReadPath, VMetadata, VPath, WritePath, VFS};
use pathutils;
use std::borrow::Cow;
use std::fmt::{self, Debug};
use std::fs::{
    canonicalize, remove_dir, remove_dir_all, remove_file, DirBuilder, File, Metadata,
    OpenOptions as FSOpenOptions, ReadDir,
};
use std::io::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A "physical" file system implementation using the underlying OS file system
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
        let path = format!("/{}", path);

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

impl VPath for PhysicalPath {
    type Metadata = Metadata;

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
            pathutils::resolve(self.root.as_path().to_string_lossy().as_ref(), path).unwrap();
        let path = full_path.replace(self.root.to_str().unwrap(), "");
        return PhysicalPath {
            path: path,
            root: self.root.clone(),
            full_path: PathBuf::from(full_path),
        };
    }

    fn exists(&self) -> bool {
        self.full_path.exists()
    }

    fn metadata(&self) -> Result<Self::Metadata> {
        self.full_path.metadata()
    }

    fn to_path_buf(&self) -> Option<PathBuf> {
        Some(self.full_path.clone())
    }

    fn to_string(&self) -> Cow<str> {
        Cow::from(&self.path)
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

impl ReadPath for PhysicalPath {
    type Read = File;
    type Iterator = PhysicalReadDir;
    fn open(&self) -> Result<File> {
        File::open(&self.full_path)
    }

    fn read_dir(&self) -> Result<PhysicalReadDir> {
        self.full_path.read_dir().map(|inner| PhysicalReadDir {
            inner: inner,
            root: self.root.clone(),
        })
    }
}

impl WritePath for PhysicalPath {
    type Write = File;
    fn create(&self) -> Result<File> {
        File::create(&self.full_path)
    }

    fn append(&self) -> Result<File> {
        FSOpenOptions::new()
            .write(true)
            .append(true)
            .open(&self.full_path)
    }

    fn mkdir(&self) -> Result<()> {
        DirBuilder::new().recursive(true).create(&self.full_path)
    }

    fn rm(&self) -> Result<()> {
        if self.full_path.is_dir() {
            remove_dir(&self.full_path)
        } else {
            remove_file(&self.full_path)
        }
    }

    fn rm_all(&self) -> Result<()> {
        if self.full_path.is_dir() {
            remove_dir_all(&self.full_path)
        } else {
            remove_file(&self.full_path)
        }
    }
}

pub struct PhysicalReadDir {
    inner: ReadDir,
    root: Arc<PathBuf>,
}

impl Iterator for PhysicalReadDir {
    type Item = Result<PhysicalPath>;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|result| {
            result.map(|entry| {
                let fullp = entry.path();
                let path = fullp
                    .to_str()
                    .unwrap()
                    .replace(self.root.to_str().unwrap(), "");

                PhysicalPath {
                    root: self.root.clone(),
                    full_path: fullp,
                    path: path,
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Result};
    use std::path::PathBuf;

    use super::VPath;
    use super::*;
    #[test]
    fn read_file() {
        let vfs = PhysicalFS::new(".").unwrap();
        let path = vfs.path("Cargo.toml");
        let mut file = path.open().unwrap();
        let mut string: String = "".to_owned();
        file.read_to_string(&mut string).unwrap();
        assert!(string.len() > 10);
        assert!(path.exists());
        assert!(path.metadata().unwrap().is_file());
        assert!(PathBuf::from(".").metadata().unwrap().is_dir());
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
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path("./src");
        let entries: Vec<Result<PhysicalPath>> = src.read_dir().unwrap().collect();
        println!("{:#?}", entries);
    }

    #[test]
    fn file_name() {
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path("./src/lib.rs");
        assert_eq!(src.file_name(), Some("lib.rs".to_owned()));
        assert_eq!(src.extension(), Some("rs".to_owned()));
    }

    // #[test]
    // fn to_path_buf() {
    //     let vfs = PhysicalFS::new(".").unwrap();
    //     let src = vfs.path("./src/lib.rs");
    //     //assert_eq!(Some(src.clone()), src.to_path_buf());
    // }

}
