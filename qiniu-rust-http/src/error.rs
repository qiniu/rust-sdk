use super::{
    method::Method,
    request::{self, Request},
    response::Response,
};
use getset::Getters;
use std::{boxed::Box, error, fmt, result};

pub type URL = request::URL;
pub type RequestID = String;
pub type Result<T> = result::Result<T, Error>;

#[derive(Getters)]
#[get = "pub"]
pub struct Error {
    kind: ErrorKind,
    method: Method,
    request_id: Option<RequestID>,
    url: URL,
}

impl Error {
    fn new(kind: ErrorKind, request: &Request, response: Option<&Response>) -> Error {
        Error {
            kind: kind,
            method: request.method().to_owned(),
            request_id: Self::extract_req_id_from_response(response),
            url: request.url().to_owned(),
        }
    }

    pub fn new_retryable_error<E: error::Error + 'static>(
        cause: E,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new(
            ErrorKind::RetryableError(Box::new(cause)),
            request,
            response,
        )
    }

    pub fn new_host_unretryable_error<E: error::Error + 'static>(
        cause: E,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new(
            ErrorKind::HostUnretryableError(Box::new(cause)),
            request,
            response,
        )
    }

    pub fn new_unretryable_error<E: error::Error + 'static>(
        cause: E,
        request: &Request,
        response: Option<&Response>,
    ) -> Error {
        Self::new(
            ErrorKind::UnretryableError(Box::new(cause)),
            request,
            response,
        )
    }

    pub fn new_retryable_error_from_parts<E: error::Error + 'static>(
        cause: E,
        method: Method,
        url: URL,
    ) -> Error {
        Error {
            kind: ErrorKind::RetryableError(Box::new(cause)),
            method: method,
            request_id: None,
            url: url,
        }
    }

    pub fn new_host_unretryable_error_from_parts<E: error::Error + 'static>(
        cause: E,
        method: Method,
        url: URL,
    ) -> Error {
        Error {
            kind: ErrorKind::HostUnretryableError(Box::new(cause)),
            method: method,
            request_id: None,
            url: url,
        }
    }

    pub fn new_unretryable_error_from_parts<E: error::Error + 'static>(
        cause: E,
        method: Method,
        url: URL,
    ) -> Error {
        Error {
            kind: ErrorKind::UnretryableError(Box::new(cause)),
            method: method,
            request_id: None,
            url: url,
        }
    }

    fn extract_req_id_from_response(response: Option<&Response>) -> Option<RequestID> {
        response
            .map(|resp| resp.headers().get("X-Reqid").map(|v| v.to_owned()))
            .unwrap_or(None)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Error")
            .field("kind", &self.kind)
            .field("method", &self.method)
            .field("url", &self.url)
            .finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}: ", self.method, self.url)?;
        match self.kind {
            ErrorKind::RetryableError(ref e) => fmt::Display::fmt(e, f),
            ErrorKind::HostUnretryableError(ref e) => fmt::Display::fmt(e, f),
            ErrorKind::UnretryableError(ref e) => fmt::Display::fmt(e, f),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self.kind {
            ErrorKind::RetryableError(ref e) => e.description(),
            ErrorKind::HostUnretryableError(ref e) => e.description(),
            ErrorKind::UnretryableError(ref e) => e.description(),
        }
    }

    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn error::Error> {
        match self.kind {
            ErrorKind::RetryableError(ref e) => e.cause(),
            ErrorKind::HostUnretryableError(ref e) => e.cause(),
            ErrorKind::UnretryableError(ref e) => e.cause(),
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.kind {
            ErrorKind::RetryableError(ref e) => e.source(),
            ErrorKind::HostUnretryableError(ref e) => e.source(),
            ErrorKind::UnretryableError(ref e) => e.source(),
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    RetryableError(Box<error::Error>),
    HostUnretryableError(Box<error::Error>),
    UnretryableError(Box<error::Error>),
}
