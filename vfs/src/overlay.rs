use super::traits::*;
use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt;
use std::io::{Error, ErrorKind, Read, Result};
use std::path::PathBuf;

pub trait Overlay: VFS + Sized
where
    <Self as VFS>::Path: ReadPath,
{
    fn merge<T: VFS>(self, overlay: T) -> Merge<Self, T>
    where
        <T as VFS>::Path: ReadPath;
}

impl<T> Overlay for T
where
    T: VFS,
    <Self as VFS>::Path: ReadPath,
{
    fn merge<O: VFS>(self, overlay: O) -> Merge<Self, O>
    where
        <O as VFS>::Path: ReadPath,
    {
        Merge::new(self, overlay)
    }
}

pub struct Merge<S, P> {
    s: S,
    p: P,
}

impl<S, P> Merge<S, P> {
    pub fn new(s: S, p: P) -> Merge<S, P> {
        Merge { s, p }
    }
}

impl<S, P> VFS for Merge<S, P>
where
    S: VFS,
    P: VFS,
{
    type Path = MergePath<S::Path, P::Path>;

    fn path(&self, path: &str) -> Self::Path {
        MergePath::new(Multi {
            s: self.s.path(&path),
            p: self.p.path(&path),
        })
    }
}

pub(super) enum OneOf<S, P> {
    First(S),
    Second(P),
    // None,
}

impl<S: Clone, P: Clone> Clone for OneOf<S, P> {
    fn clone(&self) -> Self {
        match self {
            OneOf::First(m) => OneOf::First(m.clone()),
            OneOf::Second(m) => OneOf::Second(m.clone()),
            // OneOf::None => OneOf::None,
        }
    }
}

#[derive(Clone)]
pub(super) struct Multi<S: Clone, P: Clone> {
    s: S,
    p: P,
}

#[derive(Clone)]
pub struct MergePath<S: Clone, P: Clone> {
    inner: Multi<S, P>,
}

impl<S: Clone, P: Clone> MergePath<S, P> {
    pub(super) fn new(one: Multi<S, P>) -> MergePath<S, P> {
        MergePath { inner: one }
    }
}

impl<S, P> VPath for MergePath<S, P>
where
    S: VPath,
    P: VPath,
{
    type Metadata = MergeMetadata<S, P>;

    fn file_name(&self) -> Option<String> {
        match self.inner.p.file_name() {
            Some(m) => Some(m),
            None => self.inner.s.file_name(),
        }
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        match self.inner.p.extension() {
            Some(m) => Some(m),
            None => self.inner.s.extension(),
        }
    }

    /// append a segment to this path
    fn resolve(&self, path: &String) -> Self {
        let p1 = self.inner.s.resolve(&path);
        let p2 = self.inner.p.resolve(&path);
        MergePath::new(Multi { s: p1, p: p2 })
    }

    /// Get the parent path
    fn parent(&self) -> Option<Self> {
        let p1 = match self.inner.s.parent() {
            None => return None,
            Some(m) => m,
        };

        let p2 = match self.inner.p.parent() {
            None => return None,
            Some(m) => m,
        };

        Some(MergePath::new(Multi { s: p1, p: p2 }))
    }

    /// Check if the file existst
    fn exists(&self) -> bool {
        if self.inner.p.exists() {
            return true;
        }
        self.inner.s.exists()
    }

    /// Get the file's metadata
    fn metadata(&self) -> Result<Self::Metadata> {
        if let Ok(m) = self.inner.p.metadata() {
            return Ok(MergeMetadata::new(
                OneOf::<S::Metadata, P::Metadata>::Second(m),
            ));
        }
        match self.inner.s.metadata() {
            Ok(m) => Ok(MergeMetadata::new(
                OneOf::<S::Metadata, P::Metadata>::First(m),
            )),
            Err(e) => Err(e),
        }
    }

    fn to_string(&self) -> Cow<str> {
        self.inner.s.to_string()
    }

    fn to_path_buf(&self) -> Option<PathBuf> {
        if let Some(i) = self.inner.p.to_path_buf() {
            return Some(i);
        }
        self.inner.s.to_path_buf()
    }
}

impl<S, P> fmt::Debug for MergePath<S, P>
where
    S: VPath,
    P: VPath,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <fmt::Debug>::fmt(&self.inner.s, f)?;
        <fmt::Debug>::fmt(&self.inner.p, f)
    }
}

