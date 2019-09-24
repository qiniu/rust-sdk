mod error;
mod header;
mod method;
mod request;
mod response;
pub use error::{Error, ErrorKind, Result, RetryKind};
pub use header::{HeaderName, HeaderValue, Headers};
pub use method::Method;
pub use request::{Body as RequestBody, Request, RequestBuilder, URL};
pub use response::{Body as ResponseBody, Response, ResponseBuilder, StatusCode};

pub trait HTTPCaller {
    fn call(&self, request: &Request) -> Result<Response>;
    fn on_retry_request(&self, _request: &Request, _error: &Error, _retried: usize, _retries: usize) {}
    fn on_host_failed(&self, _failed_host: &str, _error: &Error) {}
    fn on_request_built(&self, _request: &mut Request) {}
    fn on_response(&self, _request: &Request, _response: &Response) {}
    fn on_error(&self, _err: &Error) {}
}
