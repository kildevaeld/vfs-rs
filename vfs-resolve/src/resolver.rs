use alloc::{collections::VecDeque, vec::Vec};
use core::fmt::Debug;
#[cfg(feature = "async")]
use futures_lite::StreamExt;
use vfs::{Error, VPath};

pub trait Patterns {
    fn matches(&self, path: &str) -> bool;
}

impl<'a, T> Patterns for &'a [T]
where
    T: AsRef<str>,
{
    fn matches(&self, path: &str) -> bool {
        self.into_iter()
            .any(|i| vfs_glob::glob::glob_match(i.as_ref(), path))
    }
}

impl<T> Patterns for Vec<T>
where
    T: AsRef<str>,
{
    fn matches(&self, path: &str) -> bool {
        (&**self).matches(path)
    }
}

impl<'a> Patterns for &'a str {
    fn matches(&self, path: &str) -> bool {
        vfs_glob::glob::glob_match(self, path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveType {
    Project,
    File,
}

pub struct Resolver<P> {
    ty: ResolveType,
    patterns: P,
}

impl<P> Resolver<P> {
    pub fn new(ty: ResolveType, patterns: P) -> Resolver<P> {
        Resolver { ty, patterns }
    }
}

impl<P> Resolver<P> {
    pub fn resolve<V: VPath>(self, path: &V) -> Result<WalkIter<V, P>, Error> {
        WalkIter::new(path, self.ty, self.patterns)
    }

    #[cfg(feature = "async")]
    pub async fn resolve_async<'a, V: vfs::VAsyncPath>(
        self,
        path: &'a V,
    ) -> Result<impl futures_core::Stream<Item = Result<V, Error>> + 'a, Error>
    where
        V: 'a,
        V::ReadDir: core::marker::Unpin + 'a,
        P: Patterns + Send + Sync + 'a,
    {
        let mut mainroot = Some(path.read_dir().await?);

        Ok(async_stream::try_stream! {

            let mut queue = VecDeque::<V>::default();

            loop {
                let root = match &mut mainroot {
                    Some(root) => root,
                    None => break,
                };

                let next = match root.try_next().await? {
                    Some(next) => next,
                    None => {
                        let Some(new) = queue.pop_front() else {
                            break;
                        };

                        let readdir =  new.read_dir().await?;

                        mainroot = Some(readdir);
                        continue;
                    }
                };

                let metadata =  next.metadata().await?;

                if metadata.is_dir() {
                    queue.push_back(next);
                } else if self
                    .patterns
                    .matches(&next.to_string())
                {
                    if self.ty == ResolveType::Project {
                        mainroot = if let Some(new) = queue.pop_front() {
                            Some(new.read_dir().await?)
                        } else {
                            None
                        };
                    }
                    yield next;
                }

            }

        })
    }
}

pub struct WalkIter<V: VPath, P> {
    root: Option<V::ReadDir>,
    queue: VecDeque<V>,
    patterns: P,
    ty: ResolveType,
}

impl<V: VPath, P> WalkIter<V, P> {
    pub fn new(path: &V, ty: ResolveType, patterns: P) -> Result<WalkIter<V, P>, Error> {
        let root = path.read_dir()?;
        Ok(WalkIter {
            root: Some(root),
            queue: Default::default(),
            patterns,
            ty,
        })
    }
}

impl<V, P> Iterator for WalkIter<V, P>
where
    V: VPath + Debug,
    P: Patterns,
{
    type Item = Result<V, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let root = match &mut self.root {
                Some(root) => root,
                None => return None,
            };

            let next = match root.next() {
                Some(Ok(next)) => next,
                Some(Err(err)) => return Some(Err(err)),
                None => {
                    let Some(new) = self.queue.pop_front() else {
                        return None;
                    };

                    let readdir = match new.read_dir() {
                        Ok(ret) => ret,
                        Err(err) => return Some(Err(err)),
                    };

                    self.root = Some(readdir);
                    continue;
                }
            };

            let metadata = match next.metadata() {
                Ok(ret) => ret,
                Err(err) => return Some(Err(err)),
            };

            if metadata.is_dir() {
                self.queue.push_back(next);
            } else if self.patterns.matches(&next.to_string()) {
                if self.ty == ResolveType::Project {
                    self.root = if let Some(new) = self.queue.pop_front() {
                        Some(new.read_dir().unwrap())
                    } else {
                        None
                    };
                }
                return Some(Ok(next));
            }
        }
    }
}

#[cfg(test)]
mod test {
    use vfs::VFS;

    use super::*;

    #[test]
    fn test() {
        let fs = vfs_std::Fs::new(".").expect("open");

        let resolver = Resolver::new(ResolveType::Project, &["src/**/*.rs"][..]);

        let mut iter = resolver
            .resolve(&fs.path(".").expect("open"))
            .expect("resolver");

        panic!("iter: {:?}", iter.next())
    }
}
