use super::traits::*;
use std::borrow::Cow;
use std::collections::HashSet;
use std::io::{ErrorKind, Read, Result};
use std::path::PathBuf;

pub trait Overlay: VFS + Sized
where
    <Self as VFS>::Path: VPath,
{
    fn merge<T: VFS>(self, overlay: T) -> Merge<Self, T>
    where
        <T as VFS>::Path: VPath;
}

impl<T> Overlay for T
where
    T: VFS,
    <Self as VFS>::Path: VPath,
{
    fn merge<O: VFS>(self, overlay: O) -> Merge<Self, O>
    where
        <O as VFS>::Path: VPath,
    {
        Merge::new(self, overlay)
    }
}

#[derive(Debug)]
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
    type Path = OneOrTwo<S::Path, P::Path>;

    fn path(&self, path: &str) -> Self::Path {
        OneOrTwo::Two(self.s.path(path), self.p.path(path))
    }
}

#[derive(Debug)]
pub enum OneOf<S, P> {
    First(S),
    Second(P),
}

impl<S: Clone, P: Clone> Clone for OneOf<S, P> {
    fn clone(&self) -> Self {
        match self {
            OneOf::First(m) => OneOf::First(m.clone()),
            OneOf::Second(m) => OneOf::Second(m.clone()),
        }
    }
}

