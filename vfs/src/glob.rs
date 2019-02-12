use super::traits::{ReadPath, VPath};
use super::utils::WalkDirIter;
use globset::{Candidate, Glob, GlobMatcher, GlobSet, GlobSetBuilder};

#[derive(Clone)]
enum Globber {
    Single(GlobMatcher),
    Set(GlobSet),
}

pub struct GlobWalkDirIter<P> {
    inner: WalkDirIter<P>,
    glob: Globber,
}

impl<P> GlobWalkDirIter<P>
where
    P: VPath,
{
    pub fn new<S: AsRef<str>>(path: P, pattern: S) -> GlobWalkDirIter<P> {
        let glob = Glob::new(pattern.as_ref()).unwrap().compile_matcher();
        GlobWalkDirIter {
            inner: WalkDirIter::new(path),
            glob: Globber::Single(glob),
        }
    }

    pub fn new_set<S: AsRef<str>>(path: P, patterns: Vec<S>) -> GlobWalkDirIter<P> {
        let mut builder = GlobSetBuilder::new();

        for p in patterns {
            let glob = Glob::new(p.as_ref()).unwrap();
            builder.add(glob);
        }

        let glob = builder.build().unwrap();

        GlobWalkDirIter {
            inner: WalkDirIter::new(path),
            glob: Globber::Set(glob),
        }
    }

    fn is_match(&self, path: &P) -> bool {
        let pa = path.to_string().into_owned();
        match &self.glob {
            Globber::Set(p) => p.is_match(pa),
            Globber::Single(p) => p.is_match(pa),
        }
    }
}

impl<P> Iterator for GlobWalkDirIter<P>
where
    P: ReadPath,
{
    type Item = P;
    fn next(&mut self) -> Option<P> {
        loop {
            match self.inner.next() {
                None => return None,
                Some(path) => {
                    if self.is_match(&path) {
                        return Some(path);
                    }
                }
            }
        }
    }
}
