use super::boxed::*;
use super::traits::{OpenOptions, VMetadata, VFS};
use std::collections::HashMap;
use std::io::{ErrorKind, Result};

#[derive(Debug, Clone)]
struct EmptyPath;

impl BPath for EmptyPath {
    fn file_name(&self) -> Option<String> {
        None
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        None
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Box<dyn BPath> {
        Box::new(EmptyPath)
    }

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BPath>> {
        None
    }

    /// Check if the file existst
    fn exists(&self) -> bool {
        false
    }

    /// Get the file's metadata
    fn metadata(&self) -> Result<Box<dyn VMetadata>> {
        Err(ErrorKind::NotFound.into())
    }

    fn open(&self, options: OpenOptions) -> Result<Box<dyn BFile>> {
        Err(ErrorKind::NotFound.into())
    }

    fn read_dir(&self) -> Result<Box<dyn Iterator<Item = Result<Box<dyn BPath>>>>> {
        Err(ErrorKind::NotFound.into())
    }

    // fn create(&self) -> Result<Box<dyn BFile>>;
    // fn append(&self) -> Result<Box<dyn BFile>>;
    /// Create a directory at the location by this path
    fn mkdir(&self) -> Result<()> {
        Err(ErrorKind::NotFound.into())
    }
    /// Remove a file
    fn rm(&self) -> Result<()> {
        Err(ErrorKind::PermissionDenied.into())
    }
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> Result<()> {
        Err(ErrorKind::PermissionDenied.into())
    }
    fn box_clone(&self) -> Box<dyn BPath> {
        Box::new(self.clone())
    }
    fn to_string(&self) -> std::borrow::Cow<str> {
        std::borrow::Cow::default()
    }
}

pub struct Composite {
    mounts: HashMap<String, Box<dyn BVFS>>,
}

impl Composite {
    pub fn new() -> Composite {
        Composite {
            mounts: HashMap::default(),
        }
    }

    pub fn mount<S: AsRef<str>, V: VFS + 'static>(mut self, name: S, mount: V) -> Self {
        self.mounts
            .insert(name.as_ref().to_string(), vfs_box(mount));
        self
    }
}

impl VFS for Composite {
    type Path = Box<dyn BPath>;
    fn path(&self, path: &str) -> Self::Path {
        let split = path.split("/");
        let first = path.split("/").next();
    
        match first {
            Some(s) => match self.mounts.get(s) {
                Some(p) => p.path(split.collect::<String>().as_str()),
                None => Box::new(EmptyPath),
            },
            None => Box::new(EmptyPath),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::memory::*;
    use super::super::traits::*;
    use super::*;
    use std::io::Write;

    #[test]
    fn test_composite() {
        let mut m1 = MemoryFS::new();
        let mut f = m1.path("/test.txt").create().unwrap();
        f.write(b"Hello, World!");
        f.flush();
        let m2 = MemoryFS::new();

        let com = Composite::new().mount("app1", m1).mount("app2", m2);

        assert!(!com.path("test/mig").exists());
        assert!(com.path("app1/test.txt").exists());
    }

}
