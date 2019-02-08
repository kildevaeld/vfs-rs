// use super::boxed::{BReadPath, BWritePath};
#[cfg(feature = "glob")]
use super::glob::GlobWalkDirIter;
use super::traits::{ReadPath, VMetadata, WritePath};

impl<T: ?Sized> ReadPathExt for T where T: ReadPath {}

pub trait ReadPathExt: ReadPath {
    fn walk_dir(&self) -> WalkDirIter<Self> {
        WalkDirIter::new(self.clone())
    }

    #[cfg(feature = "glob")]
    fn glob_walk<S: AsRef<str>>(&self, pattern: S) -> GlobWalkDirIter<Self> {
        GlobWalkDirIter::new(self.clone(), pattern)
    }

    #[cfg(feature = "glob")]
    fn glob_walk_set<S: AsRef<str>>(&self, pattern: Vec<S>) -> GlobWalkDirIter<Self> {
        GlobWalkDirIter::new_set(self.clone(), pattern)
    }
}

pub struct WalkDirIter<P> {
    todo: Vec<P>,
}

impl<P> WalkDirIter<P> {
    pub fn new(path: P) -> WalkDirIter<P> {
        WalkDirIter { todo: vec![path] }
    }
}

impl<P> Iterator for WalkDirIter<P>
where
    P: ReadPath,
{
    type Item = P;
    // TODO: handle loops
    fn next(&mut self) -> Option<P> {
        let res = self.todo.pop();
        if let Some(ref path) = res {
            if let Ok(metadata) = path.metadata() {
                if metadata.is_dir() {
                    if let Ok(entries) = path.read_dir() {
                        for entry in entries {
                            if let Ok(child) = entry {
                                self.todo.push(child);
                            }
                        }
                    }
                }
            }
        }
        res
    }
}
