use super::traits::VPath;
use globset::{Glob, GlobMatcher, GlobSet, GlobSetBuilder};

#[derive(Clone)]
pub enum Globber {
    Single(GlobMatcher),
    Set(GlobSet),
}

impl Globber {
    pub fn new<S: AsRef<str>>(pattern: S) -> Globber {
        Globber::Single(Glob::new(pattern.as_ref()).unwrap().compile_matcher())
    }

    pub fn new_set<S: AsRef<str>>(patterns: &[S]) -> Globber {
        let mut builder = GlobSetBuilder::new();

        for p in patterns {
            let glob = Glob::new(p.as_ref()).unwrap();
            builder.add(glob);
        }

        let glob = builder.build().unwrap();
        Globber::Set(glob)
    }

    pub fn is_match<P: VPath>(&self, path: &P) -> bool {
        let pa = path.to_string().into_owned();
        match self {
            Globber::Set(p) => p.is_match(pa),
            Globber::Single(p) => p.is_match(pa),
        }
    }
}
