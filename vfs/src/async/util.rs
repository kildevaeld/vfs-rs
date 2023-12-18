use super::types::{VMetadata, VPath};
use super::OpenOptions;
use futures_util::StreamExt;

use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::io::Error;
use std::pin::Pin;

#[derive(Debug)]
pub enum CopyError {
    Io(Error),
    InvalidPath,
}

impl fmt::Display for CopyError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

pub fn copy_with<S: VPath, D: VPath, F, U>(
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
            let d_path = dest.resolve(&source.to_string())?;
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
                copy_with(next, dest.clone(), copy_file.clone()).await?;
            }
        }

        Ok(())
    })
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
    Ok(copy_with(source, dest, |source, mut dest| async move {
        futures_util::io::copy(source.file, &mut dest.file)
            .await
            .map(|_| ())
            .map_err(CopyError::Io)
    })
    .await?)
}
