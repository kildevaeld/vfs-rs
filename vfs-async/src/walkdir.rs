#[cfg(feature = "glob")]
use crate::glob::*;
use crate::{VMetadata, VPath};
use async_stream::{stream, try_stream};
use futures_core::{Stream, TryStream};
use futures_util::{pin_mut, StreamExt};
use std::future::Future;
use std::io;
use std::pin::Pin;

pub fn walkdir<V: VPath + 'static + std::marker::Unpin>(
    path: V,
) -> Pin<Box<dyn Future<Output = io::Result<Pin<Box<dyn Stream<Item = io::Result<V>>>>>>>> {
    // let out = async move {
    //     let readdir = path.read_dir().await?;
    //     let out = try_stream! {
    //         pin_mut!(readdir);
    //         while let Some(value) = readdir.next().await {
    //             let value = value?;
    //             let meta = value.metadata().await?;
    //             if meta.is_file() {
    //                 yield value;
    //             } else {
    //                 let readdir = walkdir(value).await?;
    //                 pin_mut!(readdir);
    //                 while let Some(value) = readdir.next().await {
    //                     let value = value?;
    //                     yield value;
    //                 }
    //             }
    //         }
    //     };

    //     Ok(Box::pin(out) as Pin<Box<dyn Stream<Item = io::Result<V>>>>)
    // };

    // Box::pin(out)
    walkdir_match(path, |_| true)
}

pub fn walkdir_match<V: VPath + 'static + std::marker::Unpin, F>(
    path: V,
    check: F,
) -> Pin<Box<dyn Future<Output = io::Result<Pin<Box<dyn Stream<Item = io::Result<V>>>>>>>>
where
    F: 'static + Clone + Fn(&V) -> bool,
{
    let out = async move {
        let readdir = path.read_dir().await?;
        let out = try_stream! {
            pin_mut!(readdir);
            while let Some(value) = readdir.next().await {
                let value = value?;
                if !check(&value) {
                    continue;
                }
                let meta = value.metadata().await?;
                if meta.is_file() {
                    yield value;
                } else {
                    let readdir = walkdir_match(value, check.clone()).await?;
                    pin_mut!(readdir);
                    while let Some(value) = readdir.next().await {
                        let value = value?;
                        yield value;
                    }
                }
            }
        };

        Ok(Box::pin(out) as Pin<Box<dyn Stream<Item = io::Result<V>>>>)
    };

    Box::pin(out)
}

#[cfg(feature = "glob")]
pub fn glob<P: VPath>(
    path: P,
    glob: Globber,
) -> Pin<Box<dyn Future<Output = io::Result<Pin<Box<dyn Stream<Item = io::Result<P>>>>>>>>
where
    P: VPath + 'static + std::marker::Unpin,
{
    walkdir_match(path, move |path| glob.is_match(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[tokio::test]
    async fn test_walkdir() {
        let fs = PhysicalFS::new("../").unwrap();
        let path = fs.path(".");

        let mut readdir = walkdir(path).await.unwrap();
        while let Some(path) = readdir.next().await {
            println!("TEST TEST {:?}", path);
        }
    }
}
