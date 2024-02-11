use alloc::collections::VecDeque;
use core::fmt::Debug;
#[cfg(feature = "async")]
use futures_lite::StreamExt;
use vfs::{Error, VPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveType {
    Project,
    File,
}

pub struct Resolver<'a> {
    ty: ResolveType,
    patterns: &'a [&'a str],
}

impl<'a> Resolver<'a> {
    pub fn new(ty: ResolveType, patterns: &'a [&'a str]) -> Resolver<'a> {
        Resolver { ty, patterns }
    }
}

impl<'a> Resolver<'a> {
    pub fn resolve<V: VPath>(self, path: &V) -> Result<WalkIter<'a, V>, Error> {
        WalkIter::new(path, self.ty, self.patterns)
    }

    #[cfg(feature = "async")]
    pub async fn resolve_async<V: vfs::VAsyncPath>(
        self,
        path: &V,
    ) -> Result<impl futures_core::Stream<Item = Result<V, Error>> + 'a, Error>
    where
        V: 'a,
        V::ReadDir: core::marker::Unpin + 'a,
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
                    .iter()
                    .any(|p| vfs_glob::glob::glob_match(&p, &next.to_string()))
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

pub struct WalkIter<'a, V: VPath> {
    root: Option<V::ReadDir>,
    queue: VecDeque<V>,
    patterns: &'a [&'a str],
    ty: ResolveType,
}

impl<'a, V: VPath> WalkIter<'a, V> {
    pub fn new(
        path: &V,
        ty: ResolveType,
        patterns: &'a [&'a str],
    ) -> Result<WalkIter<'a, V>, Error> {
        let root = path.read_dir()?;
        Ok(WalkIter {
            root: Some(root),
            queue: Default::default(),
            patterns,
            ty,
        })
    }
}

impl<'a, V> Iterator for WalkIter<'a, V>
where
    V: VPath + Debug,
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
            } else if self
                .patterns
                .iter()
                .any(|p| vfs_glob::glob::glob_match(&p, &next.to_string()))
            {
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

        let resolver = Resolver::new(ResolveType::Project, &["src/**/*.rs"]);

        let mut iter = resolver
            .resolve(&fs.path(".").expect("open"))
            .expect("resolver");

        panic!("iter: {:?}", iter.next())
    }
}
