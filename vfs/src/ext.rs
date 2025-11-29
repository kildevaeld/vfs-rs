#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
use core::{
    pin::Pin,
    task::{Context, Poll, ready},
};
use pin_project_lite::pin_project;
#[cfg(all(feature = "std", not(feature = "alloc")))]
use std::vec::Vec;

use crate::{Error, ErrorKind, SeekFrom, VFile, VPath};

pub trait VPathExt: VPath {
    #[cfg(any(feature = "std", feature = "alloc"))]
    fn boxed(self) -> crate::boxed::BoxVPath
    where
        Self: Clone + 'static,
        Self: VPath + Send + Sync,
        Self::File: Send + Sync + 'static,
        Self::Metadata: Send + 'static,
        Self::Open: Send + 'static,
        Self::CreateDir: Send + 'static,
        Self::Remove: Send + 'static,
        Self::ReadDir: Send + 'static,
        Self::ListDir: Send + 'static,
    {
        crate::boxed::path_box(self)
    }
}

impl<T> VPathExt for T where T: VPath {}

pub trait VFileExt: VFile {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Read<'a, Self>
    where
        Self: Sized + Unpin,
    {
        Read { reader: self, buf }
    }

    #[cfg(any(feature = "std", feature = "alloc"))]
    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> ReadToEnd<'a, Self>
    where
        Self: Sized + Unpin,
    {
        ReadToEnd::new(self, buf)
    }

    fn seek<'a>(&'a mut self, seek: SeekFrom) -> Seek<'a, Self>
    where
        Self: Sized + Unpin,
    {
        Seek {
            file: self,
            pos: seek,
        }
    }

    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAll<'a, Self>
    where
        Self: Sized + Unpin,
    {
        WriteAll::new(self, buf)
    }
}

impl<T> VFileExt for T where T: VFile {}

pub struct Read<'a, T: VFile + ?Sized> {
    reader: &'a mut T,
    buf: &'a mut [u8],
}

impl<A> Future for Read<'_, A>
where
    A: VFile + ?Sized + Unpin,
{
    type Output = Result<usize, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        Pin::new(&mut this.reader).poll_read(cx, this.buf)
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
pub(crate) struct Guard<'a> {
    pub buf: &'a mut Vec<u8>,
    pub len: usize,
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        unsafe {
            self.buf.set_len(self.len);
        }
    }
}

// This uses an adaptive system to extend the vector when it fills. We want to
// avoid paying to allocate and zero a huge chunk of memory if the reader only
// has 4 bytes while still making large reads if the reader does have a ton
// of data to return. Simply tacking on an extra DEFAULT_BUF_SIZE space every
// time is 4,500 times (!) slower than this if the reader has a very small
// amount of data to return.
//
// Because we're extending the buffer with uninitialized data for trusted
// readers, we need to make sure to truncate that if any of this panics.
#[cfg(any(feature = "std", feature = "alloc"))]
fn read_to_end_internal<R: VFile + ?Sized>(
    mut rd: core::pin::Pin<&mut R>,
    cx: &mut core::task::Context<'_>,
    buf: &mut Vec<u8>,
) -> Poll<Result<(), Error>> {
    let mut g = Guard {
        len: buf.len(),
        buf,
    };
    let ret;
    loop {
        if g.len == g.buf.len() {
            unsafe {
                g.buf.reserve(32);
                let capacity = g.buf.capacity();
                g.buf.set_len(capacity);
                // rd.initializer().initialize(&mut g.buf[g.len..]);
            }
        }

        match rd.as_mut().poll_read(cx, &mut g.buf[g.len..]) {
            Poll::Ready(Ok(0)) => {
                ret = Poll::Ready(Ok(()));
                break;
            }
            Poll::Ready(Ok(n)) => g.len += n,
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(e)) => {
                ret = Poll::Ready(Err(e));
                break;
            }
        }
    }

    ret
}

#[cfg(any(feature = "std", feature = "alloc"))]
#[derive(Debug)]
pub struct ReadToEnd<'a, R: ?Sized + Unpin> {
    reader: &'a mut R,
    buf: &'a mut Vec<u8>,
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<R: ?Sized + Unpin> Unpin for ReadToEnd<'_, R> {}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a, R: VFile + ?Sized + Unpin> ReadToEnd<'a, R> {
    pub(super) fn new(reader: &'a mut R, buf: &'a mut Vec<u8>) -> Self {
        ReadToEnd { reader, buf }
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<A> Future for ReadToEnd<'_, A>
where
    A: VFile + ?Sized + Unpin,
{
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        read_to_end_internal(Pin::new(&mut this.reader), cx, this.buf)
    }
}

/// Future for the [`write_all`](super::AsyncWriteExt::write_all) method.
#[derive(Debug)]
pub struct WriteAll<'a, W: ?Sized + Unpin> {
    writer: &'a mut W,
    buf: &'a [u8],
}

impl<W: ?Sized + Unpin> Unpin for WriteAll<'_, W> {}

impl<'a, W: VFile + ?Sized + Unpin> WriteAll<'a, W> {
    pub(super) fn new(writer: &'a mut W, buf: &'a [u8]) -> Self {
        WriteAll { writer, buf }
    }
}

impl<W: VFile + ?Sized + Unpin> Future for WriteAll<'_, W> {
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let this = &mut *self;
        while !this.buf.is_empty() {
            let n = ready!(Pin::new(&mut this.writer).poll_write(cx, this.buf))?;
            {
                let (_, rest) = core::mem::replace(&mut this.buf, &[]).split_at(n);
                this.buf = rest;
            }
            if n == 0 {
                return Poll::Ready(Err(ErrorKind::WriteZero.into()));
            }
        }

        Poll::Ready(Ok(()))
    }
}

pin_project! {
    pub struct Seek<'a W: VFile> {
        #[pin]
        file: &'a mut W,
        pos: SeekFrom
    }
}

impl<'a, W: VFile + Unpin> Future for Seek<'a, W> {
    type Output = Result<u64, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.file.poll_seek(cx, *this.pos)
    }
}
