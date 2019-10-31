use curl::{
    easy::{Easy2, Handler, List, ReadError, SeekResult, WriteError},
    Version,
};
use derive_builder::Builder;
use lazy_static::lazy_static;
use qiniu_http::{
    Error, HTTPCaller, HTTPCallerError, HTTPCallerErrorKind, Headers, Method, Request, Response, ResponseBuilder,
    Result, StatusCode,
};
use std::{
    borrow::Cow,
    convert::TryInto,
    default::Default,
    env,
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
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
    static ref USER_AGENT: Box<str> = format!(
        "QiniuRust-libcurl/qiniu-{}/rust-{}/libcurl-{}",
        env!("CARGO_PKG_VERSION"),
        rustc_version_runtime::version(),
        Version::get().version(),
    )
    .into();
    static ref TEMP_DIR: PathBuf = env::temp_dir();
}

#[derive(Debug, Builder)]
#[builder(pattern = "owned", setter(into, strip_option), default)]
pub struct CurlClient {
    buffer_size: usize,

    temp_dir: Option<PathBuf>,

    #[builder(setter(skip))]
    user_agent: Option<String>,
}

impl Default for CurlClient {
    fn default() -> Self {
        INITIALIZER.call_once(curl::init);
        CurlClient {
            buffer_size: 1 << 22,
            temp_dir: None,
            user_agent: None,
        }
    }
}

impl HTTPCaller for CurlClient {
    fn call(&self, request: &Request) -> Result<Response> {
        let mut ctx = Context {
            request_body: None,
            response_body: None,
            response_headers: None,
            buffer_size: self.buffer_size,
            temp_dir: self
                .temp_dir
                .as_ref()
                .map(|dir| dir.as_path())
                .unwrap_or_else(|| &TEMP_DIR),
            progress_status: ProgressStatus::Initialized,
            upload_progress: request.on_uploading_progress(),
            download_progress: request.on_downloading_progress(),
        };
        self.set_context(&mut ctx, request);
        self.perform(ctx, request)
    }

    fn append_user_agent(&mut self, append_user_agent: &str) {
        if let Some(user_agent) = &mut self.user_agent {
            user_agent.push_str(append_user_agent);
        } else {
            let mut user_agent: String = USER_AGENT.to_string();
            user_agent.push_str(append_user_agent);
            self.user_agent = Some(user_agent);
        }
    }
}

impl CurlClient {
    fn perform(&self, context: Context, request: &Request) -> Result<Response> {
        let mut easy = Easy2::new(context);
        self.set_method(&mut easy, request)?;
        self.set_url(&mut easy, request)?;
        self.set_headers(&mut easy, request)?;
        self.set_body(&mut easy, request)?;
        self.set_options(&mut easy, request)?;
        Self::handle_if_err(easy.perform(), request)?;
        let status_code = Self::handle_if_err(easy.response_code(), request)? as StatusCode;
        let server_ip: Option<IpAddr> =
            Self::handle_if_err(easy.primary_ip().map(|s| s.and_then(|s| s.parse().ok())), request)?;
        let server_port = Self::handle_if_err(easy.primary_port(), request)?;
        self.build_response(easy.get_mut(), status_code, server_ip, server_port)
    }

