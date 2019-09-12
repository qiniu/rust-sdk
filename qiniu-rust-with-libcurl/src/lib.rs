use curl::{
    easy::{Easy2, Handler, List, ReadError, SeekResult, WriteError},
    Version,
};
use derive_builder::Builder;
use qiniu_http::{Error, HTTPCaller, Headers, Method, Request, Response, ResponseBuilder, Result, StatusCode};
use std::{
    borrow::Cow,
    convert::TryInto,
    default::Default,
    env,
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    result,
    sync::Once,
};

static INITIALIZER: Once = Once::new();

#[derive(Debug, Builder)]
#[builder(pattern = "owned", setter(into, strip_option), default)]
pub struct CurlClient {
    buffer_size: usize,
    temp_dir: PathBuf,
}

impl Default for CurlClient {
    fn default() -> Self {
        INITIALIZER.call_once(|| curl::init());
        CurlClient {
            buffer_size: 1 << 22,
            temp_dir: env::temp_dir(),
        }
    }
}

impl HTTPCaller for CurlClient {
    fn call(&self, request: &Request) -> Result<Response> {
        let mut ctx = Context {
            request_body: None,
            response_body: None,
            response_headers: Headers::new(),
            buffer_size: self.buffer_size,
            temp_dir: self.temp_dir.as_path(),
        };
        Self::set_context(&mut ctx, request);
        let response_code = Self::perform(&mut ctx, request)?;
        Self::build_response(ctx, response_code)
    }
}

impl CurlClient {
    fn perform(context: &mut Context, request: &Request) -> Result<StatusCode> {
        let mut easy = Easy2::new(context);
        Self::set_method(&mut easy, request)?;
        Self::set_url(&mut easy, request)?;
        Self::set_headers(&mut easy, request)?;
        Self::set_body(&mut easy, request)?;
        Self::set_options(&mut easy, request)?;
        Self::handle_if_err(easy.perform(), request)?;
        Ok(Self::handle_if_err(easy.response_code(), request)? as StatusCode)
    }

    fn build_response(context: Context, status_code: StatusCode) -> Result<Response> {
        let mut builder = ResponseBuilder::default()
            .status_code(status_code)
            .headers(context.response_headers);
        if let Some(response_body) = context.response_body {
            match response_body {
                ResponseBody::Memory(memory) => {
                    builder = builder.stream(Cursor::new(memory));
                }
                ResponseBody::File(file) => {
                    builder = builder.stream(file);
                }
            }
        }
        Ok(builder.build().unwrap())
    }

    fn set_context<'r>(mut context: &mut Context<'r>, request: &Request<'r>) {
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

    fn set_method<T>(easy: &mut Easy2<T>, request: &Request) -> Result<()> {
        let result = match request.method() {
            Method::GET => easy.get(true),
            Method::HEAD => easy.nobody(true),
            Method::POST => easy.post(true),
            Method::PUT => easy.upload(true),
            m => easy.custom_request(m.as_str()),
        };
        Self::handle_if_err(result, request)
    }

    fn set_url<T>(easy: &mut Easy2<T>, request: &Request) -> Result<()> {
        Self::handle_if_err(easy.url(request.url()), request)
    }

    fn set_headers<T>(easy: &mut Easy2<T>, request: &Request) -> Result<()> {
        let mut header_list = List::new();
        for (header_name, header_value) in request.headers().iter() {
            let h = header_name.as_ref().to_string() + ": " + header_value;
            Self::handle_if_err(header_list.append(&h), request)?;
        }
        Self::handle_if_err(easy.http_headers(header_list), request)
    }

    fn set_body<T>(easy: &mut Easy2<T>, request: &Request) -> Result<()> {
        if let Some(body) = request.body() {
            Self::handle_if_err(easy.post_field_size(body.len().try_into().unwrap()), request)
        } else {
            Ok(())
        }
    }

    fn set_options<T>(easy: &mut Easy2<T>, request: &Request) -> Result<()> {
        Self::handle_if_err(easy.accept_encoding(""), request)?;
        Self::handle_if_err(easy.transfer_encoding(true), request)?;
        Self::handle_if_err(easy.follow_location(true), request)?;
        Self::handle_if_err(easy.max_redirections(3), request)?;
        Self::handle_if_err(
            easy.useragent(&format!(
                "QiniuRust-libcurl/qiniu-{}/rust-{}/libcurl-{}",
                env!("CARGO_PKG_VERSION"),
                rustc_version_runtime::version(),
                Version::get().version(),
            )),
            request,
        )?;
        Self::handle_if_err(easy.show_header(false), request)?;
        Ok(())
    }

    fn handle_if_err<T>(result: result::Result<T, curl::Error>, request: &Request) -> Result<T> {
        match result {
            Ok(result) => Ok(result),
            Err(err) => {
                if err.is_unsupported_protocol() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_failed_init() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_url_malformed() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_couldnt_resolve_proxy() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_couldnt_resolve_host() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_couldnt_connect() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_partial_file() {
                    Err(Error::new_retryable_error(err, false, request, None))
                } else if err.is_read_error() {
                    Err(Error::new_retryable_error(err, false, request, None))
                } else if err.is_write_error() {
                    Err(Error::new_retryable_error(err, true, request, None))
                } else if err.is_out_of_memory() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_operation_timedout() {
                    Err(Error::new_host_unretryable_error(err, false, request, None))
                } else if err.is_range_error() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_http_post_error() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_ssl_connect_error() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_bad_download_resume() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_function_not_found() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_aborted_by_callback() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_bad_function_argument() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_interface_failed() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_too_many_redirects() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_unknown_option() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_peer_failed_verification() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_got_nothing() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_ssl_engine_notfound() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_ssl_engine_setfailed() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_send_error() {
                    Err(Error::new_retryable_error(err, true, request, None))
                } else if err.is_recv_error() {
                    Err(Error::new_retryable_error(err, false, request, None))
                } else if err.is_ssl_certproblem() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_ssl_cipher() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_ssl_cacert() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_bad_content_encoding() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_filesize_exceeded() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_use_ssl_failed() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_send_fail_rewind() {
                    Err(Error::new_unretryable_error(err, request, None))
                } else if err.is_ssl_engine_initfailed() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_ssl_cacert_badfile() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_ssl_crl_badfile() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_ssl_shutdown_failed() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_again() {
                    Err(Error::new_retryable_error(err, true, request, None))
                } else if err.is_ssl_issuer_error() {
                    Err(Error::new_host_unretryable_error(err, true, request, None))
                } else if err.is_chunk_failed() {
                    Err(Error::new_retryable_error(err, true, request, None))
                } else {
                    Err(Error::new_unretryable_error(err, request, None))
                }
            }
        }
    }
}

struct Context<'r> {
    request_body: Option<Cursor<&'r [u8]>>,
    response_body: Option<ResponseBody>,
    response_headers: Headers<'static>,
    buffer_size: usize,
    temp_dir: &'r Path,
}

enum ResponseBody {
    Memory(Vec<u8>),
    File(File),
}

impl<'r> Handler for &mut Context<'r> {
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
        match (header_name, header_value) {
            (Some(header_name), Some(header_value)) => {
                self.response_headers.insert(
                    Cow::Owned(header_name.to_string()),
                    Cow::Owned(header_value.to_string()),
                );
            }
            _ => {}
        }
        true
    }
}
