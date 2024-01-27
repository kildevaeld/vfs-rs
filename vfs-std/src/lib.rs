use std::{
    io::{Read, Seek, Write},
    os::unix::fs::MetadataExt,
    path::PathBuf,
};

use pathdiff::diff_paths;
use relative_path::RelativePathBuf;
use vfs::{Error, FileType, Metadata, OpenOptions, SeekFrom, VFile, VPath, VFS};

pub struct File(std::fs::File);

impl VFile for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        Ok(self.0.read(buf)?)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        Ok(self.0.write(buf)?)
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(self.0.flush()?)
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        Ok(self.0.seek(pos.into())?)
    }

    fn close(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Path {
    root: PathBuf,
    path: relative_path::RelativePathBuf,
}

impl Path {
    pub fn fullpath(&self) -> PathBuf {
        self.path.to_logical_path(&self.root)
    }
}

impl VPath for Path {
    type FS = Fs;
    type File = File;

    type ReadDir = ReadDir;

    fn file_name(&self) -> Option<&str> {
        self.path.file_name()
    }

    fn extension(&self) -> Option<&str> {
        self.path.extension()
    }

    fn resolve(&self, path: &str) -> Result<Self, Error> {
        let path = self.path.join_normalized(path);

        // if !path.starts_with(&self.path) {
        //     todo!("parent: {:?} {:?}", self.path, path)
        // }
        println!("{:?} {:?}", self.path, path);
        Ok(Path {
            root: self.root.clone(),
            path,
        })
    }

    fn parent(&self) -> Option<Self> {
        self.path.parent().map(|path| Path {
            root: self.root.clone(),
            path: path.to_relative_path_buf(),
        })
    }

    fn exists(&self) -> bool {
        self.path.to_path(&self.root).exists()
    }

    fn metadata(&self) -> Result<Metadata, Error> {
        let meta = self.path.to_path(&self.root).metadata()?;
        let ty = meta.file_type();
        let ty = if ty.is_dir() {
            FileType::Dir
        } else if ty.is_file() {
            FileType::File
        } else {
            panic!("invalid type")
        };
        Ok(Metadata {
            size: meta.size(),
            kind: ty,
        })
    }

    fn to_string(&self) -> String {
        self.path.to_string()
    }

    fn open(&self, options: OpenOptions) -> Result<Self::File, Error> {
        let opts: std::fs::OpenOptions = options.into();
        let file = opts.open(self.fullpath())?;
        Ok(File(file))
    }

    fn read_dir(&self) -> Result<Self::ReadDir, Error> {
        if !self.fullpath().is_dir() {
            panic!()
        }

        let fullpath = self.fullpath();

        Ok(ReadDir {
            iter: fullpath.read_dir()?,
            base: self.path.clone(),
            root: self.root.clone(),
        })
    }

    fn create_dir(&self) -> Result<(), Error> {
        Ok(std::fs::create_dir_all(self.fullpath())?)
    }

    fn rm(&self) -> Result<(), Error> {
        let meta = self.metadata()?;
        if meta.is_dir() {
            std::fs::remove_dir(self.fullpath())?
        } else {
            std::fs::remove_file(self.fullpath())?
        }

        Ok(())
    }

    fn rm_all(&self) -> Result<(), Error> {
        let meta = self.metadata()?;
        if meta.is_dir() {
            std::fs::remove_dir_all(self.fullpath())?
        } else {
            std::fs::remove_file(self.fullpath())?
        }

        Ok(())
    }
}

pub struct ReadDir {
    iter: std::fs::ReadDir,
    base: RelativePathBuf,
    root: PathBuf,
}

impl Iterator for ReadDir {
    type Item = Result<Path, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(next) = self.iter.next() else {
            return None;
        };

        let next = match next {
            Ok(ret) => ret,
            Err(err) => return Some(Err(err.into())),
        };

        let diff = diff_paths(next.path(), &self.root).unwrap();

        match RelativePathBuf::from_path(diff) {
            Ok(ret) => Some(Ok(Path {
                path: ret,
                root: self.root.clone(),
            })),
            Err(err) => {
                todo!("err {err:?}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Fs(PathBuf);

impl Fs {
    pub fn new(path: impl AsRef<std::path::Path>) -> Result<Fs, Error> {
        let path = path.as_ref().canonicalize()?;
        Ok(Fs(path))
    }
}

impl VFS for Fs {
    type Path = Path;
    fn path(&self, path: &str) -> Result<Self::Path, Error> {
        let mut path = RelativePathBuf::from(path);

        if !path.is_normalized() {
            path = path.normalize();
        }

        let fullp = path.to_logical_path(&self.0);

        if !fullp.starts_with(&self.0) {
            panic!("invalid base")
        }

        Ok(Path {
            path,
            root: self.0.clone(),
        })
    }

    fn from_path(path: &Self::Path) -> Result<Self, Error> {
        Ok(Fs(path.fullpath()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let fs = Fs::new(".").expect("root");

        let path = fs.path(".").expect("path");

        println!("{:?}", path.resolve("../Cargo.toml").expect("").fullpath());
    }

    #[test]
    fn readdir() {
        let fs = Fs::new(".").expect("root");

        let path = fs.path(".").expect("path");

        let mut read = path.read_dir().expect("readdir");

        println!("{:?}", read.next());
        println!("{:?}", read.next());
    }
}
