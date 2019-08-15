pub mod error;
pub mod header;
pub mod method;
pub mod request;
pub mod response;
pub use error::{Error, ErrorKind, Result};
pub use header::{HeaderName, HeaderValue, Headers};
pub use method::Method;
pub use request::{Request, RequestBuilder};
pub use response::{Response, ResponseBuilder};

pub trait HTTPCaller {
    fn call(&self, request: &Request) -> Result<Response>;
}
