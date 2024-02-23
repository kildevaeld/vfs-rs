use futures::TryStreamExt;
use vfs::{VAsyncFS, VAsyncFSExt, VAsyncPath, VAsyncPathExt};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let fs = vfs_tokio::PhysicalFS::new(".").await?;

    let output = fs.read_to_string("license.md").await?;

    println!("output: {}", output);

    let mut read_dir = fs.path(".")?.read_dir().await?;

    while let Some(next) = read_dir.try_next().await? {
        let meta = next.metadata().await?;

        if meta.is_file() {
            let s = next.read_to_string().await?;
            println!("S: {s}");
        }
    }

    Ok(())
}
