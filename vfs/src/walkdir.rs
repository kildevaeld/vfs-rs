#[cfg(feature = "glob")]
use crate::glob::*;
use crate::{VMetadata, VPath};
use async_stream::{stream, try_stream};
use futures_lite::{Stream, StreamExt};
// use futures_core::{future::BoxFuture, Stream, TryStream};
// use futures_util::{pin_mut, StreamExt};
use std::future::Future;
use std::io;
use std::pin::Pin;

pub async fn walkdir<V: VPath + 'static + std::marker::Unpin>(
    path: V,
) -> io::Result<Pin<Box<dyn Stream<Item = io::Result<V>> + Send>>>
where
    V::ReadDir: Send + std::marker::Unpin,
    V::Metadata: Send,
{
    walkdir_match::<V, _>(path, |_| true).await
}

pub fn walkdir_match<V: VPath + 'static + std::marker::Unpin, F>(
    path: V,
    check: F,
) -> Pin<Box<Future<Outptut = io::Result<Pin<Box<dyn Stream<Item = io::Result<V>> + Send>>>>>>
where
    F: Sync + Send + 'static + Clone + Fn(&V) -> bool,
    V::ReadDir: Send + std::marker::Unpin,
    V::Metadata: Send,
{
    let out = async move {
        let readdir = path.read_dir().await?;
        let out = try_stream! {
            while let Some(value) = readdir.next().await {
                let value = value?;
                let meta = value.metadata().await?;
                if meta.is_dir()  {
                    let readdir = walkdir_match::<V, F>(value, check.clone()).await?;
                    // pin_mut!(readdir);
                    while let Some(value) = readdir.next().await {
                        let value = value?;
                        if check(&value) {
                            yield value;
                        }
                    }
                } else if meta.is_file() {
                    if check(&value) {
                        yield value;
                    }
                } else {
                    continue;
                }
            }
        };

        Ok(Box::pin(out) as Pin<Box<dyn Stream<Item = io::Result<V>> + Send>>)
    };

    Box::pin(out)
    //Ok(Box::pin(out) as Pin<Box<dyn Stream<Item = io::Result<V>> + Send>>)
}

#[cfg(feature = "glob")]
pub fn glob<P: VPath>(
    path: P,
    glob: Globber,
) -> BoxFuture<'static, io::Result<Pin<Box<dyn Stream<Item = io::Result<P>> + Send>>>>
where
    P: VPath + 'static + std::marker::Unpin,
    P::ReadDir: Send,
    P::Metadata: Send,
{
    walkdir_match(path, move |path| glob.is_match(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    // #[tokio::test]
    // async fn test_walkdir() {
    //     let fs = PhysicalFS::new("../").unwrap();
    //     let path = fs.path(".");

    //     let mut readdir = walkdir(path).await.unwrap();
    //     while let Some(path) = readdir.next().await {
    //         //println!("TEST TEST {:?}", path);
    //     }
    // }

    // #[cfg(feature = "glob")]
    // #[tokio::test]
    // async fn test_glob() {
    //     let fs = PhysicalFS::new("../../").unwrap();
    //     let path = fs.path(".");
    //     println!("PATH {:?}", path);
    //     let mut readdir = glob(path, Globber::new("**/*.toml")).await.unwrap();
    //     while let Some(path) = readdir.next().await {
    //         println!("TEST TEST {:?}", path);
    //     }
    // }
}
