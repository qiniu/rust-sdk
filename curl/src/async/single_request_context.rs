use super::{
    super::{
        http::{
            AsyncFile, AsyncResponseBuilder, AsyncResponseResult, HeaderName, HeaderValue,
            HeadersOwned, Request, RequestBody, ResponseError, ResponseErrorKind, StatusCode,
        },
        utils::{easy::handle, header},
    },
    CurlHTTPCaller,
};
use curl::{
    easy::{Easy2, Handler, ReadError, SeekResult, WriteError},
    Error as CurlError,
};
use futures::{
    channel::oneshot::{channel, Sender},
    executor::block_on,
    future::BoxFuture,
    io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt, Cursor as AsyncCursor},
    pin_mut,
};
use once_cell::sync::Lazy;
use sluice::pipe::{pipe, PipeReader, PipeWriter};
use std::{
    borrow::Cow,
    env::temp_dir,
    ffi::CStr,
    fmt,
    io::{Error as IOError, ErrorKind as IOErrorKind, Result as IOResult, SeekFrom},
    mem::take,
    os::raw::{c_char, c_long},
    path::{Path, PathBuf},
    pin::Pin,
    ptr::{null, null_mut},
    result::Result,
    str::from_utf8,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc,
    },
    task::{Context, Poll, Waker},
};
use tempfile::tempfile_in;

static TEMP_DIR: Lazy<PathBuf> = Lazy::new(temp_dir);

#[derive(Debug, Copy, Clone)]
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

type OnProgress<'a> = Option<&'a (dyn Fn(u64, u64) -> bool + Send + Sync)>;
type OnBody<'a> = Option<&'a (dyn Fn(&[u8]) -> bool + Send + Sync)>;
type OnStatusCode<'a> = Option<&'a (dyn Fn(StatusCode) -> bool + Send + Sync)>;
type OnHeader<'a> = Option<&'a (dyn Fn(&HeaderName, &HeaderValue) -> bool + Send + Sync)>;

pub(super) struct SingleRequestContext<'ctx> {
    sender: Option<Sender<Result<AsyncResponseBuilder, ResponseError>>>,
    err_sender: Option<Sender<Result<(), ResponseError>>>,
    statuses: Arc<Statuses>,
    request_body_waker: Option<Waker>,
    response_body_waker: Option<Waker>,
    request_body: AsyncCursor<RequestBody<'ctx>>,
    response_body_writer: Option<PipeWriter>,
    response_headers: HeadersOwned,
    progress_status: ProgressStatus,
    on_uploading_progress: OnProgress<'ctx>,
    on_downloading_progress: OnProgress<'ctx>,
    on_send_request_body: OnBody<'ctx>,
    on_receive_response_status: OnStatusCode<'ctx>,
    on_receive_response_body: OnBody<'ctx>,
    on_receive_response_header: OnHeader<'ctx>,
    raw: *mut curl_sys::CURL,
}

unsafe impl Send for SingleRequestContext<'_> {}

impl<'ctx> SingleRequestContext<'ctx> {
    pub(super) fn new(
        http_client: &CurlHTTPCaller,
        request: &'ctx Request<'ctx>,
    ) -> (Easy2<Self>, BoxFuture<'ctx, AsyncResponseResult>) {
        let (sender, receiver) = channel();
        let (err_sender, err_receiver) = channel();
        let (response_body_reader, response_body_writer) = pipe();
        let statuses: Arc<Statuses> = Arc::new(Statuses::new(
            http_client.buffer_size(),
            http_client.temp_dir().map(|s| s.to_owned()).map(Cow::Owned),
        ));
        let context = Self {
            sender: Some(sender),
            err_sender: Some(err_sender),
            statuses: statuses.to_owned(),
            request_body_waker: None,
            response_body_waker: None,
            request_body: AsyncCursor::new(Cow::Borrowed(request.body().as_ref())),
            response_body_writer: Some(response_body_writer),
            response_headers: Default::default(),
            progress_status: Default::default(),
            on_uploading_progress: request.on_uploading_progress(),
            on_downloading_progress: request.on_downloading_progress(),
            on_send_request_body: request.on_send_request_body(),
            on_receive_response_status: request.on_receive_response_status(),
            on_receive_response_body: request.on_receive_response_body(),
            on_receive_response_header: request.on_receive_response_header(),
            raw: null_mut(),
        };
        let mut easy = Easy2::new(context);
        let raw = easy.raw();
        easy.get_mut().raw = raw;
        let future = async move {
            let response_builder = receiver
                .await
                .map_err(|err| ResponseError::new(ResponseErrorKind::UnknownError, err))??;
            let reader = AsyncResponseBodyReader {
                pipe_reader: response_body_reader,
                statuses,
            };
            let builder = reader.set_body(response_builder).await?;
            if let Ok(result) = err_receiver.await {
                result?;
            }
            Ok(builder.build())
        };
        (easy, Box::pin(future))
    }

