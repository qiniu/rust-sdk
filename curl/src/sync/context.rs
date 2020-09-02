use super::CurlHTTPCaller;
use curl::easy::{Handler, ReadError, SeekResult, WriteError};
use once_cell::sync::Lazy;
use qiniu_http::{HeaderName, HeaderValue, HeadersOwned, Method, Request, StatusCode};
use std::{
    env::temp_dir,
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    mem::take,
    path::{Path, PathBuf},
};
use tempfile::tempfile_in;

static TEMP_DIR: Lazy<PathBuf> = Lazy::new(temp_dir);

pub(super) enum ResponseBody {
    Bytes(Vec<u8>),
    File(File),
}

impl Default for ResponseBody {
    #[inline]
    fn default() -> Self {
        Self::Bytes(Vec::new())
    }
}

enum ProgressStatus {
    Initialized,
    Uploading(u64),
    Downloading(u64),
    Completed,
}

impl Default for ProgressStatus {
    #[inline]
    fn default() -> Self {
        Self::Initialized
    }
}

type OnProgress<'r> = Option<&'r (dyn Fn(u64, u64) -> bool + Send + Sync)>;
type OnBody<'r> = Option<&'r (dyn Fn(&[u8]) -> bool + Send + Sync)>;
type OnStatusCode<'r> = Option<&'r (dyn Fn(StatusCode) -> bool + Send + Sync)>;
type OnHeader<'r> = Option<&'r (dyn Fn(&HeaderName, &HeaderValue) -> bool + Send + Sync)>;

pub(super) struct Context<'r> {
    request_body: Cursor<&'r [u8]>,
    response_body: ResponseBody,
    response_headers: HeadersOwned,
    buffer_size: usize,
    temp_dir: &'r Path,
    progress_status: ProgressStatus,
    on_uploading_progress: OnProgress<'r>,
    on_downloading_progress: OnProgress<'r>,
    on_send_request_body: OnBody<'r>,
    on_receive_response_status: OnStatusCode<'r>,
    on_receive_response_body: OnBody<'r>,
    on_receive_response_header: OnHeader<'r>,
}

impl Default for Context<'_> {
    #[inline]
    fn default() -> Self {
        Self {
            buffer_size: 1 << 22,
            temp_dir: TEMP_DIR.as_ref(),
            request_body: Default::default(),
            response_body: Default::default(),
            response_headers: Default::default(),
            progress_status: Default::default(),
            on_uploading_progress: Default::default(),
            on_downloading_progress: Default::default(),
            on_send_request_body: Default::default(),
            on_receive_response_status: Default::default(),
            on_receive_response_body: Default::default(),
            on_receive_response_header: Default::default(),
        }
    }
}

impl<'ctx> Context<'ctx> {
    pub(super) fn take_response_headers(&mut self) -> HeadersOwned {
        take(&mut self.response_headers)
    }

    pub(super) fn take_response_body(&mut self) -> ResponseBody {
        take(&mut self.response_body)
    }

    pub(super) fn reset<'r: 'ctx>(&mut self, client: &'r CurlHTTPCaller, request: &'r Request<'r>) {
        *self = Default::default();
        self.buffer_size = client.buffer_size();
        self.temp_dir = client
            .temp_dir
            .as_ref()
            .map(|dir| dir.as_path())
            .unwrap_or_else(|| &TEMP_DIR);
        self.request_body = Cursor::new(request.body());
        if request.method() != Method::HEAD {
            self.response_body = ResponseBody::Bytes(Vec::with_capacity(self.buffer_size));
        }
        self.on_uploading_progress = request.on_uploading_progress();
        self.on_downloading_progress = request.on_downloading_progress();
        self.on_send_request_body = request.on_send_request_body();
        self.on_receive_response_status = request.on_receive_response_status();
        self.on_receive_response_body = request.on_receive_response_body();
        self.on_receive_response_header = request.on_receive_response_header();
    }
}

