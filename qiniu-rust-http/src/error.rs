use super::{
    method::Method,
    request::Request,
    response::{Response, StatusCode},
};
use getset::{CopyGetters, Getters};
use std::{boxed::Box, error, fmt, io, marker::Send, ops::Deref, result};

pub type URL = Box<str>;
pub type RequestID = Box<str>;
pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Copy, Clone)]
pub enum RetryKind {
    RetryableError,
    ZoneUnretryableError,
    HostUnretryableError,
    UnretryableError,
}

#[derive(Debug)]
pub enum ErrorKind {
    HTTPCallerError(Box<dyn error::Error + Send>),
    JSONError(serde_json::Error),
    MaliciousResponse,
    IOError(io::Error),
    UnknownError(Box<dyn error::Error + Send>),
    ResponseStatusCodeError(StatusCode, Box<str>),
}

#[derive(Getters, CopyGetters)]
pub struct Error {
    #[get_copy = "pub"]
    retry_kind: RetryKind,

    #[get_copy = "pub"]
    is_retry_safe: bool,

    #[get = "pub"]
    error_kind: ErrorKind,

    #[get_copy = "pub"]
    method: Option<Method>,

    #[get = "pub"]
    request_id: Option<RequestID>,

    #[get = "pub"]
    url: Option<URL>,
}

impl Error {
    pub fn new(
        retry_kind: RetryKind,
        error_kind: ErrorKind,
        is_retry_safe: bool,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Error {
            retry_kind: retry_kind,
            error_kind: error_kind,
            is_retry_safe: is_retry_safe,
            method: Some(request.method()),
            request_id: Self::extract_req_id_from_response(response),
            url: Some(request.url().into()),
        }
    }

    pub fn new_retryable_error(
        error_kind: ErrorKind,
        is_retry_safe: bool,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new(RetryKind::RetryableError, error_kind, is_retry_safe, request, response)
    }

    pub fn new_zone_unretryable_error(
        error_kind: ErrorKind,
        is_retry_safe: bool,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new(
            RetryKind::ZoneUnretryableError,
            error_kind,
            is_retry_safe,
            request,
            response,
        )
    }

    pub fn new_host_unretryable_error(
        error_kind: ErrorKind,
        is_retry_safe: bool,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new(
            RetryKind::HostUnretryableError,
            error_kind,
            is_retry_safe,
            request,
            response,
        )
    }

    pub fn new_unretryable_error(error_kind: ErrorKind, request: &Request, response: Option<&Response>) -> Error {
        Self::new(RetryKind::UnretryableError, error_kind, false, request, response)
    }

    pub fn new_from_parts(
        retry_kind: RetryKind,
        error_kind: ErrorKind,
        is_retry_safe: bool,
        method: Option<Method>,
        url: Option<URL>,
    ) -> Error {
        Error {
            retry_kind: retry_kind,
            error_kind: error_kind,
            is_retry_safe: is_retry_safe,
            method: method,
            request_id: None,
            url: url,
        }
    }

    pub fn new_retryable_error_from_parts(
        error_kind: ErrorKind,
        is_retry_safe: bool,
        method: Option<Method>,
        url: Option<URL>,
    ) -> Error {
        Self::new_from_parts(RetryKind::RetryableError, error_kind, is_retry_safe, method, url)
    }

    pub fn new_zone_unretryable_error_from_parts(
        error_kind: ErrorKind,
        is_retry_safe: bool,
        method: Option<Method>,
        url: Option<URL>,
    ) -> Error {
        Self::new_from_parts(RetryKind::ZoneUnretryableError, error_kind, is_retry_safe, method, url)
    }

    pub fn new_host_unretryable_error_from_parts(
        error_kind: ErrorKind,
        is_retry_safe: bool,
        method: Option<Method>,
        url: Option<URL>,
    ) -> Error {
        Self::new_from_parts(RetryKind::HostUnretryableError, error_kind, is_retry_safe, method, url)
    }

    pub fn new_unretryable_error_from_parts(error_kind: ErrorKind, method: Option<Method>, url: Option<URL>) -> Error {
        Self::new_from_parts(RetryKind::UnretryableError, error_kind, false, method, url)
    }

    fn extract_req_id_from_response(response: Option<&Response>) -> Option<RequestID> {
        response.and_then(|resp| resp.headers().get("X-Reqid".into()).map(|v| v.as_ref().into()))
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Error")
            .field("retry_kind", &self.retry_kind)
            .field("error_kind", &self.error_kind)
            .field("method", &self.method)
            .field("url", &self.url)
            .field("request_id", &self.request_id)
            .field("is_retry_safe", &self.is_retry_safe)
            .finish()
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::HTTPCallerError(err) => write!(f, "HTTPCallerError({})", err),
            ErrorKind::JSONError(err) => write!(f, "JSONError({})", err),
            ErrorKind::MaliciousResponse => write!(f, "MaliciousResponse"),
            ErrorKind::IOError(err) => write!(f, "IOError({})", err),
            ErrorKind::UnknownError(err) => write!(f, "UnknownError({})", err),
            ErrorKind::ResponseStatusCodeError(status_code, error_message) => write!(
                f,
                "ResponseStatusCodeError(status_code = {}, error_message = {})",
                status_code, error_message
            ),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "retry_kind: {:?}, error_kind: {}, method: {}, url: {}, request_id: {}, is_retry_safe: {}",
            self.retry_kind,
            self.error_kind,
            self.method.as_ref().map(|m| m.as_str()).unwrap_or("None"),
            self.url.as_ref().map(|u| &u as &str).unwrap_or("None"),
            self.request_id.as_ref().map(|u| &u as &str).unwrap_or("None"),
            if self.is_retry_safe { "True" } else { "False" },
        )
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match &self.error_kind {
            ErrorKind::HTTPCallerError(err) => err.description(),
            ErrorKind::JSONError(err) => err.description(),
            ErrorKind::MaliciousResponse => "Malicious",
            ErrorKind::IOError(err) => err.description(),
            ErrorKind::UnknownError(err) => err.description(),
            ErrorKind::ResponseStatusCodeError(_, error_message) => &error_message,
        }
    }

    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn error::Error> {
        match &self.error_kind {
            ErrorKind::HTTPCallerError(err) => Some(err.deref()),
            ErrorKind::JSONError(err) => Some(err),
            ErrorKind::IOError(err) => Some(err),
            ErrorKind::UnknownError(err) => Some(err.deref()),
            ErrorKind::MaliciousResponse => None,
            ErrorKind::ResponseStatusCodeError(_, _) => None,
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.error_kind {
            ErrorKind::HTTPCallerError(err) => Some(err.deref()),
            ErrorKind::JSONError(err) => Some(err),
            ErrorKind::IOError(err) => Some(err),
            ErrorKind::UnknownError(err) => Some(err.deref()),
            ErrorKind::MaliciousResponse => None,
            ErrorKind::ResponseStatusCodeError(_, _) => None,
        }
    }
}
