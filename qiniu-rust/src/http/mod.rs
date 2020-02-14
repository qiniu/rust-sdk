//! HTTP 模块
//!
//! 负责对整个 SDK 的 HTTP 逻辑进行处理，包含 HTTP 请求的重试逻辑，HTTP 请求中间件和域名管理等。

pub use qiniu_http::{
    Error, ErrorKind, HTTPCaller, HTTPCallerErrorKind, HeaderName, HeaderValue, Headers, Method, Result, RetryKind,
    StatusCode,
};
mod client;
pub(crate) use client::Client;

pub mod domains_manager;
pub use domains_manager::{Choice, DomainsManager, DomainsManagerBuilder};

mod handler;
pub(crate) use handler::PanickedHTTPCaller;

mod middleware;
pub use middleware::{HTTPAfterAction, HTTPBeforeAction};

pub(crate) mod request;
mod response;
pub(crate) use response::Response;

mod token;
pub(crate) use token::Version as TokenVersion;
