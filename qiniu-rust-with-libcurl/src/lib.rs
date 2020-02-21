use curl::{
    easy::{Easy2, Handler, List, ReadError, SeekResult, WriteError},
    Version,
};
use derive_builder::Builder;
use lazy_static::lazy_static;
use object_pool::Pool;
use qiniu_http::{
    Error, ErrorKind, HTTPCaller, HTTPCallerErrorKind, Headers, Method, ProgressCallback, Request, Response,
    ResponseBuilder, Result, StatusCode,
};
use std::{
    convert::TryInto,
    default::Default,
    env,
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    mem::{size_of, transmute, transmute_copy},
    net::IpAddr,
    path::{Path, PathBuf},
    result,
    sync::Once,
};
use url::Url;

static INITIALIZER: Once = Once::new();
lazy_static! {
    static ref IPV6_SUPPORT: bool = Version::get().feature_ipv6();
    static ref MULTI_IP_ADDRS_SUPPORT: bool = Version::get().version_num() >= 0x07_3b_00;
    static ref FULL_USER_AGENT: Box<str> = format!(
        "QiniuRust/qiniu-http-{}/rust-{}/libcurl-{}",
        env!("CARGO_PKG_VERSION"),
        rustc_version_runtime::version(),
        Version::get().version(),
    )
    .into();
    static ref PART_USER_AGENT: Box<str> = format!("libcurl-{}", Version::get().version()).into();
    static ref TEMP_DIR: PathBuf = env::temp_dir();
    static ref CURL_POOL: Pool<'static, Easy2ContextRef> = Pool::new(16, Easy2ContextRef::default);
}

#[derive(Debug, Builder)]
#[builder(
    pattern = "owned",
    setter(into, strip_option),
    build_fn(name = "inner_build", private)
)]
pub struct CurlClient {
    #[builder(default = "1 << 22")]
    buffer_size: usize,

    #[builder(default)]
    temp_dir: Option<PathBuf>,

    #[builder(default, setter(skip))]
    user_agent: Option<String>,
}

impl HTTPCaller for CurlClient {
    fn call(&self, request: &Request) -> Result<Response> {
        let r: &mut Easy2ContextRef = &mut *CURL_POOL.pull();
        let mut easy: Box<Easy2<Context>> = r.into();
        self.reset_context(&mut easy);
        self.set_context(easy.get_mut(), request);
        let result = self.perform(&mut easy, request);
        let _: Easy2ContextRef = easy.into();
        result
    }
}

impl CurlClient {
    fn perform(&self, easy: &mut Easy2<Context>, request: &Request) -> Result<Response> {
        self.set_method(easy, request)?;
        self.set_url(easy, request)?;
        self.set_headers(easy, request)?;
        self.set_body(easy, request)?;
        self.set_options(easy, request)?;
        Self::handle_if_err(easy.perform(), request)?;
        let status_code = Self::handle_if_err(easy.response_code(), request)? as StatusCode;
        let server_ip: Option<IpAddr> =
            Self::handle_if_err(easy.primary_ip().map(|s| s.and_then(|s| s.parse().ok())), request)?;
        let server_port = Self::handle_if_err(easy.primary_port(), request)?;
        self.build_response(easy.get_mut(), request, status_code, server_ip, server_port)
    }

    fn build_response(
        &self,
        context: &mut Context,
        request: &Request,
        status_code: StatusCode,
        server_ip: Option<IpAddr>,
        server_port: u16,
    ) -> Result<Response> {
        let mut builder = ResponseBuilder::default().status_code(status_code);
        if let Some(response_headers) = context.response_headers.take() {
            builder = builder.headers(response_headers);
        }
        if let Some(response_body) = context.response_body.take() {
            match response_body {
                ResponseBody::Bytes(bytes) => {
                    builder = builder.bytes_as_body(bytes);
                }
                ResponseBody::File(file) => {
                    builder = builder
                        .file_as_body(file)
                        .map_err(|err| Error::new_unretryable_error(ErrorKind::IOError(err), request, None))?;
                }
            }
        }
        if let Some(server_ip) = server_ip {
            builder = builder.server_ip(server_ip);
        }
        builder = builder.server_port(server_port);
        Ok(builder.build())
    }

