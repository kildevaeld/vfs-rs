#[cfg(feature = "boxed")]
use super::boxed;
use super::VPath;
use async_trait::async_trait;
use futures::TryStreamExt;

#[async_trait]
pub trait VPathExt: VPath {
    #[cfg(feature = "boxed")]
    fn boxed(self) -> Box<dyn boxed::BVPath>
    where
        Self: Sized + 'static,
        <Self as VPath>::ReadDir: Send,
    {
        boxed::path_box(self)
    }

    async fn resolve_with(&self, basename: &str, exts: &[&str]) -> Result<Option<Self>> {
        for ext in exts {
            let file_name = pathutils::join(basename, ext);
            let path = self.resolve(file_name);
            if !path.exists().await {
                return Ok(None);
            } else {
                Ok(Some(path))
            }
        }
        None
    }
}

impl<T> VPathExt for T where T: VPath {}
