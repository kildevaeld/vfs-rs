use super::traits::{VPath, VMetadata};
use futures_core::Stream;
use futures_util::future::BoxFuture;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;

enum WalkDirState<'a, P>
where
    P: VPath + 'a,
{
    None,
    Open(P, BoxFuture<'a, Result<P::ReadDir, io::Error>>),
    Next(P, P::ReadDir),
    Meta(P, P::ReadDir, P, BoxFuture<'a, Result<P::Metadata, io::Error>>),
}

pub struct WalkDir<P>
where
    P: VPath + 'static,
{
    todos: Vec<P>,
    filter: Box<dyn Fn(&P) -> bool>,
    state: WalkDirState<'static, P>
}

impl<P> WalkDir<P>
where
    P: VPath + 'static,
{
    pub fn new(path: P) -> WalkDir<P> {
        WalkDir {
            todos: vec![path],
            filter: Box::new(|_| true),
            state: WalkDirState::None,
        }
    }
}

impl<P> Stream for WalkDir<P>
where
    P: VPath + 'static,
{
    type Item = Result<P, io::Error>;

    #[allow(unreachable_code)]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = unsafe { Pin::get_unchecked_mut(self) };
       
        loop {

            let mut state = std::mem::replace(&mut this.state, WalkDirState::None);

            let (next_state, poll): (Option<WalkDirState<'_, P>>, Option<Poll<Option<Result<P, io::Error>>>>) = match state {
                WalkDirState::None => {
                    match this.todos.pop() {
                        Some(path) => {
                            let open = path.read_dir();

                            (Some(WalkDirState::Open(path, open)), None)

                        },
                        None => (None, Some(Poll::Ready(None)))
                    }
                }
                WalkDirState::Open(root, mut future) => {
                    match unsafe { Pin::new_unchecked(&mut future) }.poll(cx) {
                        Poll::Pending =>  (None, Some(Poll::Pending)),
                        Poll::Ready(Ok(s)) => {
                            (Some(WalkDirState::Next(root, s)), None)
                        },
                        Poll::Ready(Err(e)) => {
                            (None, Some(Poll::Ready(Some(Err(e)))))
                        }
                    }
                },
                WalkDirState::Next(root, mut walkdir) => {
                    match unsafe { Pin::new_unchecked(&mut walkdir) }.poll_next(cx) {
                        Poll::Pending =>  (None, Some(Poll::Pending)),
                        Poll::Ready(None) => {
                            (Some(WalkDirState::None), None)
                        },
                        Poll::Ready(Some(Ok(path))) => {
                            let meta = path.metadata();
                            //WalkDirState::None
                            let meta = WalkDirState::Meta(root, walkdir, path, meta);
                            (Some(meta), None)
                            //(Some(WalkDirState::Meta(root, walkdir, path, meta), None)
                        },
                        Poll::Ready(Some(Err(err))) => {
                            (None, Some(Poll::Ready(Some(Err(err)))))
                        }

                    }
                }

                WalkDirState::Meta(root, readdir, path, mut future) => {
                    match unsafe { Pin::new_unchecked(&mut future) }.poll(cx) {
                        Poll::Pending =>  (None, Some(Poll::Pending)),
                        Poll::Ready(Ok(meta)) => {
                            if meta.is_dir() {
                                this.todos.push(path.clone());
                                (Some(WalkDirState::None), None)
                            } else {
                                (Some(WalkDirState::Next(root, readdir)), Some(Poll::Ready(Some(Ok(path)))))
                            }

                    
                        },
                        Poll::Ready(Err(e)) => {
                            (None, Some(Poll::Ready(Some(Err(e)))))
                        }
                    }
                }
            };


            if let Some(next_state) = next_state {
                this.state = next_state;
            }

            if let Some(poll) = poll {
                return poll;
            }



            /*
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
                            return Poll::Ready(Some(Err(e)))
                        }
                    }
                }
               
                WalkDirState::Meta(root, path, future) => {
                    match unsafe { Pin::new_unchecked(future) }.poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Ok(meta)) => {
                            if meta.is_dir() {
                                this.todos.push(path.clone());
                                WalkDirState::
                            } else {
                                
                            }

                            return
                        }
                        Poll::Ready(Err(e)) => {
                            return Poll::Ready(Err(e))
                        }
                    }
                }
            };

            this.state = next_state;

            break*/
    

        }

        Poll::Pending
    }
}
