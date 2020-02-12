use delegate::delegate;
use std::io::{Error as IOError, ErrorKind as IOErrorKind, IoSliceMut, Read, Result as IOResult, Seek, SeekFrom};

const NOT_IMPLEMENTED: &str = "Not Implemented";

pub struct SeekAdapter<R: Read>(pub R);

impl<R: Read> Read for SeekAdapter<R> {
    delegate! {
        target self.0 {
            fn read(&mut self, buf: &mut [u8]) -> IOResult<usize>;
            fn read_vectored(&mut self, bufs: &mut [IoSliceMut]) -> IOResult<usize>;
            fn read_to_end(&mut self, buf: &mut Vec<u8>) -> IOResult<usize>;
            fn read_to_string(&mut self, buf: &mut String) -> IOResult<usize>;
            fn read_exact(&mut self, buf: &mut [u8]) -> IOResult<()>;
        }
    }
}

impl<R: Read> Seek for SeekAdapter<R> {
    fn seek(&mut self, _pos: SeekFrom) -> IOResult<u64> {
        Err(IOError::new(IOErrorKind::Other, NOT_IMPLEMENTED))
    }
}
