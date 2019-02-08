#[cfg(feature = "glob")]
use super::glob::GlobWalkDirIter;
use super::traits::{ReadPath, VMetadata, WritePath, VFS};
use crossbeam;
use crossbeam::channel::{bounded, Receiver, Sender};
use std::io;
use std::thread;

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

pub fn copy<S, P, D>(source: S, dest: D)
where
    S: Iterator<Item = P> + Send,
    P: ReadPath,
    D: VFS + Send + Sync,
    <D as VFS>::Path: WritePath,
{
    crossbeam::scope(|scope| {
        let (sx, rx) = bounded(10);
        scope.spawn(move |_| {
            for p in source {
                let meta = match p.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                if !meta.is_file() {
                    continue;
                }

                let file = p.open().unwrap();

                sx.send((p, file)).unwrap();
            }
        });
        scope.spawn(move |_| loop {
            let (path, mut reader) = match rx.recv() {
                Ok(m) => m,
                Err(_) => return,
            };
            let path = dest.path(&path.to_string());
            let mut file = path.create().unwrap();
            io::copy(&mut reader, &mut file).unwrap();
        });
    })
    .unwrap();
}