    fn build_response(
        &self,
        context: &mut Context,
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
                ResponseBody::Memory(memory) => {
                    builder = builder.stream(Cursor::new(memory));
                }
                ResponseBody::File(file) => {
                    builder = builder.stream(file);
                }
            }
        }
        if let Some(server_ip) = server_ip {
            builder = builder.server_ip(server_ip);
        }
        builder = builder.server_port(server_port);
        Ok(builder.build().unwrap())
    }

    fn set_context<'r>(&self, mut context: &mut Context<'r>, request: &Request<'r>) {
        if let Some(request_body) = request.body() {
            if !request_body.is_empty() {
                context.request_body = Some(Cursor::new(request_body));
            }
        }

        match request.method() {
            Method::HEAD => (),
            _ => {
                context.response_body = Some(ResponseBody::Memory(Vec::with_capacity(context.buffer_size)));
            }
        }
    }

    fn set_method<T>(&self, easy: &mut Easy2<T>, request: &Request) -> Result<()> {
        let result = match request.method() {
            Method::GET => easy.get(true),
            Method::HEAD => easy.nobody(true),
            Method::POST => easy.post(true),
            Method::PUT => easy.upload(true),
            m => easy.custom_request(m.as_str()),
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
        if let Some(body) = request.body() {
            Self::handle_if_err(easy.post_field_size(body.len().try_into().unwrap()), request)
        } else {
            Ok(())
        }
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
        Self::handle_if_err(
            easy.useragent(
                self.user_agent
                    .as_ref()
                    .map(|ua| ua.as_str())
                    .unwrap_or_else(|| &USER_AGENT),
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
                        HTTPCallerError::new(HTTPCallerErrorKind::UnknownError, err),
                        false,
                        request,
                        None,
                    ))
                } else if err.is_recv_error() {
                    Err(Error::new_retryable_error(
                        HTTPCallerError::new(HTTPCallerErrorKind::ResponseError, err),
                        false,
                        request,
                        None,
                    ))
                } else if err.is_write_error() || err.is_again() || err.is_chunk_failed() {
                    Err(Error::new_retryable_error(
                        HTTPCallerError::new(HTTPCallerErrorKind::UnknownError, err),
                        true,
                        request,
                        None,
                    ))
                } else if err.is_operation_timedout() {
                    Err(Error::new_retryable_error(
                        HTTPCallerError::new(HTTPCallerErrorKind::TimeoutError, err),
                        true,
                        request,
                        None,
                    ))
                } else if err.is_send_error() {
                    Err(Error::new_retryable_error(
                        HTTPCallerError::new(HTTPCallerErrorKind::RequestError, err),
                        true,
                        request,
                        None,
                    ))
                } else if err.is_too_many_redirects() || err.is_got_nothing() {
                    Err(Error::new_host_unretryable_error(
                        HTTPCallerError::new(HTTPCallerErrorKind::UnknownError, err),
                        true,
                        request,
                        None,
                    ))
                } else if err.is_couldnt_resolve_proxy() {
                    Err(Error::new_host_unretryable_error(
                        HTTPCallerError::new(HTTPCallerErrorKind::ResolveError, err),
                        true,
                        request,
                        None,
                    ))
                } else if err.is_couldnt_connect() {
                    Err(Error::new_host_unretryable_error(
                        HTTPCallerError::new(HTTPCallerErrorKind::ConnectionError, err),
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
                        HTTPCallerError::new(HTTPCallerErrorKind::SSLError, err),
                        true,
                        request,
                        None,
                    ))
                } else {
                    Err(Error::new_unretryable_error(
                        HTTPCallerError::new(HTTPCallerErrorKind::UnknownError, err),
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
    Uploading(usize),
    Downloading(usize),
    Completed,
}

struct Context<'r> {
    request_body: Option<Cursor<&'r [u8]>>,
    response_body: Option<ResponseBody>,
    response_headers: Option<Headers<'static>>,
    buffer_size: usize,
    temp_dir: &'r Path,
    progress_status: ProgressStatus,
    upload_progress: Option<&'r dyn Fn(usize, usize)>,
    download_progress: Option<&'r dyn Fn(usize, usize)>,
}

enum ResponseBody {
    Memory(Vec<u8>),
    File(File),
}

impl<'r> Handler for Context<'r> {
    fn write(&mut self, data: &[u8]) -> result::Result<usize, WriteError> {
        match self.response_body {
            Some(ResponseBody::Memory(ref mut memory)) => {
                if memory.len() + data.len() > self.buffer_size {
                    let mut tmpfile = tempfile::tempfile_in(&self.temp_dir).map_err(|_| WriteError::Pause)?;
                    tmpfile.write_all(memory).map_err(|_| WriteError::Pause)?;
                    tmpfile.write_all(data).map_err(|_| WriteError::Pause)?;
                    self.response_body = Some(ResponseBody::File(tmpfile));
                } else {
                    memory.extend_from_slice(data);
                }
            }
            Some(ResponseBody::File(ref mut file)) => {
                file.write_all(data).map_err(|_| WriteError::Pause)?;
            }
            _ => {}
        }
        Ok(data.len())
    }

    fn read(&mut self, data: &mut [u8]) -> result::Result<usize, ReadError> {
        if let Some(request_body) = &mut self.request_body {
            match request_body.read(data) {
                Ok(have_read) => Ok(have_read),
                Err(_) => Err(ReadError::Abort),
            }
        } else {
            Ok(0)
        }
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
        let header = String::from_utf8_lossy(data).into_owned();
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
                response_headers.insert(
                    Cow::Owned(header_name.to_string()),
                    Cow::Owned(header_value.to_string()),
                );
            } else {
                let mut response_headers = Headers::with_capacity(1);
                response_headers.insert(
                    Cow::Owned(header_name.to_string()),
                    Cow::Owned(header_value.to_string()),
                );
                self.response_headers = Some(response_headers);
            }
        }
        true
    }

    fn progress(&mut self, dltotal: f64, dlnow: f64, ultotal: f64, ulnow: f64) -> bool {
        let dltotal = dltotal as usize;
        let dlnow = dlnow as usize;
        let ultotal = ultotal as usize;
        let ulnow = ulnow as usize;

        if dltotal == 0 && ultotal == 0 {
            return true;
        }
        match self.progress_status {
            ProgressStatus::Initialized => {
                if ultotal == 0 {
                    if let Some(download_progress) = self.download_progress.as_ref() {
                        (download_progress)(dlnow as usize, dltotal as usize);
                    }
                    if dlnow == dltotal {
                        self.progress_status = ProgressStatus::Completed;
                    } else {
                        self.progress_status = ProgressStatus::Downloading(dlnow);
                    }
                } else {
                    if let Some(upload_progress) = self.upload_progress.as_ref() {
                        (upload_progress)(ulnow as usize, ultotal as usize);
                    }
                    self.progress_status = ProgressStatus::Uploading(ulnow);
                }
            }
            ProgressStatus::Uploading(now) if now < ulnow => {
                if let Some(upload_progress) = self.upload_progress.as_ref() {
                    (upload_progress)(ulnow as usize, ultotal as usize);
                }
                if ulnow == ultotal {
                    self.progress_status = ProgressStatus::Downloading(dlnow);
                } else {
                    self.progress_status = ProgressStatus::Uploading(ulnow);
                }
            }
            ProgressStatus::Downloading(now) if now < dlnow => {
                if let Some(download_progress) = self.download_progress.as_ref() {
                    (download_progress)(dlnow as usize, dltotal as usize);
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
