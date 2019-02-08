//! A "physical" file system implementation using the underlying OS file system

use super::traits::{ReadPath, VMetadata, VPath, WritePath, VFS};
use std::borrow::Cow;
use std::fs::{
    canonicalize, remove_dir, remove_dir_all, remove_file, DirBuilder, File, Metadata,
    OpenOptions as FSOpenOptions, ReadDir,
};
use std::io::Result;
use std::path::{Path, PathBuf};

/// A "physical" file system implementation using the underlying OS file system
pub struct PhysicalFS {
    root: PathBuf,
}

impl PhysicalFS {
    pub fn new<S: AsRef<Path>>(path: S) -> Result<PhysicalFS> {
        let path = path.as_ref();
        let meta = path.metadata()?;
        if !meta.is_dir() {}

        let abs = canonicalize(path)?;

        Ok(PhysicalFS { root: abs })
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
    // type PATH = PathBuf;
    // type FILE = File;
    // type METADATA = Metadata;
    type Path = PathBuf;

    fn path(&self, path: &str) -> PathBuf {
        if path.is_empty() {
            return self.root.clone();
        }
        if path.chars().nth(0).unwrap() == '/' {
            return self.root.join(path.chars().skip(1).collect::<String>());
        }

        self.root.join(path)
    }
}

impl VPath for PathBuf {
    type Metadata = Metadata;

    fn parent(&self) -> Option<Self> {
        match <Path>::parent(&self) {
            Some(path) => Some(path.to_path_buf()),
            None => None,
        }
    }

    fn file_name(&self) -> Option<String> {
        match <Path>::file_name(&self) {
            Some(name) => Some(name.to_string_lossy().into_owned()),
            None => None,
        }
    }

    fn extension(&self) -> Option<String> {
        match <Path>::extension(&self) {
            Some(name) => Some(name.to_string_lossy().into_owned()),
            None => None,
        }
    }

    fn resolve(&self, path: &String) -> Self {
        let mut result = self.clone();
        <PathBuf>::push(&mut result, path);
        return result;
    }

    fn exists(&self) -> bool {
        <Path>::exists(self)
    }

    fn metadata(&self) -> Result<Self::Metadata> {
        <Path>::metadata(self)
    }

    fn to_path_buf(&self) -> Option<PathBuf> {
        Some(self.clone())
    }

    fn to_string(&self) -> Cow<str> {
        <Path>::to_string_lossy(self)
    }
}

impl ReadPath for PathBuf {
    type Read = File;
    type Iterator = PhysicalReadDir;
    fn open(&self) -> Result<File> {
        File::open(&self)
    }

    fn read_dir(&self) -> Result<PhysicalReadDir> {
        <Path>::read_dir(self).map(|inner| PhysicalReadDir { inner: inner })
    }
}

impl WritePath for PathBuf {
    type Write = File;
    fn create(&self) -> Result<File> {
        File::create(&self)
    }

    fn append(&self) -> Result<File> {
        FSOpenOptions::new().write(true).append(true).open(&self)
    }

    fn mkdir(&self) -> Result<()> {
        DirBuilder::new().recursive(true).create(&self)
    }

    fn rm(&self) -> Result<()> {
        if self.is_dir() {
            remove_dir(&self)
        } else {
            remove_file(&self)
        }
    }

    fn rm_all(&self) -> Result<()> {
        if self.is_dir() {
            remove_dir_all(&self)
        } else {
            remove_file(&self)
        }
    }
}

pub struct PhysicalReadDir {
    inner: ReadDir,
}

impl Iterator for PhysicalReadDir {
    type Item = Result<PathBuf>;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|result| result.map(|entry| entry.path()))
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
        let path = PathBuf::from("Cargo.toml");
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
        let src = PathBuf::from("./src");
        let parent = PathBuf::from(".");
        assert_eq!(src.parent().unwrap().to_string(), parent.to_string());
        assert!(PathBuf::from("/").parent().is_none());
    }

    #[test]
    fn read_dir() {
        let src = PathBuf::from("./src");
        let entries: Vec<Result<PathBuf>> = src.read_dir().unwrap().collect();
        println!("{:#?}", entries);
    }

    #[test]
    fn file_name() {
        let src = PathBuf::from("./src/lib.rs");
        assert_eq!(src.file_name(), Some("lib.rs".to_owned()));
        assert_eq!(src.extension(), Some("rs".to_owned()));
    }

    #[test]
    fn to_path_buf() {
        let src = PathBuf::from("./src/lib.rs");
        assert_eq!(Some(src.clone()), src.to_path_buf());
    }

}
