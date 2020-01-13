use super::traits::VPath;
use futures_core::Stream;
use futures_util::future::BoxFuture;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;

pub enum WalkDirState<'a, P>
where
    P: VPath,
{
    None,
    Open(P, BoxFuture<'a, Result<P::ReadDir, io::Error>>),
    Next(P, P::ReadDir),
    Meta(P, P, BoxFuture<'a, Result<P::Metadata, io::Error>>),
}

pub struct WalkDir<P>
where
    P: VPath,
{
    todos: Vec<P>,
    filter: Box<dyn Fn(&P) -> bool>,
    state: WalkDirState<'static, P>
}

impl<P> WalkDir<P>
where
    P: VPath,
{
    pub fn new(path: P) -> WalkDir<P> {
        WalkDir {
            todos: Vec::new(),
            filter: Box::new(|_| true),
            state: WalkDirState::None,
        }
    }
}

impl<P> Stream for WalkDir<P>
where
    P: VPath,
{
    type Item = P;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = unsafe { Pin::get_unchecked_mut(self) };
        loop {

            let next_state = match &mut this.state {
                WalkDirState::None => {
                    let path = this.todos.pop();
                    let path = match path {
                        Some(s) => s,
                        None => return Poll::Ready(None)
                    };

                    let open = path.read_dir();

                    WalkDirState::Open(path, open)

                },
                WalkDirState::Next(root, walkdir) => {
                    match unsafe { Pin::new_unchecked(walkdir) }.poll_next(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(None) => {
                            WalkDirState::None
                        }
                        Poll::Ready(Some(Ok(path))) => {
                            let meta = path.metadata();
                            //WalkDirState::None
                            WalkDirState::Meta(root.clone(), path, meta)
                        },
                        Poll::Ready(Some(Err(err))) => {
                            return Poll::Ready(Some(Err(err)));
                        }
                    }
                }
                WalkDirState::Open(root, future) => {
                    match unsafe { Pin::new_unchecked(future) }.poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Ok(s)) => {
                            WalkDirState::Next(root.clone(), s)
                        }
                        Poll::Ready(Err(e)) => {
                            return Poll::Ready(Err(e))
                        }
                    }
                }
               
                WalkDirState::Meta(root, path, future) => {
                    match unsafe { Pin::new_unchecked(future) }.poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Ok(meta)) => {
                            if meta.is_dir() {

                            } else {
                                
                            }
                        }
                        Poll::Ready(Err(e)) => {
                            return Poll::Ready(Err(e))
                        }
                    }
                }
            };

            this.state = next_state;

            break
    

        }
        Poll::Pending
    }
}
