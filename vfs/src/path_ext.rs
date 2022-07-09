use crate::{vpath_box, OpenOptions, VPath, VPathBox};
use async_trait::async_trait;
use std::io::Result;

#[async_trait]
pub trait VPathExt: VPath {
    async fn create(&self) -> Result<Self::File> {
        self.open(OpenOptions::default().create(true)).await
    }

    fn boxed(self) -> VPathBox
    where
        Self: Sized + 'static,
        <Self as VPath>::ReadDir: Send,
        <Self as VPath>::Metadata: Send,
    {
        vpath_box(self)
    }
}

impl<P> VPathExt for P where P: VPath {}
