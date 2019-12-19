use assert_impl::assert_impl;
use qiniu_http::Error as HTTPError;
use std::{
    io::{Error as IOError, ErrorKind as IOErrorKind, Read},
    sync::Mutex,
};

pub(super) enum Result {
    Success,
    IOError(IOError),
    HTTPError(HTTPError),
}

enum Status<R: Read + Send> {
    Uploading(R, usize),
    IOError(IOError),
    HTTPError(HTTPError),
    Success,
}

pub(super) struct IOStatusManager<R: Read + Send> {
    inner: Mutex<Status<R>>,
}

impl<R: Read + Send> IOStatusManager<R> {
    pub(super) fn new(io: R) -> IOStatusManager<R> {
        IOStatusManager {
            inner: Mutex::new(Status::Uploading(io, 0)),
        }
    }

    pub(super) fn read(&self, buf: &mut [u8], c: &mut usize) -> Option<usize> {
        let mut lock = self.inner.lock().unwrap();
        match &mut *lock {
            Status::Uploading(io, count) => {
                let mut have_read = 0;
                loop {
                    match io.read(&mut buf[have_read..]) {
                        Ok(0) => {
                            *c = *count + 1;
                            *lock = Status::Success;
                            if have_read > 0 {
                                return Some(have_read);
                            } else {
                                return None;
                            }
                        }
                        Ok(n) => {
                            *count += 1;
                            *c = *count;
                            have_read += n;
                            if have_read == buf.len() {
                                return Some(have_read);
                            }
                        }
                        Err(ref err) if err.kind() == IOErrorKind::Interrupted => {
                            continue;
                        }
                        Err(err) => {
                            *lock = Status::IOError(err);
                            return None;
                        }
                    }
                }
            }
            _ => None,
        }
    }

    pub(super) fn error(&self, err: HTTPError) {
        *self.inner.lock().unwrap() = Status::HTTPError(err);
    }

    pub(super) fn result(self) -> Result {
        match self.inner.into_inner().unwrap() {
            Status::Success => Result::Success,
            Status::IOError(err) => Result::IOError(err),
            Status::HTTPError(err) => Result::HTTPError(err),
            Status::Uploading(_, _) => {
                panic!("Unexpected uploading status of task_manager");
            }
        }
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl<R: Read + Send> Status<R> {
    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
    }
}