    fn reset_context<'r>(&'r self, easy: &mut Easy2<Context<'r>>) {
        easy.reset();
        let context = easy.get_mut();
        context.reset();
        context.buffer_size = self.buffer_size;
        context.temp_dir = self
            .temp_dir
            .as_ref()
            .map(|dir| dir.as_path())
            .unwrap_or_else(|| &TEMP_DIR);
    }

    fn set_context<'r>(&self, mut context: &mut Context<'r>, request: &'r Request<'r>) {
        context.upload_progress = request.on_uploading_progress();
        context.download_progress = request.on_downloading_progress();

        if let Some(request_body) = request.body().as_ref().map(|body| body.as_ref()) {
            if !request_body.is_empty() {
                context.request_body = Some(Cursor::new(request_body));
            }
        }

        match request.method() {
            Method::HEAD => (),
            _ => {
                context.response_body = Some(ResponseBody::Bytes(Vec::with_capacity(context.buffer_size)));
            }
        }
    }

    fn set_method<T>(&self, easy: &mut Easy2<T>, request: &Request) -> Result<()> {
        let result = match request.method() {
            Method::GET => easy.get(true),
            Method::HEAD => easy.nobody(true),
            Method::POST => easy.post(true),
            Method::PUT => easy.upload(true),
        };
        Self::handle_if_err(result, request)
    }

    fn set_url<T>(&self, easy: &mut Easy2<T>, request: &Request) -> Result<()> {
        Self::handle_if_err(easy.url(request.url()), request)
    }

    fn set_headers<T>(&self, easy: &mut Easy2<T>, request: &Request) -> Result<()> {
        let mut header_list = List::new();
        Self::handle_if_err(header_list.append("Expect:"), request)?;
        for (header_name, header_value) in request.headers().iter() {
            let h = header_name.as_ref().to_string() + ": " + header_value;
            Self::handle_if_err(header_list.append(&h), request)?;
        }
        Self::handle_if_err(easy.http_headers(header_list), request)
    }

    fn set_body<T>(&self, easy: &mut Easy2<T>, request: &Request) -> Result<()> {
        request
            .body()
            .as_ref()
            .map(|body| Self::handle_if_err(easy.post_field_size(body.len().try_into().unwrap()), request))
            .unwrap_or(Ok(()))
    }

    fn set_options<T>(&self, easy: &mut Easy2<T>, request: &Request) -> Result<()> {
        if !request.resolved_socket_addrs().is_empty() {
            let url = Url::parse(request.url()).unwrap();
            let mut addr =
                url.host_str().unwrap().to_owned() + ":" + &url.port_or_known_default().unwrap().to_string() + ":";
            for (i, socket_addr) in request.resolved_socket_addrs().iter().enumerate() {
                if !*IPV6_SUPPORT && socket_addr.is_ipv6() {
                    continue;
                }
                if i > 0 {
                    addr.push_str(",");
                }
                addr.push_str(&socket_addr.to_string());
                if !*MULTI_IP_ADDRS_SUPPORT {
                    break;
                }
            }
            if !addr.ends_with(':') {
                let mut list = List::new();
                Self::handle_if_err(list.append(&addr), request)?;
                Self::handle_if_err(easy.resolve(list), request)?;
            }
        }
        Self::handle_if_err(easy.accept_encoding(""), request)?;
        Self::handle_if_err(easy.transfer_encoding(true), request)?;
        Self::handle_if_err(easy.follow_location(request.follow_redirection()), request)?;
        Self::handle_if_err(easy.max_redirections(3), request)?;
        Self::handle_if_err(easy.connect_timeout(request.connect_timeout()), request)?;
        Self::handle_if_err(easy.timeout(request.request_timeout()), request)?;
        Self::handle_if_err(easy.tcp_keepalive(true), request)?;
        Self::handle_if_err(easy.tcp_keepidle(request.tcp_keepalive_idle_timeout()), request)?;
        Self::handle_if_err(easy.tcp_keepintvl(request.tcp_keepalive_probe_interval()), request)?;
        Self::handle_if_err(easy.low_speed_limit(request.low_transfer_speed()), request)?;
        Self::handle_if_err(easy.low_speed_time(request.low_transfer_speed_timeout()), request)?;
        Self::handle_if_err(
            easy.useragent(
                request
                    .user_agent()
                    .map(|user_agent| user_agent.to_owned() + &PART_USER_AGENT + "/")
                    .as_ref()
                    .map_or_else(|| (&FULL_USER_AGENT).as_ref(), |user_agent| user_agent.as_str()),
            ),
            request,
        )?;
        Self::handle_if_err(easy.show_header(false), request)?;
        Self::handle_if_err(
            easy.progress(request.on_uploading_progress().is_some() || request.on_downloading_progress().is_some()),
            request,
        )?;
        Ok(())
    }

    fn handle_if_err<T>(result: result::Result<T, curl::Error>, request: &Request) -> Result<T> {
        match result {
            Ok(result) => Ok(result),
            Err(err) => {
                if err.is_partial_file() || err.is_read_error() {
                    Err(Error::new_retryable_error(
                        ErrorKind::new_http_caller_error_kind(HTTPCallerErrorKind::UnknownError, err),
                        false,
                        request,
                        None,
                    ))
                } else if err.is_recv_error() {
                    Err(Error::new_retryable_error(
                        ErrorKind::new_http_caller_error_kind(HTTPCallerErrorKind::ResponseError, err),
                        false,
                        request,
                        None,
                    ))
                } else if err.is_write_error() || err.is_again() || err.is_chunk_failed() {
                    Err(Error::new_retryable_error(
                        ErrorKind::new_http_caller_error_kind(HTTPCallerErrorKind::UnknownError, err),
                        true,
                        request,
                        None,
                    ))
                } else if err.is_operation_timedout() {
                    Err(Error::new_retryable_error(
                        ErrorKind::new_http_caller_error_kind(HTTPCallerErrorKind::TimeoutError, err),
                        true,
                        request,
                        None,
                    ))
                } else if err.is_send_error() {
                    Err(Error::new_retryable_error(
                        ErrorKind::new_http_caller_error_kind(HTTPCallerErrorKind::RequestError, err),
                        true,
                        request,
                        None,
                    ))
                } else if err.is_too_many_redirects() || err.is_got_nothing() {
                    Err(Error::new_host_unretryable_error(
                        ErrorKind::new_http_caller_error_kind(HTTPCallerErrorKind::UnknownError, err),
                        true,
                        request,
                        None,
                    ))
                } else if err.is_couldnt_resolve_proxy() {
                    Err(Error::new_host_unretryable_error(
                        ErrorKind::new_http_caller_error_kind(HTTPCallerErrorKind::ResolveError, err),
                        true,
                        request,
                        None,
                    ))
                } else if err.is_couldnt_connect() {
                    Err(Error::new_host_unretryable_error(
                        ErrorKind::new_http_caller_error_kind(HTTPCallerErrorKind::ConnectionError, err),
                        true,
                        request,
                        None,
                    ))
                } else if err.is_ssl_connect_error()
                    || err.is_peer_failed_verification()
                    || err.is_ssl_engine_notfound()
                    || err.is_ssl_certproblem()
                    || err.is_ssl_cipher()
                    || err.is_ssl_cacert()
                    || err.is_use_ssl_failed()
                    || err.is_ssl_engine_initfailed()
                    || err.is_ssl_cacert_badfile()
                    || err.is_ssl_crl_badfile()
                    || err.is_ssl_shutdown_failed()
                    || err.is_ssl_issuer_error()
                {
                    Err(Error::new_host_unretryable_error(
                        ErrorKind::new_http_caller_error_kind(HTTPCallerErrorKind::SSLError, err),
                        true,
                        request,
                        None,
                    ))
                } else {
                    Err(Error::new_unretryable_error(
                        ErrorKind::new_http_caller_error_kind(HTTPCallerErrorKind::UnknownError, err),
                        request,
                        None,
                    ))
                }
            }
        }
    }
}

