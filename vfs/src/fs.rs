use crate::{Error, VPath};

pub trait VFS: Sized {
    type Path: VPath<FS = Self>;

    fn path(&self, path: &str) -> Result<Self::Path, Error>;
}
