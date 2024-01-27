use async_trait::async_trait;

use crate::{
    error::ErrorKind, vapath_box, Error, OpenOptions, VAsyncFileExt, VAsyncPath, VAsyncPathBox,
};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

#[async_trait]
pub trait VAsyncPathExt: VAsyncPath {
    fn boxed(self) -> VAsyncPathBox
    where
        Self: Sized + 'static,
        <Self as VAsyncPath>::ReadDir: Send,
        <Self as VAsyncPath>::FS: Clone,
    {
        vapath_box(self)
    }

    async fn read(&self) -> Result<Vec<u8>, Error>
    where
        <Self as VAsyncPath>::File: Unpin,
    {
        let mut file = self.open(OpenOptions::default().read(true)).await?;

        let mut buffer = Vec::default();
        file.read_to_end(&mut buffer).await?;

        Ok(buffer)
    }

    async fn read_to_string(&self) -> Result<String, Error>
    where
        <Self as VAsyncPath>::File: Unpin,
    {
        let buffer = self.read().await?;
        String::from_utf8(buffer).map_err(|err| Error::new(ErrorKind::InvalidData, err.to_string()))
    }
}

impl<P> VAsyncPathExt for P where P: VAsyncPath {}