    #[inline]
    pub(super) fn set_wakers(&mut self, request_body_waker: Waker, response_body_waker: Waker) {
        self.request_body_waker = Some(request_body_waker);
        self.response_body_waker = Some(response_body_waker);
    }

    #[inline]
    fn is_future_canceled(&self) -> bool {
        self.sender
            .as_ref()
            .map(|sender| sender.is_canceled())
            .unwrap_or(false)
    }

    #[inline]
    fn take_response_headers(&mut self) -> HeadersOwned {
        take(&mut self.response_headers)
    }
}

pub(super) fn handle_response(easy: &mut Easy2<SingleRequestContext>, error: Option<CurlError>) {
    easy.get_ref().statuses.completed.store(true, Relaxed);
    if let Some(error) = error {
        complete_response(easy.get_mut(), handle(Err(error)));
    } else {
        build_response(easy.get_mut());
    }
}

fn build_response(ctx: &mut SingleRequestContext) {
    if ctx.sender.is_some() {
        let result = _build_response(ctx);
        return complete_response(ctx, result);
    }

    fn _build_response(
        ctx: &mut SingleRequestContext,
    ) -> Result<AsyncResponseBuilder, ResponseError> {
        let status_code = handle(get_response_code(ctx.raw))?;
        let server_ip = handle(get_primary_ip(ctx.raw).map(|s| s.and_then(|s| s.parse().ok())))?;
        let server_port = handle(get_primary_port(ctx.raw))?;

        Ok(AsyncResponseBuilder::default()
            .status_code(status_code)
            .headers(ctx.take_response_headers())
            .server_ip(server_ip)
            .server_port(server_port))
    }

    #[inline]
    fn get_response_code(raw: *mut curl_sys::CURL) -> Result<StatusCode, CurlError> {
        getopt_long(raw, curl_sys::CURLINFO_RESPONSE_CODE).map(|c| c as StatusCode)
    }

    #[inline]
    fn get_primary_ip<'a>(raw: *mut curl_sys::CURL) -> Result<Option<&'a str>, CurlError> {
        getopt_str(raw, curl_sys::CURLINFO_PRIMARY_IP)
    }

    #[inline]
    fn get_primary_port(raw: *mut curl_sys::CURL) -> Result<u16, CurlError> {
        getopt_long(raw, curl_sys::CURLINFO_PRIMARY_PORT).map(|c| c as u16)
    }

    #[inline]
    fn getopt_long(raw: *mut curl_sys::CURL, opt: curl_sys::CURLINFO) -> Result<c_long, CurlError> {
        let mut p = 0;
        verify_code(unsafe { curl_sys::curl_easy_getinfo(raw, opt, &mut p) })?;
        Ok(p)
    }

    #[inline]
    fn getopt_str<'a>(
        raw: *mut curl_sys::CURL,
        opt: curl_sys::CURLINFO,
    ) -> Result<Option<&'a str>, CurlError> {
        match getopt_bytes(raw, opt) {
            Ok(Some(bytes)) => from_utf8(bytes).map_or_else(
                |_| Err(CurlError::new(curl_sys::CURLE_CONV_FAILED)),
                |s| Ok(Some(s)),
            ),
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
    }

    #[inline]
    fn getopt_bytes<'b>(
        raw: *mut curl_sys::CURL,
        opt: curl_sys::CURLINFO,
    ) -> Result<Option<&'b [u8]>, CurlError> {
        let p = getopt_ptr(raw, opt)?;
        if p.is_null() {
            Ok(None)
        } else {
            Ok(Some(unsafe { CStr::from_ptr(p) }.to_bytes()))
        }
    }

    #[inline]
    fn getopt_ptr(
        raw: *mut curl_sys::CURL,
        opt: curl_sys::CURLINFO,
    ) -> Result<*const c_char, CurlError> {
        let mut p: *const c_char = null();
        verify_code(unsafe { curl_sys::curl_easy_getinfo(raw, opt, &mut p) })?;
        Ok(p)
    }

    #[inline]
    fn verify_code(code: curl_sys::CURLcode) -> Result<(), CurlError> {
        if code == curl_sys::CURLE_OK {
            Ok(())
        } else {
            Err(CurlError::new(code))
        }
    }
}

