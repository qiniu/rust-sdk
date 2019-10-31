use qiniu_http::Error as HTTPError;
use std::{
    io::{Error as IOError, ErrorKind as IOErrorKind, Read},
    marker::PhantomData,
    sync::Mutex,
};

pub(super) enum Result {
    Success,
    IOError(IOError),
    HTTPError(HTTPError),
}

enum Status<'r, R: Read + Send + Sync + 'r> {
    Uploading(R, PhantomData<&'r R>),
    IOError(IOError),
    HTTPError(HTTPError),
    Success,
}

pub(super) struct IOStatusManager<'r, R: Read + Send + Sync + 'r> {
    inner: Mutex<Status<'r, R>>,
}

impl<'r, R: Read + Send + Sync + 'r> IOStatusManager<'r, R> {
    pub(super) fn new(io: R) -> IOStatusManager<'r, R> {
        IOStatusManager {
            inner: Mutex::new(Status::Uploading(io, PhantomData)),
        }
    }

    pub(super) fn read(&self, buf: &mut [u8]) -> Option<usize> {
        let mut lock = self.inner.lock().unwrap();
        match &mut *lock {
            Status::Uploading(io, _) => {
                let mut have_read = 0;
                loop {
                    match io.read(&mut buf[have_read..]) {
                        Ok(0) => {
                            *lock = Status::Success;
                            if have_read > 0 {
                                return Some(have_read);
                            } else {
                                return None;
                            }
                        }
                        Ok(n) => {
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
}