impl Handler for Context<'_> {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        match &mut self.response_body {
            ResponseBody::Bytes(bytes) => {
                if bytes.len() + data.len() > self.buffer_size {
                    let mut tmpfile = tempfile_in(self.temp_dir).map_err(|_| WriteError::Pause)?;
                    tmpfile.write_all(bytes).map_err(|_| WriteError::Pause)?;
                    tmpfile.write_all(data).map_err(|_| WriteError::Pause)?;
                    self.response_body = ResponseBody::File(tmpfile);
                } else {
                    bytes.extend_from_slice(data);
                }
                if !self.on_receive_response_body.map_or(true, |f| f(data)) {
                    return Err(WriteError::Pause);
                }
                Ok(data.len())
            }
            ResponseBody::File(file) => {
                file.write(data)
                    .map_err(|_| WriteError::Pause)
                    .and_then(|len| {
                        if !self
                            .on_receive_response_body
                            .map_or(true, |f| f(data.get(0..len).unwrap()))
                        {
                            return Err(WriteError::Pause);
                        }
                        Ok(len)
                    })
            }
        }
    }

    fn read(&mut self, data: &mut [u8]) -> Result<usize, ReadError> {
        self.request_body
            .read(data)
            .map_err(|_| ReadError::Abort)
            .and_then(|len| {
                if !self
                    .on_send_request_body
                    .map_or(true, |f| f(data.get(0..len).unwrap()))
                {
                    return Err(ReadError::Abort);
                }
                Ok(len)
            })
    }

    fn seek(&mut self, whence: SeekFrom) -> SeekResult {
        self.request_body
            .seek(whence)
            .map_or_else(|_| SeekResult::Fail, |_| SeekResult::Ok)
    }

    fn header(&mut self, data: &[u8]) -> bool {
        let header = match String::from_utf8(data.to_vec()) {
            Ok(header) => header,
            Err(_) => {
                return false;
            }
        };
        if header == "\r\n" {
            return true;
        } else if header.starts_with("HTTP/") {
            if let Some(on_receive_response_status) = self.on_receive_response_status {
                if !header
                    .split_whitespace()
                    .take(2)
                    .nth(1)
                    .and_then(|code| code.parse::<StatusCode>().ok())
                    .map_or(false, on_receive_response_status)
                {
                    return false;
                }
            }
            self.response_headers.clear();
            return true;
        }
        let (header_name, header_value) = {
            let mut iter = header
                .trim_matches(char::is_whitespace)
                .splitn(2, ':')
                .take(2)
                .map(|s| s.trim_matches(char::is_whitespace));
            (iter.next(), iter.next())
        };
        if let (Some(header_name), Some(header_value)) = (header_name, header_value) {
            let header_name = header_name.to_string();
            let header_value = header_value.to_string();
            if let Some(on_receive_response_header) = self.on_receive_response_header {
                on_receive_response_header(
                    &header_name.as_str().into(),
                    &header_value.as_str().into(),
                );
            }
            self.response_headers
                .insert(header_name.into(), header_value.into());
            true
        } else {
            false
        }
    }

    fn progress(&mut self, dltotal: f64, dlnow: f64, ultotal: f64, ulnow: f64) -> bool {
        let dltotal = dltotal as u64;
        let dlnow = dlnow as u64;
        let ultotal = ultotal as u64;
        let ulnow = ulnow as u64;

        if dltotal == 0 && ultotal == 0 {
            return true;
        }
        match self.progress_status {
            ProgressStatus::Initialized => {
                if ultotal == 0 {
                    if let Some(on_downloading_progress) = self.on_downloading_progress {
                        on_downloading_progress(dlnow, dltotal);
                    }
                    if dlnow == dltotal {
                        self.progress_status = ProgressStatus::Completed;
                    } else {
                        self.progress_status = ProgressStatus::Downloading(dlnow);
                    }
                } else {
                    if let Some(on_uploading_progress) = self.on_uploading_progress {
                        on_uploading_progress(ulnow, ultotal);
                    }
                    self.progress_status = ProgressStatus::Uploading(ulnow);
                }
            }
            ProgressStatus::Uploading(now) if now < ulnow => {
                if let Some(on_uploading_progress) = self.on_uploading_progress {
                    on_uploading_progress(ulnow, ultotal);
                }
                if ulnow == ultotal {
                    self.progress_status = ProgressStatus::Downloading(dlnow);
                } else {
                    self.progress_status = ProgressStatus::Uploading(ulnow);
                }
            }
            ProgressStatus::Downloading(now) if now < dlnow => {
                if let Some(on_downloading_progress) = self.on_downloading_progress {
                    on_downloading_progress(dlnow, dltotal);
                }
                if dlnow == dltotal {
                    self.progress_status = ProgressStatus::Completed;
                } else {
                    self.progress_status = ProgressStatus::Downloading(dlnow);
                }
            }
            _ => {}
        }
        true
    }
}
