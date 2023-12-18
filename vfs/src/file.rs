use alloc::vec::Vec;

use crate::{
    error::{Error, ErrorKind},
    types::SeekFrom,
};

pub trait VFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error>;
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error>;
    fn flush(&mut self) -> Result<(), Error>;
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error>;
    fn close(&mut self) -> Result<(), Error>;
}

pub trait VFileExt: VFile {
    fn read_to_end(&mut self, buffer: &mut Vec<u8>) -> Result<usize, Error> {
        read_to_end(self, buffer)
    }
    fn write_all(&mut self, mut buf: &[u8]) -> Result<(), Error> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => {
                    return Err(Error::new_const(
                        ErrorKind::WriteZero,
                        &"failed to write whole buffer",
                    ));
                }
                Ok(n) => buf = &buf[n..],
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

impl<T> VFileExt for T where T: VFile {}

const DEFAULT_BUF_SIZE: usize = 8 * 1024;

pub(crate) struct Guard<'a> {
    pub buf: &'a mut Vec<u8>,
    pub len: usize,
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        unsafe {
            self.buf.set_len(self.len);
        }
    }
}

fn read_to_end<R: VFile + ?Sized>(r: &mut R, buf: &mut Vec<u8>) -> Result<usize, Error> {
    read_to_end_with_reservation(r, buf, |_| 32)
}

fn read_to_end_with_reservation<R, F>(
    r: &mut R,
    buf: &mut Vec<u8>,
    mut reservation_size: F,
) -> Result<usize, Error>
where
    R: VFile + ?Sized,
    F: FnMut(&R) -> usize,
{
    let start_len = buf.len();
    let mut g = Guard {
        len: buf.len(),
        buf,
    };
    loop {
        if g.len == g.buf.len() {
            unsafe {
                g.buf.reserve(reservation_size(r));
                let capacity = g.buf.capacity();
                g.buf.set_len(capacity);
                // r.initializer().initialize(&mut g.buf[g.len..]);
            }
        }

        let buf = &mut g.buf[g.len..];
        match r.read(buf) {
            Ok(0) => return Ok(g.len - start_len),
            Ok(n) => {
                // We can't allow bogus values from read. If it is too large, the returned vec could have its length
                // set past its capacity, or if it overflows the vec could be shortened which could create an invalid
                // string if this is called via read_to_string.
                assert!(n <= buf.len());
                g.len += n;
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
}
