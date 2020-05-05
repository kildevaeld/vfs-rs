use super::traits::{VPath, VFS};
use futures_core::Stream;
use futures_io::AsyncRead;
use pin_project::pin_project;
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio;

#[pin_project]
struct ByteStream<R, N: ArrayLength<u8>>(#[pin] R, GenericArray<u8, N>);

impl<R, N: ArrayLength<u8>> ByteStream<R, N> {
    pub fn new(read: R) -> ByteStream<R, N> {
        ByteStream(read, GenericArray::default())
    }
}

impl<R: AsyncRead, N: ArrayLength<u8>> Stream for ByteStream<R, N> {
    // The same as our future above:
    type Item = Result<Bytes, std::io::Error>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        //let mut buf = [0; 1024];
        match this.0.poll_read(cx, &mut this.1) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(ret)) => {
                if ret == 0 {
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(Ok(Bytes::copy_from_slice(&this.1[0..ret]))))
                }
            }
            Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err))),
        }
    }
}

// enum Msg<P, F> {
//     File(P, F),
//     Dir(P),
//     //Err(io::Error),
// }

// pub async fn copy<S, P, D: ?Sized>(source: S, dest: D)
// where
//     S: Stream<Item = Result<P, Error>> + Send,
//     P: VPath,
//     D: VPath + Send + Sync,
//     // <D as VFS>::Path: VPath,
// {

//     tokio::spawn(async move {

//     });

//     crossbeam::scope(|scope| {
//         let (sx, rx) = bounded(10);
//         scope.spawn(move |_| {
//             for p in source {
//                 let meta = match p.metadata() {
//                     Ok(m) => m,
//                     Err(_) => continue,
//                 };

//                 let msg = if meta.is_dir() {
//                     Msg::Dir(p)
//                 } else if meta.is_file() {
//                     if let Some(parent) = p.parent() {
//                         sx.send(Msg::Dir(parent)).unwrap();
//                     }
//                     let file = p.open(OpenOptions::new().read(true)).unwrap();
//                     Msg::File(p, file)
//                 } else {
//                     continue;
//                 };

//                 sx.send(msg).unwrap();
//             }
//         });
//         scope.spawn(move |_| loop {
//             let mut msg = match rx.recv() {
//                 Ok(m) => m,
//                 Err(_) => return,
//             };

//             let ret = match &mut msg {
//                 Msg::Dir(path) => {
//                     let path = dest.path(&path.to_string());
//                     if path.exists() {
//                         continue;
//                     }
//                     path.mkdir()
//                 }
//                 Msg::File(path, reader) => {
//                     let path = dest.path(&path.to_string());
//                     let mut file = path.open(OpenOptions::new().create(true)).unwrap();
//                     io::copy(reader, &mut file).map(|_| ())
//                 }
//             };
//             if ret.is_err() {}
//         });
//     })
//     .unwrap();
// }
