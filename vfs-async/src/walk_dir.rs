#[cfg(feature = "glob")]
use super::glob::Globber;
use super::traits::{VMetadata, VPath};
use futures_core::Stream;
use futures_util::future::BoxFuture;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

enum WalkDirState<P>
where
    P: VPath + 'static,
{
    None,
    Open(P, BoxFuture<'static, Result<P::ReadDir, io::Error>>),
    Next(P, P::ReadDir),
    Meta(
        P,
        P::ReadDir,
        P,
        BoxFuture<'static, Result<P::Metadata, io::Error>>,
    ),
}

pub struct WalkDir<P>
where
    P: VPath + 'static,
{
    todos: Vec<P>,
    filter: Box<dyn Fn(&P) -> bool>,
    state: WalkDirState<P>,
}

impl<P> WalkDir<P>
where
    P: VPath + 'static,
{
    pub fn new(path: P) -> WalkDir<P> {
        WalkDir::new_with(path, |_| true)
    }

    pub fn new_with<F: 'static + Fn(&P) -> bool>(path: P, f: F) -> WalkDir<P> {
        WalkDir {
            todos: vec![path],
            filter: Box::new(f),
            state: WalkDirState::None,
        }
    }

    #[cfg(feature = "glob")]
    pub fn glob(path: P, glob: Globber) -> WalkDir<P> {
        Self::new_with(path, move |p| glob.is_match(p))
    }
}

impl<P> Stream for WalkDir<P>
where
    P: VPath + 'static + std::fmt::Debug,
{
    type Item = Result<P, io::Error>;

    #[allow(unreachable_code)]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = unsafe { Pin::get_unchecked_mut(self) };
        loop {
            let state = std::mem::replace(&mut this.state, WalkDirState::None);

            let (next_state, poll): (
                Option<WalkDirState<P>>,
                Option<Poll<Option<Result<P, io::Error>>>>,
            ) = match state {
                WalkDirState::None => match this.todos.pop() {
                    Some(path) => {
                        let open = path.read_dir();
                        (Some(WalkDirState::Open(path, open)), None)
                    }
                    None => (None, Some(Poll::Ready(None))),
                },
                WalkDirState::Open(root, mut future) => {
                    match unsafe { Pin::new_unchecked(&mut future) }.poll(cx) {
                        Poll::Pending => {
                            (Some(WalkDirState::Open(root, future)), Some(Poll::Pending))
                        }
                        Poll::Ready(Ok(s)) => (Some(WalkDirState::Next(root, s)), None),
                        Poll::Ready(Err(e)) => (
                            Some(WalkDirState::Open(root, future)),
                            Some(Poll::Ready(Some(Err(e)))),
                        ),
                    }
                }
                WalkDirState::Next(root, mut walkdir) => {
                    match unsafe { Pin::new_unchecked(&mut walkdir) }.poll_next(cx) {
                        Poll::Pending => {
                            (Some(WalkDirState::Next(root, walkdir)), Some(Poll::Pending))
                        }
                        Poll::Ready(None) => (Some(WalkDirState::None), None),
                        Poll::Ready(Some(Ok(path))) => {
                            let meta = path.metadata();
                            let meta = WalkDirState::Meta(root, walkdir, path, meta);
                            (Some(meta), None)
                        }
                        Poll::Ready(Some(Err(err))) => (
                            Some(WalkDirState::Next(root, walkdir)),
                            Some(Poll::Ready(Some(Err(err)))),
                        ),
                    }
                }

                WalkDirState::Meta(root, readdir, path, mut future) => {
                    match unsafe { Pin::new_unchecked(&mut future) }.poll(cx) {
                        Poll::Pending => (
                            Some(WalkDirState::Meta(root, readdir, path, future)),
                            Some(Poll::Pending),
                        ),
                        Poll::Ready(Ok(meta)) => {
                            if meta.is_dir() {
                                this.todos.push(path.clone());
                                (Some(WalkDirState::Next(root, readdir)), None)
                            } else {
                                if (this.filter)(&path) {
                                    (
                                        Some(WalkDirState::Next(root, readdir)),
                                        Some(Poll::Ready(Some(Ok(path)))),
                                    )
                                } else {
                                    (Some(WalkDirState::Next(root, readdir)), None)
                                }
                            }
                        }
                        Poll::Ready(Err(e)) => (
                            Some(WalkDirState::Meta(root, readdir, path, future)),
                            Some(Poll::Ready(Some(Err(e)))),
                        ),
                    }
                }
            };

            if let Some(next_state) = next_state {
                this.state = next_state;
            }

            if let Some(poll) = poll {
                return poll;
            }
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Result};
    use std::path::PathBuf;

    use super::super::physical::*;
    use super::super::traits::{VFile, VPath, VFS};
    use super::*;
    use futures_util::io::AsyncReadExt;
    use futures_util::StreamExt;

    #[tokio::test]
    async fn test_stuff() {
        let vfs = PhysicalFS::new(".").unwrap();
        let src = vfs.path(".");

        let mut wlk = WalkDir::new(src);

        while let Some(i) = wlk.next().await {
            //println!("PATH {:?}", i);
        }
    }

    #[cfg(feature = "glob")]
    #[tokio::test]
    async fn test_glob() {
        let vfs = PhysicalFS::new("..").unwrap();
        let src = vfs.path(".");

        let mut wlk = WalkDir::glob(src, Globber::new("*.toml"));

        while let Some(i) = wlk.next().await {
            //println!("PATH2 {:?}", i);
        }
    }
}
