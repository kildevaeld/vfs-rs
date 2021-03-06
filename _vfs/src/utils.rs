#[cfg(feature = "glob")]
use super::glob::GlobWalkDirIter;
use super::traits::{ VMetadata, VPath, VFS, OpenOptions};
use crossbeam;
use crossbeam::channel::bounded;
use std::io;

impl<T: ?Sized> VPathExt for T where T: VPath {}

pub trait VPathExt: VPath + Sized {
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

// fn noop<P: VPath>(path: &P) -> bool {
//     true
// }

pub struct WalkDirIter<P> {
    todo: Vec<P>,
    f: Box<dyn (Fn(&P) -> bool) + Send>,
}

impl<P> WalkDirIter<P> {
    pub fn new(path: P) -> WalkDirIter<P> {
        WalkDirIter {
            todo: vec![path],
            f: Box::new(|_| true),
        }
    }

    pub fn from<F: (Fn(&P) -> bool) + 'static + Send>(path: P, f: F) -> WalkDirIter<P> {
        WalkDirIter {
            todo: vec![path],
            f: Box::new(f),
        }
    }
}

impl<P> Iterator for WalkDirIter<P>
where
    P: VPath,
{
    type Item = P;
    // TODO: handle loops
    fn next(&mut self) -> Option<P> {
        let res = self.todo.pop();
        if let Some(ref path) = res {
            if let Ok(metadata) = path.metadata() {
                if metadata.is_dir() && (path.to_string() == "" || (self.f)(path)) {
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

enum Msg<P, F> {
    File(P, F),
    Dir(P),
    //Err(io::Error),
}

pub fn copy<S, P, D: ?Sized>(source: S, dest: &D)
where
    S: Iterator<Item = P> + Send,
    P: VPath,
    D: VFS + Send + Sync,
    // <D as VFS>::Path: VPath,
{
    crossbeam::scope(|scope| {
        let (sx, rx) = bounded(10);
        scope.spawn(move |_| {
            for p in source {
                let meta = match p.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let msg = if meta.is_dir() {
                    Msg::Dir(p)
                } else if meta.is_file() {
                    if let Some(parent) = p.parent() {
                        sx.send(Msg::Dir(parent)).unwrap();
                    }
                    let file = p.open(OpenOptions::new().read(true)).unwrap();
                    Msg::File(p, file)
                } else {
                    continue;
                };

                sx.send(msg).unwrap();
            }
        });
        scope.spawn(move |_| loop {
            let mut msg = match rx.recv() {
                Ok(m) => m,
                Err(_) => return,
            };

            let ret = match &mut msg {
                Msg::Dir(path) => {
                    let path = dest.path(&path.to_string());
                    if path.exists() {
                        continue;
                    }
                    path.mkdir()
                }
                Msg::File(path, reader) => {
                    let path = dest.path(&path.to_string());
                    let mut file = path.open(OpenOptions::new().create(true)).unwrap();
                    io::copy(reader, &mut file).map(|_| ())
                }
            };
            if ret.is_err() {}
        });
    })
    .unwrap();
}
