use super::boxed::*;
use super::traits::{OpenOptions, VMetadata, VFS, VPath};
use std::collections::HashMap;
use std::io::{ErrorKind, Result};
use std::sync::Arc;

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

#[derive(Debug, Clone)]
struct RootPath(String, Arc<Box<dyn BVFS>>);

impl BPath for RootPath {
    fn file_name(&self) -> Option<String> {
        Some(self.0.clone())
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        None
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Box<dyn BPath> {
        self.1.path(path)
    }

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BPath>> {
        None
    }

    /// Check if the file existst
    fn exists(&self) -> bool {
        true
    }

    /// Get the file's metadata
    fn metadata(&self) -> Result<Box<dyn VMetadata>> {
        Ok(Box::new(RootMetadata))
    }

    fn open(&self, options: OpenOptions) -> Result<Box<dyn BFile>> {
        Err(ErrorKind::InvalidInput.into())
    }

    fn read_dir(&self) -> Result<Box<dyn Iterator<Item = Result<Box<dyn BPath>>>>> {
       self.1.path("").read_dir()
    }

    // fn create(&self) -> Result<Box<dyn BFile>>;
    // fn append(&self) -> Result<Box<dyn BFile>>;
    /// Create a directory at the location by this path
    fn mkdir(&self) -> Result<()> {
        self.1.path("").mkdir()
    }
    /// Remove a file
    fn rm(&self) -> Result<()> {
       self.1.path("").rm()
    }
    /// Remove a file or directory and all its contents
    fn rm_all(&self) -> Result<()> {
       self.1.path("").rm_all()
    }
    fn box_clone(&self) -> Box<dyn BPath> {
        Box::new(self.clone())
    }
    fn to_string(&self) -> std::borrow::Cow<str> {
        std::borrow::Cow::Borrowed(&self.0)
    }
}

struct RootMetadata;

impl VMetadata for RootMetadata {
    fn is_dir(&self) -> bool {
        true
    }
    /// Returns true iff this path is a file
    fn is_file(&self) -> bool {
        false
    }
    /// Returns the length of the file at this path
    fn len(&self) -> u64 {
        0
    }
}


#[derive(Clone, Debug)]
pub struct Composite {
    mounts: HashMap<String, Arc<Box<dyn BVFS>>>,
}

impl Composite {
    pub fn new() -> Composite {
        Composite {
            mounts: HashMap::default(),
        }
    }

    pub fn mount<S: AsRef<str>, V: VFS + 'static>(mut self, name: S, mount: V) -> Self {
        self.mounts
            .insert(name.as_ref().to_string(), Arc::new(vfs_box(mount)));
        self
    }
}

impl VFS for Composite {
    type Path = Box<dyn BPath>;
    fn path(&self, path: &str) -> Self::Path {

        if path.is_empty() {
            let clone = self.clone();
            return Box::new(clone)
        }

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

impl BPath for Composite {
    

    fn file_name(&self) -> Option<String> {
        None
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        None
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Box<dyn BPath> {
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

    /// Get the parent path
    fn parent(&self) -> Option<Box<dyn BPath>> {
        None
    }

    /// Check if the file existst
    fn exists(&self) -> bool {
        true
    }

    /// Get the file's metadata
    fn metadata(&self) -> Result<Box<dyn VMetadata>> {
        Ok(Box::new(RootMetadata))
    }

    fn to_string(&self) -> std::borrow::Cow<str> {
        std::borrow::Cow::default()
    }

   

    fn open(&self, options: OpenOptions) -> Result<Box<dyn BFile>> {
        Err(ErrorKind::PermissionDenied.into())
    }
    

    fn read_dir(&self) -> Result<Box<dyn Iterator<Item = Result<Box<dyn BPath>>>>> {
        let coll = self.mounts.iter().map(|m| {
           let root = RootPath(m.0.clone(), m.1.clone());
           let b: Box<dyn BPath> = Box::new(root);
           Ok(b)
       }).collect::<Vec<_>>();

       Ok(Box::new(coll.into_iter()))
    }

    /// Create a directory at the location by this path
    fn mkdir(&self) -> Result<()> {
        Err(ErrorKind::PermissionDenied.into())
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

    #[test]
    fn test_composite_iter() {
        
    }

}
