use std::env;
use vfs::boxed::read_box;
use vfs::memory::*;
use vfs::physical::*;
use vfs::prelude::*;

fn main() {
    let pwd = env::current_dir().unwrap();

    let fs = PhysicalFS::new(&pwd).unwrap();
    let path = fs.path("");

    for next in path.glob_walk_set(vec!["**/*.{rs,toml}", "*.toml"]) {
        println!("found {:?}", next);
    }

    let dest = MemoryFS::new();

    copy(fs.path("").glob_walk("**/*.rs"), &dest);

    for li in dest.path("").walk_dir() {
        println!("dest {:?}", li.to_string());
    }

    let b = read_box(fs);
    for bb in b.path("").glob_walk("**/*.rs") {
        println!("found {}", bb.to_string());
    }
}