enum ProgressStatus {
    Initialized,
    Uploading(u64),
    Downloading(u64),
    Completed,
}

struct Context<'r> {
    request_body: Option<Cursor<&'r [u8]>>,
    response_body: Option<ResponseBody>,
    response_headers: Option<Headers<'static>>,
    buffer_size: usize,
    temp_dir: &'r Path,
    progress_status: ProgressStatus,
    upload_progress: Option<ProgressCallback<'r>>,
    download_progress: Option<ProgressCallback<'r>>,
}

enum ResponseBody {
    Bytes(Vec<u8>),
    File(File),
}

impl<'r> Handler for Context<'r> {
    fn write(&mut self, data: &[u8]) -> result::Result<usize, WriteError> {
        match &mut self.response_body {
            Some(ResponseBody::Bytes(bytes)) => {
                if bytes.len() + data.len() > self.buffer_size {
                    let mut tmpfile = tempfile::tempfile_in(&self.temp_dir).map_err(|_| WriteError::Pause)?;
                    tmpfile.write_all(bytes).map_err(|_| WriteError::Pause)?;
                    tmpfile.write_all(data).map_err(|_| WriteError::Pause)?;
                    self.response_body = Some(ResponseBody::File(tmpfile));
                } else {
                    bytes.extend_from_slice(data);
                }
            }
            Some(ResponseBody::File(file)) => {
                file.write_all(data).map_err(|_| WriteError::Pause)?;
            }
            _ => {}
        }
        Ok(data.len())
    }

    fn read(&mut self, data: &mut [u8]) -> result::Result<usize, ReadError> {
        self.request_body.as_mut().map_or(Ok(0), |request_body| {
            request_body.read(data).map_err(|_| ReadError::Abort)
        })
    }

