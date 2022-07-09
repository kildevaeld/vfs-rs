use super::traits::{VMetadata, VPath, VFS};
use super::OpenOptions;
// use futures_core::Stream;
// use futures_io::AsyncRead;
use futures_lite::StreamExt;
use pin_project::pin_project;
use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

// use tokio;

// #[pin_project]
// struct ByteStream<R, N: ArrayLength<u8>>(#[pin] R, GenericArray<u8, N>);

// impl<R, N: ArrayLength<u8>> ByteStream<R, N> {
//     pub fn new(read: R) -> ByteStream<R, N> {
//         ByteStream(read, GenericArray::default())
//     }
// }

// impl<R: AsyncRead, N: ArrayLength<u8>> Stream for ByteStream<R, N> {
//     // The same as our future above:
//     type Item = Result<Bytes, std::io::Error>;
//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         let mut this = self.project();
//         //let mut buf = [0; 1024];
//         match this.0.poll_read(cx, &mut this.1) {
//             Poll::Pending => Poll::Pending,
//             Poll::Ready(Ok(ret)) => {
//                 if ret == 0 {
//                     Poll::Ready(None)
//                 } else {
//                     Poll::Ready(Some(Ok(Bytes::copy_from_slice(&this.1[0..ret]))))
//                 }
//             }
//             Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err))),
//         }
//     }
// }

#[derive(Debug)]
pub enum CopyError {
    Io(Error),
    InvalidPath,
}

impl fmt::Display for CopyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl StdError for CopyError {}

impl From<Error> for CopyError {
    fn from(error: Error) -> CopyError {
        CopyError::Io(error)
    }
}

pub struct Entry<P: VPath> {
    pub path: P,
    pub file: P::File,
}

impl<P: VPath> Entry<P> {
    fn new(path: P, file: P::File) -> Entry<P> {
        Entry { path, file }
    }
}

pub async fn copy<S: VPath, D: VPath>(source: S, dest: D) -> Result<(), CopyError>
where
    S: 'static,
    S::File: std::marker::Unpin,
    S::Metadata: Send,
    S::ReadDir: std::marker::Unpin + Send,
    D: Clone + 'static,
    D::File: std::marker::Unpin,
    D::Metadata: Send,
{
    Ok(copy_path(source, dest, |source, dest| async move {
        futures_lite::io::copy(source.file, dest.file)
            .await
            .map(|_| ())
            .map_err(CopyError::Io)
    })
    .await?)
}

pub fn copy_path<S: VPath, D: VPath, F, U>(
    source: S,
    dest: D,
    copy_file: F,
) -> Pin<Box<dyn Future<Output = Result<(), CopyError>> + Send>>
where
    S: 'static,
    S::File: std::marker::Unpin,
    S::Metadata: Send,
    S::ReadDir: std::marker::Unpin + Send,
    D: Clone + 'static,
    D::File: std::marker::Unpin,
    D::Metadata: Send,
    F: 'static + Sync + Send + Clone + Fn(Entry<S>, Entry<D>) -> U,
    U: 'static + Send + Future<Output = Result<(), CopyError>>,
{
    Box::pin(async move {
        let s_meta = source.metadata().await?;
        let d_meta = dest.metadata().await?;

        if s_meta.is_dir() && d_meta.is_file() {
            return Err(CopyError::InvalidPath);
        } else if s_meta.is_file() && d_meta.is_file() {
            let s_file = source.open(OpenOptions::new().read(true)).await?;
            let d_file = dest
                .open(OpenOptions::new().write(true).truncate(true).create(true))
                .await?;
            return Ok(copy_file(
                Entry::new(source.clone(), s_file),
                Entry::new(dest.clone(), d_file),
            )
            .await
            .map(|_| ())?);
        } else if s_meta.is_file() && d_meta.is_dir() {
            let s_file = source.open(OpenOptions::new().read(true)).await?;
            let d_path = dest.resolve(&source.to_string());
            d_path.parent().unwrap().create_dir().await?;

            let d_file = d_path
                .open(OpenOptions::new().write(true).truncate(true).create(true))
                .await?;
            return Ok(
                copy_file(Entry::new(source, s_file), Entry::new(d_path, d_file))
                    .await
                    .map(|_| ())?,
            );
        } else {
            let mut read_dir = source.read_dir().await?;

            while let Some(next) = read_dir.next().await {
                let next = next?;
                copy_path(next, dest.clone(), copy_file.clone()).await?;
            }
        }

        Ok(())
    })
}
