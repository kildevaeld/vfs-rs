#![no_std]

use alloc::{collections::VecDeque, vec::Vec};
use vfs::{Error, VPath};

extern crate alloc;

mod glob;

pub fn walk<'a, V: VPath>(path: &V, patterns: &'a [&'a str]) -> Result<WalkIter<'a, V>, Error> {
    WalkIter::new(path, patterns)
}

pub struct WalkIter<'a, V: VPath> {
    root: V::ReadDir,
    queue: VecDeque<V>,
    patterns: &'a [&'a str],
}

impl<'a, V: VPath> WalkIter<'a, V> {
    pub fn new(path: &V, patterns: &'a [&'a str]) -> Result<WalkIter<'a, V>, Error> {
        let root = path.read_dir()?;
        Ok(WalkIter {
            root,
            queue: Default::default(),
            patterns,
        })
    }
}

impl<'a, V> Iterator for WalkIter<'a, V>
where
    V: VPath,
{
    type Item = Result<V, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = match self.root.next() {
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

                    self.root = readdir;
                    continue;
                }
            };

            let metadata = match next.metadata() {
                Ok(ret) => ret,
                Err(err) => return Some(Err(err)),
            };

            if metadata.id_dir() {
                self.queue.push_back(next);
            } else if self
                .patterns
                .iter()
                .any(|p| crate::glob::glob_match(&p, &next.to_string()))
            {
                return Some(Ok(next));
            }
        }
    }
}
