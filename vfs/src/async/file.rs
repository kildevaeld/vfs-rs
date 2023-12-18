use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};


use alloc::{vec::Vec, boxed::Box};
use futures_core::ready;

use crate::{
    error::{Error, ErrorKind},
    file::Guard,
    types::SeekFrom,
};

pub trait VAsyncFile: Send + Sync {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>>;

    fn poll_seek(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<Result<u64, Error>>;

    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>>;

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>>;

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>>;
}

impl<'a, T> VAsyncFile for &'a mut T
where
    T: VAsyncFile + ?Sized + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut **self).poll_read(cx, buf)
    }

    fn poll_seek(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<Result<u64, Error>> {
        Pin::new(&mut **self).poll_seek(cx, pos)
    }

    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut **self).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut **self).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut **self).poll_close(cx)
    }
}

impl<T: ?Sized + VAsyncFile + Unpin> VAsyncFile for Box<T>

{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut **self).poll_read(cx, buf)
    }

    fn poll_seek(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<Result<u64, Error>> {
        Pin::new(&mut **self).poll_seek(cx, pos)
    }

    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(&mut **self).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut **self).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut **self).poll_close(cx)
    }
}

impl<P> VAsyncFile for Pin<P>
where
    P: core::ops::DerefMut + Unpin + Send + Sync,
    P::Target: VAsyncFile,{
    fn poll_read(
         self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        self.get_mut().as_mut().poll_read(cx, buf)
    }

    fn poll_seek(
         self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<Result<u64, Error>> {
        self.get_mut().as_mut().poll_seek(cx, pos)
    }

    fn poll_write(
         self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        self.get_mut().as_mut().poll_write(cx, buf)
    }

    fn poll_flush( self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.get_mut().as_mut().poll_flush(cx)
    }

    fn poll_close( self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.get_mut().as_mut().poll_close(cx)
    }
}


pub trait VAsyncFileExt: VAsyncFile {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Read<'a, Self>
    where
        Self: Sized + Unpin,
    {
        Read { reader: self, buf }
    }
    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> ReadToEnd<'a, Self>
    where
        Self: Sized + Unpin,
    {
        ReadToEnd::new(self, buf)
    }

    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAll<'a, Self>
    where
        Self: Sized + Unpin,
    {
        WriteAll::new(self, buf)
    }
}

impl<T> VAsyncFileExt for T where T: VAsyncFile {}

pub struct Read<'a, T: VAsyncFile + ?Sized> {
    reader: &'a mut T,
    buf: &'a mut [u8],
}

impl<A> Future for Read<'_, A>
where
    A: VAsyncFile + ?Sized + Unpin,
{
    type Output = Result<usize, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        Pin::new(&mut this.reader).poll_read(cx, this.buf)
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
fn read_to_end_internal<R: VAsyncFile + ?Sized>(
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

#[derive(Debug)]
pub struct ReadToEnd<'a, R: ?Sized + Unpin> {
    reader: &'a mut R,
    buf: &'a mut Vec<u8>,
}

impl<R: ?Sized + Unpin> Unpin for ReadToEnd<'_, R> {}

impl<'a, R: VAsyncFile + ?Sized + Unpin> ReadToEnd<'a, R> {
    pub(super) fn new(reader: &'a mut R, buf: &'a mut Vec<u8>) -> Self {
        ReadToEnd { reader, buf }
    }
}

impl<A> Future for ReadToEnd<'_, A>
where
    A: VAsyncFile + ?Sized + Unpin,
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

impl<'a, W: VAsyncFile + ?Sized + Unpin> WriteAll<'a, W> {
    pub(super) fn new(writer: &'a mut W, buf: &'a [u8]) -> Self {
        WriteAll { writer, buf }
    }
}

impl<W: VAsyncFile + ?Sized + Unpin> Future for WriteAll<'_, W> {
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