impl<S, P> ReadPath for MergePath<S, P>
where
    S: ReadPath,
    P: ReadPath,
{
    type Read = MergeFile<S::Read, P::Read>;
    type Iterator = MergeIterator<S, P>;

    fn open(&self) -> Result<Self::Read> {
        if self.inner.p.exists() {
            self.inner.p.open().map(|m| MergeFile {
                inner: OneOf::<S::Read, P::Read>::Second(m),
            })
        } else {
            self.inner.s.open().map(|m| MergeFile {
                inner: OneOf::<S::Read, P::Read>::First(m),
            })
        }
    }

    fn read_dir(&self) -> Result<Self::Iterator> {
        let i1 = self.inner.s.read_dir();
        let i2 = self.inner.p.read_dir();
        if i1.is_err() && i2.is_err() {
            return Err(Error::from(ErrorKind::NotFound));
        }
        Ok(MergeIterator {
            s: self.inner.s.clone(),
            si: match i1 {
                Ok(m) => Some(m),
                Err(_) => None,
            },
            p: self.inner.p.clone(),
            pi: match i2 {
                Ok(m) => Some(m),
                Err(_) => None,
            },
            seen: HashSet::new(),
        })
    }
}

pub struct MergeFile<S, P> {
    inner: OneOf<S, P>,
}

impl<S, P> Read for MergeFile<S, P>
where
    S: Read,
    P: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match &mut self.inner {
            OneOf::First(file) => file.read(buf),
            OneOf::Second(file) => file.read(buf),
            // OneOf::None => Err(Error::from(ErrorKind::NotFound)),
        }
    }
}

pub struct MergeIterator<S, P>
where
    S: ReadPath,
    P: ReadPath,
{
    s: S,
    si: Option<S::Iterator>,
    p: P,
    pi: Option<P::Iterator>,
    seen: HashSet<String>,
}

impl<S, P> Iterator for MergeIterator<S, P>
where
    S: ReadPath,
    P: ReadPath,
{
    type Item = Result<MergePath<S, P>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pi.is_some() {
            match self.pi.as_mut().unwrap().next() {
                Some(m) => match m {
                    Ok(m) => {
                        let fl = m.to_string();
                        let p = fl.replace(&self.p.to_string().to_string(), "");
                        self.seen.insert(String::from(fl));
                        return Some(Ok(MergePath::new(Multi {
                            s: self.s.resolve(&p),
                            p: m,
                        })));
                    }
                    Err(e) => return Some(Err(e)),
                },
                None => {
                    self.pi = None;
                }
            };
        }

        if self.si.is_some() {
            while self.si.is_some() {
                let m = match self.si.as_mut().unwrap().next() {
                    Some(m) => match m {
                        Ok(m) => {
                            let fl = m.to_string();
                            let p = fl.replace(&self.s.to_string().to_string(), "");
                            if self.seen.contains(&String::from(fl)) {
                                None
                            } else {
                                Some(Ok(MergePath::new(Multi {
                                    p: self.p.resolve(&p),
                                    s: m,
                                })))
                            }
                        }
                        Err(e) => Some(Err(e)),
                    },
                    None => {
                        self.si = None;
                        None
                    }
                };

                if m.is_some() {
                    return m;
                }
            }
            None
        } else {
            None
        }
    }
}

pub struct MergeMetadata<S, P>
where
    S: VPath,
    P: VPath,
{
    inner: OneOf<S::Metadata, P::Metadata>,
}

impl<S, P> MergeMetadata<S, P>
where
    S: VPath,
    P: VPath,
{
    pub(super) fn new(inner: OneOf<S::Metadata, P::Metadata>) -> MergeMetadata<S, P> {
        MergeMetadata { inner }
    }
}

impl<S, P> VMetadata for MergeMetadata<S, P>
where
    S: VPath,
    P: VPath,
{
    fn is_dir(&self) -> bool {
        match &self.inner {
            OneOf::First(m) => m.is_dir(),
            OneOf::Second(m) => m.is_dir(),
            // OneOf::None => false,
        }
    }
    /// Returns true iff this path is a file
    fn is_file(&self) -> bool {
        match &self.inner {
            OneOf::First(m) => m.is_file(),
            OneOf::Second(m) => m.is_file(),
            // OneOf::None => false,
        }
    }
    /// Returns the length of the file at this path
    fn len(&self) -> u64 {
        match &self.inner {
            OneOf::First(m) => m.len(),
            OneOf::Second(m) => m.len(),
            // OneOf::None => 0,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::super::memory::*;
    use super::*;
    use std::io::Write;

    #[test]
    fn test_overlay() {
        let m1 = MemoryFS::new();
        let mut f = m1.path("/test.txt").create().unwrap();
        f.write(b"Hello, World!");
        f.flush();

        let m2 = MemoryFS::new();
        let mut f = m1.path("/test2.txt").create().unwrap();
        f.write(b"Hello, World!");
        f.flush();

        let overlay = m1.merge(m2);

        assert!(overlay.path("/test.txt").exists());
        assert!(overlay.path("/test2.txt").exists());
    }
}
