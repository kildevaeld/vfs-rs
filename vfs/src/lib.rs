#[cfg(feature = "boxed")]
pub mod boxed;
#[cfg(feature = "memory")]
mod memory;
#[cfg(feature = "fs")]
mod physical;
mod traits;

#[cfg(feature = "embed")]
pub mod embed;
pub mod util;
// #[cfg(feature = "extra")]
// mod walkdir;

#[cfg(feature = "memory")]
pub use memory::*;
#[cfg(feature = "fs")]
pub use physical::*;

pub use traits::*;

use futures_lite::{AsyncReadExt, AsyncWriteExt};

#[async_trait::async_trait]
pub trait VFSExt: VFS {
    #[cfg(feature = "boxed")]
    fn boxed(self) -> Box<dyn boxed::BVFS>
    where
        Self: Sized + 'static,
        <Self::Path as VPath>::ReadDir: Send,
        <Self::Path as VPath>::Metadata: Send,
    {
        boxed::vfs_box(self)
    }

    async fn read(&self, path: &str) -> Result<Vec<u8>, std::io::Error>
    where
        <Self::Path as VPath>::File: std::marker::Unpin,
    {
        let mut file = self.path(path).open(OpenOptions::new().read(true)).await?;
        let mut out = Vec::new();
        file.read_to_end(&mut out).await?;
        Ok(out)
    }

    async fn write(&self, path: &str, data: &[u8]) -> Result<(), std::io::Error>
    where
        <Self::Path as VPath>::File: std::marker::Unpin,
    {
        let mut file = self
            .path(path)
            .open(OpenOptions::new().write(true).truncate(true).create(true))
            .await?;

        file.write_all(data).await?;

        file.flush().await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl<T> VFSExt for T where T: VFS {}

// pub trait VPathExt: VPath {
//     #[cfg(feature = "boxed")]
//     fn boxed(self) -> Box<dyn boxed::BVPath>
//     where
//         Self: Sized + 'static,
//         <Self as VPath>::ReadDir: Send,
//     {
//         boxed::path_box(self)
//     }
// }

// impl<T> VPathExt for T where T: VPath {}
