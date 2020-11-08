use super::{
    super::{
        http::{HeaderName, HeaderValue, HeadersOwned, Method, Request, StatusCode},
        utils::header,
    },
    CurlHTTPCaller,
};
use curl::easy::{Handler, ReadError, SeekResult, WriteError};
use once_cell::sync::Lazy;
use std::{
    env::temp_dir,
    fs::{File, OpenOptions},
    io::{Cursor, Read, Result as IOResult, Seek, SeekFrom, Write},
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
    canceled: bool,
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
            canceled: false,
        }
    }
}

impl<'ctx> Context<'ctx> {
    #[inline]
    pub(super) fn take_response_headers(&mut self) -> HeadersOwned {
        take(&mut self.response_headers)
    }

    #[inline]
    pub(super) fn take_response_body(&mut self) -> ResponseBody {
        take(&mut self.response_body)
    }

    #[inline]
    pub(super) fn canceled(&self) -> bool {
        self.canceled
    }

    pub(super) fn reset<'r: 'ctx>(
        &mut self,
        client: &'r CurlHTTPCaller,
        request: &'r Request<'r>,
    ) -> IOResult<()> {
        *self = Default::default();
        self.buffer_size = client.buffer_size();
        self.temp_dir = client.temp_dir().unwrap_or_else(|| &TEMP_DIR);
        self.request_body = Cursor::new(request.body());
        if let Some(response_body_buffer_path) = request.response_body_buffer_path() {
            self.response_body = ResponseBody::File(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(response_body_buffer_path)?,
            );
        } else if request.method() == Method::HEAD {
            self.response_body = ResponseBody::Bytes(Vec::new());
        } else {
            self.response_body = ResponseBody::Bytes(Vec::with_capacity(self.buffer_size));
        }
        self.on_uploading_progress = request.on_uploading_progress();
        self.on_downloading_progress = request.on_downloading_progress();
        self.on_send_request_body = request.on_send_request_body();
        self.on_receive_response_status = request.on_receive_response_status();
        self.on_receive_response_body = request.on_receive_response_body();
        self.on_receive_response_header = request.on_receive_response_header();
        self.canceled = false;
        Ok(())
    }
}

impl Handler for Context<'_> {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        if data.is_empty() || self.canceled {
            return Ok(0);
        }
        return _write(self, data).or(Ok(0));

        fn _write(context: &mut Context, data: &[u8]) -> Result<usize, WriteError> {
            match &mut context.response_body {
                ResponseBody::Bytes(bytes) => {
                    if bytes.len() + data.len() > context.buffer_size {
                        let mut tmpfile =
                            tempfile_in(context.temp_dir).map_err(|_| WriteError::Pause)?;
                        tmpfile.write_all(bytes).map_err(|_| WriteError::Pause)?;
                        tmpfile.write_all(data).map_err(|_| WriteError::Pause)?;
                        context.response_body = ResponseBody::File(tmpfile);
                    } else {
                        bytes.extend_from_slice(data);
                    }
                }
                ResponseBody::File(file) => {
                    file.write_all(data).map_err(|_| WriteError::Pause)?;
                }
            }
            if !context.on_receive_response_body.map_or(true, |f| f(data)) {
                context.canceled = true;
                return Err(WriteError::Pause);
            }
            Ok(data.len())
        }
    }

    fn read(&mut self, data: &mut [u8]) -> Result<usize, ReadError> {
        if self.canceled {
            return Err(ReadError::Abort);
        }

        self.request_body
            .read(data)
            .map_err(|_| ReadError::Abort)
            .and_then(|len| {
                if !self
                    .on_send_request_body
                    .map_or(true, |f| f(data.get(..len).unwrap()))
                {
                    self.canceled = true;
                    return Err(ReadError::Abort);
                }
                Ok(len)
            })
    }

    fn seek(&mut self, whence: SeekFrom) -> SeekResult {
        if self.canceled {
            return SeekResult::Fail;
        }

        self.request_body
            .seek(whence)
            .map_or_else(|_| SeekResult::Fail, |_| SeekResult::Ok)
    }

    fn header(&mut self, line: &[u8]) -> bool {
        if self.canceled {
            false
        } else if header::is_ended_line(line) {
            true
        } else if header::is_status_line(line) {
            if let (Some(on_receive_response_status), Some(status_code)) = (
                self.on_receive_response_status,
                header::parse_status_line(line),
            ) {
                if !on_receive_response_status(status_code) {
                    self.canceled = true;
                    return false;
                }
            }
            self.response_headers.clear();
            true
        } else if let Some((header_name, header_value)) = header::parse_header_line(line) {
            if !self
                .on_receive_response_header
                .map_or(true, |f| f(&header_name, &header_value))
            {
                self.canceled = true;
                return false;
            }
            self.response_headers
                .insert(header_name.into(), header_value.into());
            true
        } else {
            false
        }
    }

    fn progress(&mut self, dltotal: f64, dlnow: f64, ultotal: f64, ulnow: f64) -> bool {
        if self.canceled {
            return false;
        }

        let dltotal = dltotal as u64;
        let dlnow = dlnow as u64;
        let ultotal = ultotal as u64;
        let ulnow = ulnow as u64;
        let mut result = true;

        if dltotal == 0 && ultotal == 0 {
            return true;
        }
        match self.progress_status {
            ProgressStatus::Initialized => {
                if ultotal == 0 {
                    if let Some(on_downloading_progress) = self.on_downloading_progress {
                        result = on_downloading_progress(dlnow, dltotal);
                    }
                    if dlnow == dltotal {
                        self.progress_status = ProgressStatus::Completed;
                    } else {
                        self.progress_status = ProgressStatus::Downloading(dlnow);
                    }
                } else {
                    if let Some(on_uploading_progress) = self.on_uploading_progress {
                        result = on_uploading_progress(ulnow, ultotal);
                    }
                    self.progress_status = ProgressStatus::Uploading(ulnow);
                }
            }
            ProgressStatus::Uploading(now) if now < ulnow => {
                if let Some(on_uploading_progress) = self.on_uploading_progress {
                    result = on_uploading_progress(ulnow, ultotal);
                }
                if ulnow == ultotal {
                    self.progress_status = ProgressStatus::Downloading(dlnow);
                } else {
                    self.progress_status = ProgressStatus::Uploading(ulnow);
                }
            }
            ProgressStatus::Downloading(now) if now < dlnow => {
                if let Some(on_downloading_progress) = self.on_downloading_progress {
                    result = on_downloading_progress(dlnow, dltotal);
                }
                if dlnow == dltotal {
                    self.progress_status = ProgressStatus::Completed;
                } else {
                    self.progress_status = ProgressStatus::Downloading(dlnow);
                }
            }
            _ => {}
        }

        if !result {
            self.canceled = true;
        }

        result
    }
}