fn complete_response(
    context: &mut SingleRequestContext,
    result: Result<AsyncResponseBuilder, ResponseError>,
) {
    if let Some(sender) = context.sender.take() {
        sender.send(result).ok();
    } else if let Err(err) = result {
        if let Some(err_sender) = context.err_sender.take() {
            err_sender.send(Err(err)).ok();
        }
    }
}

impl fmt::Debug for SingleRequestContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SingleRequestContext").finish()
    }
}

impl<'ctx> Handler for SingleRequestContext<'ctx> {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        if self.statuses.dropped.load(Relaxed) || self.statuses.canceled.load(Relaxed) {
            return Ok(0);
        }
        build_response(self);

        if let (Some(waker), Some(response_body_writer)) = (
            self.response_body_waker.as_ref(),
            self.response_body_writer.as_mut(),
        ) {
            let mut context = Context::from_waker(waker);
            match Pin::new(response_body_writer).poll_write(&mut context, data) {
                Poll::Pending => Err(WriteError::Pause),
                Poll::Ready(Ok(len)) => {
                    if len > 0
                        && !self
                            .on_receive_response_body
                            .map_or(true, |f| f(data.get(..len).unwrap()))
                    {
                        self.statuses.canceled.store(true, Relaxed);
                        return Ok(0);
                    }
                    Ok(len)
                }
                Poll::Ready(Err(_)) => Ok(0),
            }
        } else {
            panic!("SingleRequestContext is not initialized");
        }
    }

    fn read(&mut self, data: &mut [u8]) -> Result<usize, ReadError> {
        if self.is_future_canceled() || self.statuses.canceled.load(Relaxed) {
            return Err(ReadError::Abort);
        }
        if let Some(waker) = &self.request_body_waker {
            let mut context = Context::from_waker(waker);
            match Pin::new(&mut self.request_body).poll_read(&mut context, data) {
                Poll::Pending => Err(ReadError::Pause),
                Poll::Ready(Ok(len)) => {
                    if !self
                        .on_send_request_body
                        .map_or(true, |f| f(data.get(..len).unwrap()))
                    {
                        self.statuses.canceled.store(true, Relaxed);
                        return Err(ReadError::Abort);
                    }
                    Ok(len)
                }
                Poll::Ready(Err(_)) => Err(ReadError::Abort),
            }
        } else {
            panic!("SingleRequestContext is not initialized");
        }
    }

    fn seek(&mut self, whence: SeekFrom) -> SeekResult {
        if self.is_future_canceled() || self.statuses.canceled.load(Relaxed) {
            SeekResult::Fail
        } else {
            block_on(async { self.request_body.seek(whence).await })
                .map_or_else(|_| SeekResult::Fail, |_| SeekResult::Ok)
        }
    }

    fn header(&mut self, line: &[u8]) -> bool {
        if self.is_future_canceled() || self.statuses.canceled.load(Relaxed) {
            false
        } else if header::is_ended_line(line) {
            true
        } else if header::is_status_line(line) {
            if let (Some(on_receive_response_status), Some(status_code)) = (
                self.on_receive_response_status,
                header::parse_status_line(line),
            ) {
                if !on_receive_response_status(status_code) {
                    self.statuses.canceled.store(true, Relaxed);
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
                self.statuses.canceled.store(true, Relaxed);
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
        if self.is_future_canceled() || self.statuses.canceled.load(Relaxed) {
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
            self.statuses.canceled.store(true, Relaxed);
        }

        result
    }
}

#[derive(Debug)]
struct Statuses {
    buffer_size: usize,
    temp_dir: Cow<'static, Path>,
    completed: AtomicBool,
    canceled: AtomicBool,
    dropped: AtomicBool,
}

impl Statuses {
    #[inline]
    fn new(buffer_size: usize, temp_dir: Option<Cow<'static, Path>>) -> Self {
        let mut statuses = Self::default();
        statuses.buffer_size = buffer_size;
        if let Some(temp_dir) = temp_dir {
            statuses.temp_dir = temp_dir;
        }
        statuses
    }
}

impl Default for Statuses {
    #[inline]
    fn default() -> Self {
        Self {
            buffer_size: 1 << 22,
            temp_dir: Cow::Borrowed(TEMP_DIR.as_ref()),
            completed: AtomicBool::new(false),
            canceled: AtomicBool::new(false),
            dropped: AtomicBool::new(false),
        }
    }
}

#[derive(Debug)]
struct AsyncResponseBodyReader {
    pipe_reader: PipeReader,
    statuses: Arc<Statuses>,
}

const USER_CANCELED_MESSAGE: &str = "User Canceled";

impl AsyncRead for AsyncResponseBodyReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<IOResult<usize>> {
        if self.statuses.canceled.load(Relaxed) {
            return Poll::Ready(Err(IOError::new(
                IOErrorKind::Interrupted,
                USER_CANCELED_MESSAGE,
            )));
        }

        let reader = &mut self.pipe_reader;
        pin_mut!(reader);

        match reader.poll_read(cx, buf) {
            Poll::Ready(Ok(0)) => {
                if self.statuses.completed.load(Relaxed) {
                    Poll::Ready(Ok(0))
                } else {
                    Poll::Ready(Err(IOError::new(
                        IOErrorKind::ConnectionAborted,
                        "Response is not completed but poll_read() returns zero",
                    )))
                }
            }
            poll => poll,
        }
    }
}

impl AsyncResponseBodyReader {
    async fn set_body(
        self,
        builder: AsyncResponseBuilder,
    ) -> Result<AsyncResponseBuilder, ResponseError> {
        self._set_body(builder).await.map_err(|err| {
            if err.kind() == IOErrorKind::Interrupted && err.to_string() == USER_CANCELED_MESSAGE {
                ResponseError::new(ResponseErrorKind::UserCanceled, err)
            } else {
                ResponseError::new(ResponseErrorKind::LocalIOError, err)
            }
        })
    }

    async fn _set_body(mut self, builder: AsyncResponseBuilder) -> IOResult<AsyncResponseBuilder> {
        match self.get_response_body().await {
            Ok(response_body) => match response_body {
                ResponseBody::Bytes(bytes) => Ok(builder.bytes_as_body(bytes)),
                ResponseBody::File(file) => builder.file_as_body(file).await,
            },
            Err(err) => Err(err),
        }
    }

    async fn get_response_body(&mut self) -> IOResult<ResponseBody> {
        let mut response_body = ResponseBody::default();
        let mut buffer = vec![0; self.statuses.buffer_size];

        loop {
            match self.read(&mut buffer).await {
                Ok(0) => {
                    return Ok(response_body);
                }
                Ok(len) => match &mut response_body {
                    ResponseBody::Bytes(bytes) => {
                        if bytes.len() + len > buffer.capacity() {
                            let mut tmpfile =
                                AsyncFile::from(tempfile_in(self.statuses.temp_dir.as_ref())?);
                            tmpfile.write_all(bytes).await?;
                            tmpfile.write_all(&buffer[..len]).await?;
                            response_body = ResponseBody::File(tmpfile);
                        } else {
                            bytes.extend_from_slice(&buffer[..len])
                        }
                    }
                    ResponseBody::File(file) => file.write_all(&buffer[..len]).await?,
                },
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }
}

enum ResponseBody {
    Bytes(Vec<u8>),
    File(AsyncFile),
}

impl Default for ResponseBody {
    #[inline]
    fn default() -> Self {
        Self::Bytes(Vec::new())
    }
}

impl Drop for AsyncResponseBodyReader {
    fn drop(&mut self) {
        self.statuses.dropped.store(true, Relaxed);
    }
}
