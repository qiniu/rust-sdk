mod error;
mod header;
mod method;
mod request;
mod response;
pub use error::{Error, ErrorKind, HTTPCallerError, HTTPCallerErrorKind, Result, RetryKind};
pub use header::{HeaderName, HeaderValue, Headers};
pub use method::Method;
pub use request::{Body as RequestBody, ProgressCallback, Request, RequestBuilder, URL};
pub use response::{Body as ResponseBody, Response, ResponseBuilder, StatusCode};

pub trait HTTPCaller: Send + Sync {
    fn call(&self, request: &Request) -> Result<Response>;
}
