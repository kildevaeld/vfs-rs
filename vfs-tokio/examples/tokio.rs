use futures::TryStreamExt;
use std::path::PathBuf;
use vfs::prelude::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> vfs::Result<()> {
    let fs = vfs_tokio::FS::new(PathBuf::from(".")).await?;

    let path = fs.path(".")?;

    let mut stream = path.read_dir().await?;

    while let Some(next) = stream.try_next().await? {
        let metadata = next.metadata().await?;
        println!("Next {:?} {}", metadata, next.to_string());
    }

    Ok(())
}
