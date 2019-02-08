use std::env;
use vfs2::boxed::read_box;
use vfs2::physical::*;
use vfs2::prelude::*;

fn main() {
    let pwd = env::current_dir().unwrap();

    let fs = PhysicalFS::new(pwd).unwrap();
    let path = fs.path("");

    // let iter = GlobWalkDirIter::new(path, "**/*.{rs,toml}");

    // let iter = GlobWalkDirIter::new_set(path, vec!["**/*.{rs,toml}", "*.lock"]);

    for next in path.glob_walk_set(vec!["**/*.{rs,toml}", "*.lock"]) {
        println!("found {:?}", next);
    }

    let b = read_box(fs);
}
