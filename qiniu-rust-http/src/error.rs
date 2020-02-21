use super::{
    method::Method,
    request::Request,
    response::{Response, StatusCode},
};
use getset::{CopyGetters, Getters};
use std::{boxed::Box, error::Error as StdError, fmt, io, marker::Send, ops::Deref, result};

pub type URL = Box<str>;
pub type RequestID = Box<str>;
pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RetryKind {
    RetryableError,
    ZoneUnretryableError,
    HostUnretryableError,
    UnretryableError,
}

#[derive(Debug)]
pub enum JSONError {
    SerdeJSONError(serde_json::Error),
    Description(Box<str>),
}

#[derive(Debug)]
pub enum ErrorKind {
    HTTPCallerError(HTTPCallerError),
    JSONError(JSONError),
    MaliciousResponse,
    UnexpectedRedirect,
    IOError(io::Error),
    UnknownError(Box<dyn StdError + Send>),
    ResponseStatusCodeError(StatusCode, Box<str>),
    UserCanceled,
}

#[derive(Debug, Getters, CopyGetters)]
pub struct HTTPCallerError {
    #[get_copy = "pub"]
    kind: HTTPCallerErrorKind,
    #[get = "pub"]
    inner: Box<dyn StdError + Send>,
}

impl ErrorKind {
    pub fn new_http_caller_error_kind(kind: HTTPCallerErrorKind, error: impl StdError + Send + 'static) -> Self {
        ErrorKind::HTTPCallerError(HTTPCallerError {
            kind,
            inner: Box::new(error),
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum HTTPCallerErrorKind {
    ResolveError,
    ProxyError,
    SSLError,
    ConnectionError,
    RequestError,
    ResponseError,
    TimeoutError,
    UnknownError,
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
            retry_kind,
            error_kind,
            is_retry_safe,
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
            retry_kind,
            error_kind,
            is_retry_safe,
            method,
            url,
            request_id: None,
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
        response.and_then(|resp| resp.headers().get(&"X-Reqid".into()).map(|v| v.as_ref().into()))
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

impl fmt::Display for JSONError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::SerdeJSONError(err) => err.fmt(f),
            Self::Description(err) => err.fmt(f),
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::HTTPCallerError(err) => write!(f, "HTTPCallerError({})", err.inner),
            Self::JSONError(err) => write!(f, "JSONError({})", err),
            Self::MaliciousResponse => write!(f, "MaliciousResponse"),
            Self::UnexpectedRedirect => write!(f, "UnexpectedRedirect"),
            Self::UserCanceled => write!(f, "UserCanceled"),
            Self::IOError(err) => write!(f, "IOError({})", err),
            Self::UnknownError(err) => write!(f, "UnknownError({})", err),
            Self::ResponseStatusCodeError(status_code, error_message) => write!(
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

impl StdError for JSONError {
    fn description(&self) -> &str {
        match self {
            Self::SerdeJSONError(err) => err.description(),
            Self::Description(err) => err.as_ref(),
        }
    }
    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn StdError> {
        match self {
            Self::SerdeJSONError(err) => Some(err),
            Self::Description(_) => None,
        }
    }
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::SerdeJSONError(err) => Some(err),
            Self::Description(_) => None,
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match &self.error_kind {
            ErrorKind::HTTPCallerError(err) => err.description(),
            ErrorKind::JSONError(err) => err.description(),
            ErrorKind::MaliciousResponse => "Malicious response",
            ErrorKind::UnexpectedRedirect => "Unexpected redirect",
            ErrorKind::UserCanceled => "User canceled",
            ErrorKind::IOError(err) => err.description(),
            ErrorKind::UnknownError(err) => err.description(),
            ErrorKind::ResponseStatusCodeError(_, error_message) => &error_message,
        }
    }

    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn StdError> {
        match &self.error_kind {
            ErrorKind::HTTPCallerError(err) => err.cause(),
            ErrorKind::JSONError(err) => err.cause(),
            ErrorKind::IOError(err) => Some(err),
            ErrorKind::UnknownError(err) => Some(err.deref()),
            _ => None,
        }
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match &self.error_kind {
            ErrorKind::HTTPCallerError(err) => err.source(),
            ErrorKind::JSONError(err) => err.source(),
            ErrorKind::IOError(err) => Some(err),
            ErrorKind::UnknownError(err) => Some(err.deref()),
            _ => None,
        }
    }
}

impl fmt::Display for HTTPCallerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl StdError for HTTPCallerError {
    fn description(&self) -> &str {
        self.inner.description()
    }

    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn StdError> {
        Some(self.inner.deref())
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.inner.deref())
    }
}

impl From<serde_json::Error> for JSONError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerdeJSONError(err)
    }
}

impl From<Box<str>> for JSONError {
    fn from(err: Box<str>) -> Self {
        Self::Description(err)
    }
}

impl From<String> for JSONError {
    fn from(err: String) -> Self {
        Self::Description(err.into_boxed_str())
    }
}
