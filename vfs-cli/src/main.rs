use vfs::VAsyncFSExt;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let fs = vfs_tokio::PhysicalFS::new(".").await?;

    let output = fs.read_to_string("license.md").await?;

    println!("output: {}", output);

    Ok(())
}