impl<S, P> VMetadata for OneOf<S, P>
where
    S: VMetadata,
    P: VMetadata,
{
    fn is_dir(&self) -> bool {
        match self {
            OneOf::First(m) => m.is_dir(),
            OneOf::Second(m) => m.is_dir(),
        }
    }
    /// Returns true iff this path is a file
    fn is_file(&self) -> bool {
        match self {
            OneOf::First(m) => m.is_file(),
            OneOf::Second(m) => m.is_file(),
        }
    }
    /// Returns the length of the file at this path
    fn len(&self) -> u64 {
        match self {
            OneOf::First(m) => m.len(),
            OneOf::Second(m) => m.len(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum OneOrTwo<S, P> {
    One(OneOf<S, P>),
    Two(S, P),
}

impl<S, P> VPath for OneOf<S, P>
where
    S: VPath,
    P: VPath,
{
    type Metadata = OneOf<S::Metadata, P::Metadata>;
    type File = MergeFile<S::File, P::File>;
    type Iterator = OneOfIterator<S, P>;

    fn file_name(&self) -> Option<String> {
        match self {
            OneOf::First(s) => s.file_name(),
            OneOf::Second(s) => s.file_name(),
        }
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        match self {
            OneOf::First(s) => s.extension(),
            OneOf::Second(s) => s.extension(),
        }
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Self {
        match self {
            OneOf::First(s) => OneOf::First(s.resolve(path)),
            OneOf::Second(s) => OneOf::Second(s.resolve(path)),
        }
    }

    /// Get the parent path
    fn parent(&self) -> Option<Self> {
        match self {
            OneOf::First(s) => s.parent().map(|m| OneOf::First(m)),
            OneOf::Second(s) => s.parent().map(|m| OneOf::Second(m)),
        }
    }

    /// Check if the file existst
    fn exists(&self) -> bool {
        match self {
            OneOf::First(s) => s.exists(),
            OneOf::Second(s) => s.exists(),
        }
    }

    /// Get the file's metadata
    fn metadata(&self) -> Result<Self::Metadata> {
        match self {
            OneOf::First(s) => s.metadata().map(|m| OneOf::First(m)),
            OneOf::Second(s) => s.metadata().map(|m| OneOf::Second(m)),
        }
    }

    fn to_string(&self) -> Cow<str> {
        match self {
            OneOf::First(s) => s.to_string(),
            OneOf::Second(s) => s.to_string(),
        }
    }

    fn to_path_buf(&self) -> Option<PathBuf> {
        match self {
            OneOf::First(s) => s.to_path_buf(),
            OneOf::Second(s) => s.to_path_buf(),
        }
    }

    fn open(&self, o: OpenOptions) -> Result<Self::File> {
        if o.append || o.create || o.truncate {
            return Err(ErrorKind::PermissionDenied.into());
        }
        match self {
            OneOf::First(s) => s.open(o).map(|m| MergeFile {
                inner: OneOf::<S::File, P::File>::First(m),
            }),
            OneOf::Second(s) => s.open(o).map(|m| MergeFile {
                inner: OneOf::<S::File, P::File>::Second(m),
            }),
        }
    }

    fn read_dir(&self) -> Result<Self::Iterator> {
        match self {
            OneOf::First(s) => s.read_dir().map(|m| OneOfIterator::new(OneOf::First(m))),
            OneOf::Second(s) => s.read_dir().map(|m| OneOfIterator::new(OneOf::Second(m))),
        }
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
}

impl<S, P> VPath for OneOrTwo<S, P>
where
    S: VPath,
    P: VPath,
{
    type Metadata = OneOf<S::Metadata, P::Metadata>;
    type File = MergeFile<S::File, P::File>;
    type Iterator = MergeIterator<S, P>;

    fn file_name(&self) -> Option<String> {
        match self {
            OneOrTwo::One(s) => s.file_name(),
            OneOrTwo::Two(s, _) => s.file_name(),
        }
    }

    /// The extension of this filename
    fn extension(&self) -> Option<String> {
        match self {
            OneOrTwo::One(s) => s.extension(),
            OneOrTwo::Two(s, _) => s.extension(),
        }
    }

    /// append a segment to this path
    fn resolve(&self, path: &str) -> Self {
        match self {
            OneOrTwo::One(s) => OneOrTwo::One(s.resolve(path)),
            OneOrTwo::Two(s, p) => {
                let r1 = p.resolve(path);
                let r2 = s.resolve(path);
                if r1.exists()
                    && r2.exists()
                    && r1.metadata().unwrap().is_dir()
                    && r2.metadata().unwrap().is_dir()
                {
                    OneOrTwo::Two(r2, r1)
                } else if r2.exists() && !r1.exists() {
                    OneOrTwo::One(OneOf::First(r2))
                } else {
                    OneOrTwo::One(OneOf::Second(r1))
                }
            }
        }
    }

    /// Get the parent path
    fn parent(&self) -> Option<Self> {
        match self {
            OneOrTwo::One(s) => s.parent().map(|m| OneOrTwo::One(m)),
            // FIXME: Handle if both is a dir
            OneOrTwo::Two(_, p) => p.parent().map(|m| OneOrTwo::One(OneOf::Second(m))),
        }
    }

    /// Check if the file existst
    fn exists(&self) -> bool {
        match self {
            OneOrTwo::One(s) => s.exists(),
            OneOrTwo::Two(s, p) => s.exists() || p.exists(),
        }
    }

    /// Get the file's metadata
    fn metadata(&self) -> Result<Self::Metadata> {
        match self {
            OneOrTwo::One(s) => s.metadata(),
            // FIXME: Handle if both is a dir
            OneOrTwo::Two(s, p) => {
                if p.exists() || !s.exists() {
                    p.metadata().map(|m| OneOf::Second(m))
                } else {
                    s.metadata().map(|m| OneOf::First(m))
                }
            }
        }
    }

    fn to_string(&self) -> Cow<str> {
        match self {
            OneOrTwo::One(s) => s.to_string(),
            // FIXME: Handle if both is a dir
            OneOrTwo::Two(_, p) => p.to_string(),
        }
    }

    fn to_path_buf(&self) -> Option<PathBuf> {
        match self {
            OneOrTwo::One(s) => s.to_path_buf(),
            // FIXME: Handle if both is a dir
            OneOrTwo::Two(_, p) => p.to_path_buf(),
        }
    }

    fn open(&self, o: OpenOptions) -> Result<Self::File> {
        if o.append || o.create || o.truncate {
            return Err(ErrorKind::PermissionDenied.into());
        }
        match self {
            OneOrTwo::One(s) => s.open(o),
            // FIXME: Handle if both is a dir
            OneOrTwo::Two(_, p) => p.open(o).map(|m| MergeFile {
                inner: OneOf::Second(m),
            }),
        }
    }

    fn read_dir(&self) -> Result<Self::Iterator> {
        match self {
            OneOrTwo::One(s) => match s {
                OneOf::First(s) => s
                    .read_dir()
                    .map(|m| MergeIterator::new(OneOrTwo::One(OneOf::First((s.clone(), m))))),
                OneOf::Second(s) => s
                    .read_dir()
                    .map(|m| MergeIterator::new(OneOrTwo::One(OneOf::Second((s.clone(), m))))),
            },
            OneOrTwo::Two(s, p) => {
                let s1 = s.read_dir();
                let s2 = p.read_dir();
                if s1.is_err() && s2.is_err() {
                    Ok(MergeIterator::new(OneOrTwo::Two(
                        (s.clone(), s1.unwrap()),
                        (p.clone(), s2.unwrap()),
                    )))
                } else if s2.is_ok() {
                    Ok(MergeIterator::new(OneOrTwo::One(OneOf::Second((
                        p.clone(),
                        s2.unwrap(),
                    )))))
                } else if s1.is_ok() {
                    Ok(MergeIterator::new(OneOrTwo::One(OneOf::First((
                        s.clone(),
                        s1.unwrap(),
                    )))))
                } else {
                    Err(s2.err().unwrap())
                }
            }
        }
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

impl<S, P> std::io::Write for MergeFile<S, P> {
    fn write(&mut self, _buf: &[u8]) -> Result<usize> {
        Err(ErrorKind::PermissionDenied.into())
    }

    fn flush(&mut self) -> Result<()> {
        Err(ErrorKind::PermissionDenied.into())
    }
}

impl<S, P> VFile for MergeFile<S, P>
where
    S: Read,
    P: Read,
{
}

#[derive(PartialEq, Debug)]
enum MergeIteratorState {
    First,
    Second,
    Done,
}

#[derive(Debug)]
pub struct MergeIterator<S, P>
where
    S: VPath,
    P: VPath,
{
    inner: OneOrTwo<(S, S::Iterator), (P, P::Iterator)>,
    seen: HashSet<String>,
    state: MergeIteratorState,
}

impl<S, P> MergeIterator<S, P>
where
    S: VPath,
    P: VPath,
{
    pub(crate) fn new(one: OneOrTwo<(S, S::Iterator), (P, P::Iterator)>) -> MergeIterator<S, P> {
        MergeIterator {
            inner: one,
            seen: HashSet::new(),
            state: MergeIteratorState::First,
        }
    }
}

impl<S, P> Iterator for MergeIterator<S, P>
where
    S: VPath,
    P: VPath,
{
    type Item = Result<OneOrTwo<S, P>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.state == MergeIteratorState::Done {
            return None;
        }

        let out = match &mut self.inner {
            OneOrTwo::One(OneOf::First((_, i))) => {
                i.next().map(|m| m.map(|m| OneOrTwo::One(OneOf::First(m))))
            }
            OneOrTwo::One(OneOf::Second((_, i))) => {
                i.next().map(|m| m.map(|m| OneOrTwo::One(OneOf::Second(m))))
            }
            OneOrTwo::Two(first, second) => {
                match self.state {
                    MergeIteratorState::First => match second.1.next() {
                        Some(Ok(s)) => {
                            let fl = s.to_string();
                            let p = fl.replace(&second.0.to_string().to_string(), "");
                            self.seen.insert(String::from(fl));
                            if first.0.resolve(&p).exists() {
                                Some(Ok(OneOrTwo::Two(first.0.resolve(&p), s)))
                            } else {
                                Some(Ok(OneOrTwo::One(OneOf::Second(s))))
                            }
                        }
                        Some(Err(e)) => Some(Err(e)),
                        None => {
                            self.state = MergeIteratorState::Second;
                            self.next()
                        }
                    },
                    MergeIteratorState::Second => {
                        while self.state != MergeIteratorState::Done {
                            let out = match first.1.next() {
                                Some(Ok(next)) => {
                                    let fl = next.to_string();
                                    //let p = fl.replace(&first.0.to_string().to_string(), "");
                                    if self.seen.contains(&String::from(fl)) {
                                        None
                                    } else {
                                        Some(Ok(OneOrTwo::One(OneOf::First(next))))
                                    }
                                }
                                Some(Err(err)) => Some(Err(err)),
                                None => {
                                    self.state = MergeIteratorState::Done;
                                    return None;
                                }
                            };

                            if out.is_some() {
                                return out;
                            }
                        }

                        None
                    }
                    _ => None,
                }
            }
            //_ => None,
        };

        if out.is_none() {
            self.state = MergeIteratorState::Done;
        }

        out
    }
}

#[derive(Debug)]
pub struct OneOfIterator<S, P>
where
    S: VPath,
    P: VPath,
{
    inner: OneOf<S::Iterator, P::Iterator>,
}

impl<S, P> OneOfIterator<S, P>
where
    S: VPath,
    P: VPath,
{
    pub fn new(one: OneOf<S::Iterator, P::Iterator>) -> OneOfIterator<S, P> {
        OneOfIterator { inner: one }
    }
}

impl<S, P> Iterator for OneOfIterator<S, P>
where
    S: VPath,
    P: VPath,
{
    type Item = Result<OneOf<S, P>>;
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            OneOf::First(s) => s.next().map(|m| m.map(|m| OneOf::First(m))),
            OneOf::Second(s) => s.next().map(|m| m.map(|m| OneOf::Second(m))),
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
        f.write(b"Hello, World!").unwrap();
        f.flush().unwrap();

        let m2 = MemoryFS::new();
        let mut f = m1.path("/test2.txt").create().unwrap();
        f.write(b"Hello, World!").unwrap();
        f.flush().unwrap();

        let overlay = m1.merge(m2);

        assert!(overlay.path("/test.txt").exists());
        assert!(overlay.path("/test2.txt").exists());
    }

    #[test]
    fn test_overlay_iterator() {
        let m1 = MemoryFS::new();
        let mut f = m1.path("/test.txt").create().unwrap();
        f.write(b"Hello, World!").unwrap();
        f.flush().unwrap();

        let m2 = MemoryFS::new();
        let mut f = m1.path("/test2.txt").create().unwrap();
        f.write(b"Hello, World!").unwrap();
        f.flush().unwrap();

        let overlay = m1.merge(m2);

        let iter = overlay.path("").read_dir().unwrap();

        for i in iter {
            println!("iter {:?}", i.unwrap().to_string());
        }
    }
}
