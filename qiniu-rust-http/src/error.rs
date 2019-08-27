use super::{
    method::Method,
    request::{self, Request},
    response::Response,
};
use getset::Getters;
use std::{borrow::Borrow, boxed::Box, error, fmt, result};

pub type URL = request::URL;
pub type RequestID = String;
pub type Result<T> = result::Result<T, Error>;

#[derive(Getters)]
#[get = "pub"]
pub struct Error {
    kind: ErrorKind,
    is_retry_safe: bool,
    cause: Box<dyn error::Error>,
    method: Option<Method>,
    request_id: Option<RequestID>,
    url: Option<URL>,
}

impl Error {
    pub fn new<E: error::Error + 'static>(
        kind: ErrorKind,
        cause: E,
        is_retry_safe: bool,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Error {
            kind: kind,
            cause: Box::new(cause),
            is_retry_safe: is_retry_safe,
            method: Some(request.method().to_owned()),
            request_id: Self::extract_req_id_from_response(response),
            url: Some(request.url().to_owned()),
        }
    }

    pub fn new_retryable_error<E: error::Error + 'static>(
        cause: E,
        is_retry_safe: bool,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new(ErrorKind::RetryableError, cause, is_retry_safe, request, response)
    }

    pub fn new_zone_unretryable_error<E: error::Error + 'static>(
        cause: E,
        is_retry_safe: bool,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new(ErrorKind::ZoneUnretryableError, cause, is_retry_safe, request, response)
    }

    pub fn new_host_unretryable_error<E: error::Error + 'static>(
        cause: E,
        is_retry_safe: bool,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new(ErrorKind::HostUnretryableError, cause, is_retry_safe, request, response)
    }

    pub fn new_unretryable_error<E: error::Error + 'static>(
        cause: E,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new(ErrorKind::UnretryableError, cause, false, request, response)
    }

    pub fn new_from_parts<E: error::Error + 'static>(
        kind: ErrorKind,
        cause: E,
        is_retry_safe: bool,
        method: Option<Method>,
        url: Option<URL>,
    ) -> Error {
        Error {
            kind: kind,
            cause: Box::new(cause),
            is_retry_safe: is_retry_safe,
            method: method,
            request_id: None,
            url: url,
        }
    }

    pub fn new_retryable_error_from_parts<E: error::Error + 'static>(
        cause: E,
        is_retry_safe: bool,
        method: Option<Method>,
        url: Option<URL>,
    ) -> Error {
        Error {
            kind: ErrorKind::RetryableError,
            cause: Box::new(cause),
            is_retry_safe: is_retry_safe,
            method: method,
            request_id: None,
            url: url,
        }
    }

    pub fn new_zone_unretryable_error_from_parts<E: error::Error + 'static>(
        cause: E,
        is_retry_safe: bool,
        method: Option<Method>,
        url: Option<URL>,
    ) -> Error {
        Error {
            kind: ErrorKind::ZoneUnretryableError,
            cause: Box::new(cause),
            is_retry_safe: is_retry_safe,
            method: method,
            request_id: None,
            url: url,
        }
    }

    pub fn new_host_unretryable_error_from_parts<E: error::Error + 'static>(
        cause: E,
        is_retry_safe: bool,
        method: Option<Method>,
        url: Option<URL>,
    ) -> Error {
        Error {
            kind: ErrorKind::HostUnretryableError,
            cause: Box::new(cause),
            is_retry_safe: is_retry_safe,
            method: method,
            request_id: None,
            url: url,
        }
    }

    pub fn new_unretryable_error_from_parts<E: error::Error + 'static>(
        cause: E,
        method: Option<Method>,
        url: Option<URL>,
    ) -> Error {
        Error {
            kind: ErrorKind::UnretryableError,
            cause: Box::new(cause),
            is_retry_safe: false,
            method: method,
            request_id: None,
            url: url,
        }
    }

    fn extract_req_id_from_response(response: Option<&Response>) -> Option<RequestID> {
        response.and_then(|resp| resp.headers().get("X-Reqid").map(|v| v.to_owned()))
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Error")
            .field("kind", &self.kind)
            .field("cause", &self.cause)
            .field("method", &self.method)
            .field("url", &self.url)
            .field("is_retry_safe", &self.is_retry_safe)
            .finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}: {} {}: ",
            self.kind,
            self.method.as_ref().map(|m| m.as_str()).unwrap_or("None"),
            self.url.as_ref().map(|u| u.as_str()).unwrap_or("None"),
        )?;
        self.cause.fmt(f)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        self.cause.description()
    }

    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn error::Error> {
        Some(self.cause.borrow())
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(self.cause.borrow())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ErrorKind {
    RetryableError,
    ZoneUnretryableError,
    HostUnretryableError,
    UnretryableError,
}
