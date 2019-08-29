pub use qiniu_http::{
    header::{HeaderName, HeaderValue, Headers},
    method::Method,
    request::{Request, RequestBuilder},
    response::{Response, ResponseBuilder},
};
pub(crate) mod client;
mod domains_manager;
pub use domains_manager::DomainsManager;
pub mod error;
mod http_caller;
pub(crate) mod request;
pub(crate) mod token;
pub use http_caller::PanickedHTTPCaller;
