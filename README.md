# vfs-rs

```rust
use vfs::prelude::*;
use vfs::physical::*;
use vfs::memory::*;
use std::env;

let pwd = env::current_dir().unwrap();
let fs = PhysicalFS::new(&pwd).unwrap();
let path = fs.path("");

for next in path.glob_walk_set(vec!["**/*.{rs,toml}", "*.toml"]) {
  println!("rust or toml: {}", next.to_string());
}

let dest = MemoryFS::new();
copy(fs.path("").glob_walk("**/*.rs"), &dest);

for li in dest.path("").walk_dir() {
  println!("dest {:?}", li.to_string());
}


```
