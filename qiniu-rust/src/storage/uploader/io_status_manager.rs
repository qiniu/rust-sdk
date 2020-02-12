use crate::http::Error as HTTPError;
use assert_impl::assert_impl;
use std::{
    collections::HashSet,
    convert::TryInto,
    io::{Error as IOError, ErrorKind as IOErrorKind, Read, Seek, SeekFrom},
    sync::Mutex,
};

pub(super) enum Result {
    Success,
    IOError(IOError),
    HTTPError(HTTPError),
}

enum Status<R: Read + Seek + Send> {
    Uploading {
        reader: R,
        block_size: u32,
        current_part_number: usize,
        uploaded_part_numbers: HashSet<usize>,
    },
    IOError(IOError),
    HTTPError(HTTPError),
    Success,
}

pub(super) struct IOStatusManager<R: Read + Seek + Send> {
    inner: Mutex<Status<R>>,
}

pub(super) struct PartData {
    pub(super) data: Vec<u8>,
    pub(super) part_number: usize,
}

impl<R: Read + Seek + Send> IOStatusManager<R> {
    pub(super) fn new(io: R, block_size: u32, uploaded_part_numbers: &[usize]) -> IOStatusManager<R> {
        IOStatusManager {
            inner: Mutex::new(Status::Uploading {
                reader: io,
                current_part_number: 0,
                block_size,
                uploaded_part_numbers: {
                    let mut set = HashSet::new();
                    for &part_number in uploaded_part_numbers {
                        set.insert(part_number);
                    }
                    set
                },
            }),
        }
    }

    pub(super) fn read(&self) -> Option<PartData> {
        let mut lock = self.inner.lock().unwrap();
        match &mut *lock {
            Status::Uploading {
                reader,
                block_size,
                current_part_number,
                uploaded_part_numbers,
            } => {
                let mut have_read = 0;
                let mut buf = vec![0; block_size.to_owned().try_into().unwrap_or(usize::max_value())];
                let new_part_number = {
                    let mut new_part_number = *current_part_number + 1;
                    let mut skip_bytes = 0i64;
                    while uploaded_part_numbers.get(&new_part_number).is_some() {
                        new_part_number += 1;
                        skip_bytes += *block_size as i64;
                    }
                    if skip_bytes > 0 {
                        if let Err(err) = reader.seek(SeekFrom::Current(skip_bytes)) {
                            *lock = Status::IOError(err);
                            return None;
                        }
                    }
                    new_part_number
                };
                loop {
                    match reader.read(&mut buf[have_read..]) {
                        Ok(0) => {
                            *lock = Status::Success;
                            if have_read > 0 {
                                buf.resize_with(have_read, Default::default);
                                return Some(PartData {
                                    data: buf,
                                    part_number: new_part_number,
                                });
                            } else {
                                return None;
                            }
                        }
                        Ok(n) => {
                            have_read += n;
                            if have_read == buf.len() {
                                *current_part_number = new_part_number;
                                return Some(PartData {
                                    data: buf,
                                    part_number: new_part_number,
                                });
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
            Status::Uploading { .. } => {
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

impl<R: Read + Seek + Send> Status<R> {
    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
    }
}
