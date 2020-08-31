mod error;
mod request;
mod response;

pub use error::{Error as ResponseError, ErrorKind as ResponseErrorKind};
pub use qiniu_utils::http::{
    header::{HeaderName, HeaderNameOwned, HeaderValue, HeaderValueOwned, Headers, HeadersOwned},
    method::{InvalidMethod, Method},
};
pub use request::{Body as RequestBody, Request, RequestBuilder, URL};
pub use response::{
    Body as ResponseBody, Response, ResponseBuilder, Result as ResponseResult, StatusCode,
};

use std::any::Any;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// HTTP 请求处理函数
///
/// 实现该接口，即可处理所有七牛 SDK 发送的 HTTP 请求
pub trait HTTPCaller: Any + Send + Sync {
    /// 同步发送 HTTP 请求
    fn call(&self, request: &Request) -> ResponseResult;

    /// 异步发送 HTTP 请求
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_call<'a>(&'a self, request: &'a Request) -> BoxFuture<'a, ResponseResult> {
        Box::pin(async move { self.call(request) })
    }

    fn as_http_caller(&self) -> &dyn HTTPCaller;
    fn as_any(&self) -> &dyn Any;
}
