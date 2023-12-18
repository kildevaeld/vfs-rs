use crate::{error::Error, VPath};

pub trait VFS: Send + Sync {
    type Path: VPath;
    fn path(&self, path: &str) -> Result<Self::Path, Error>;
}