    fn seek(&mut self, whence: SeekFrom) -> SeekResult {
        if let Some(request_body) = &mut self.request_body {
            match request_body.seek(whence) {
                Ok(_) => SeekResult::Ok,
                Err(_) => SeekResult::Fail,
            }
        } else {
            SeekResult::CantSeek
        }
    }

    fn header(&mut self, data: &[u8]) -> bool {
        let header = match String::from_utf8(data.to_vec()) {
            Ok(header) => header,
            Err(_) => {
                return false;
            }
        };
        if header.starts_with("HTTP/") {
            return true;
        }
        let mut iter = header
            .trim_matches(char::is_whitespace)
            .split(':')
            .take(2)
            .map(|s| s.trim_matches(char::is_whitespace));
        let header_name = iter.next();
        let header_value = iter.next();
        if let (Some(header_name), Some(header_value)) = (header_name, header_value) {
            if let Some(response_headers) = &mut self.response_headers {
                response_headers.insert(header_name.to_string().into(), header_value.to_string().into());
            } else {
                let mut response_headers = Headers::with_capacity(1);
                response_headers.insert(header_name.to_string().into(), header_value.to_string().into());
                self.response_headers = Some(response_headers);
            }
        }
        true
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
                    if let Some(download_progress) = self.download_progress {
                        download_progress.call(dlnow, dltotal);
                    }
                    if dlnow == dltotal {
                        self.progress_status = ProgressStatus::Completed;
                    } else {
                        self.progress_status = ProgressStatus::Downloading(dlnow);
                    }
                } else {
                    if let Some(upload_progress) = self.upload_progress {
                        upload_progress.call(ulnow, ultotal);
                    }
                    self.progress_status = ProgressStatus::Uploading(ulnow);
                }
            }
            ProgressStatus::Uploading(now) if now < ulnow => {
                if let Some(upload_progress) = self.upload_progress {
                    upload_progress.call(ulnow, ultotal);
                }
                if ulnow == ultotal {
                    self.progress_status = ProgressStatus::Downloading(dlnow);
                } else {
                    self.progress_status = ProgressStatus::Uploading(ulnow);
                }
            }
            ProgressStatus::Downloading(now) if now < dlnow => {
                if let Some(download_progress) = self.download_progress {
                    download_progress.call(dlnow, dltotal);
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

impl Default for CurlClient {
    fn default() -> Self {
        INITIALIZER.call_once(curl::init);
        CurlClientBuilder::default().build()
    }
}

impl CurlClientBuilder {
    fn build(self) -> CurlClient {
        self.inner_build().unwrap()
    }
}

impl<'r> Context<'r> {
    fn reset(&mut self) {
        self.request_body = None;
        self.response_body = None;
        self.response_headers = None;
        self.buffer_size = 1 << 22;
        self.temp_dir = &TEMP_DIR;
        self.progress_status = ProgressStatus::Initialized;
        self.upload_progress = None;
        self.download_progress = None;
    }
}

impl<'r> Default for Context<'r> {
    fn default() -> Self {
        Context {
            request_body: None,
            response_body: None,
            response_headers: None,
            buffer_size: 1 << 22,
            temp_dir: &TEMP_DIR,
            progress_status: ProgressStatus::Initialized,
            upload_progress: None,
            download_progress: None,
        }
    }
}

struct Easy2ContextRef([u8; size_of::<*mut Easy2<Context<'static>>>()]);

impl Default for Easy2ContextRef {
    fn default() -> Self {
        Box::new(Easy2::new(Context::default())).into()
    }
}

impl<'r> From<Easy2ContextRef> for Box<Easy2<Context<'r>>> {
    fn from(r: Easy2ContextRef) -> Self {
        unsafe {
            let ptr: *mut Easy2<Context<'r>> = transmute(r);
            Box::from_raw(ptr)
        }
    }
}

impl<'r> From<&'r Easy2ContextRef> for Box<Easy2<Context<'r>>> {
    fn from(r: &'r Easy2ContextRef) -> Self {
        unsafe {
            let ptr: *mut Easy2<Context<'r>> = transmute_copy(r);
            Box::from_raw(ptr)
        }
    }
}

impl<'r> From<&'r mut Easy2ContextRef> for Box<Easy2<Context<'r>>> {
    fn from(r: &'r mut Easy2ContextRef) -> Self {
        unsafe {
            let ptr: *mut Easy2<Context<'r>> = transmute_copy(r);
            Box::from_raw(ptr)
        }
    }
}

impl<'r> From<Box<Easy2<Context<'r>>>> for Easy2ContextRef {
    fn from(context: Box<Easy2<Context<'r>>>) -> Self {
        unsafe { transmute(Box::into_raw(context)) }
    }
}
