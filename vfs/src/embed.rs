use super::{DataHandle, MemoryFS, OpenOptions, VPath, VFS};
use rust_embed::RustEmbed;

pub async fn assets_to_vfs<R: RustEmbed>() -> MemoryFS {
    let mem = MemoryFS::new();

    for path in R::iter() {
        let parent = pathutils::parent_path(&path).unwrap();
        let mem_path = mem.path(parent);
        mem_path.create_dir_inner().unwrap();
        let mem_path = mem.path(&path);
        let d_file = mem_path
            .open_with_options(&OpenOptions::new().write(true).create(true).truncate(true))
            .unwrap();

        let s_file = R::get(&path).unwrap();
        let mut data = d_file.data.0.write().unwrap();
        *data = s_file.data.to_vec();
    }

    mem
}
