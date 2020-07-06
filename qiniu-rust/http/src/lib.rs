mod error;
mod request;
mod response;

pub use error::{Error as ResponseError, ErrorType as ResponseErrorType, Result as ResponseResult};
pub use qiniu_utils::http::{
    header::{HeaderName, HeaderNameOwned, HeaderValue, HeaderValueOwned, Headers, HeadersOwned},
    method::{InvalidMethod, Method},
};
pub use request::{Body as RequestBody, Request, RequestBuilder, URL};
pub use response::{Body as ResponseBody, Response, ResponseBuilder, StatusCode};

use std::any::Any;

/// HTTP 请求处理函数
///
/// 实现该接口，即可处理所有七牛 SDK 发送的 HTTP 请求
pub trait HTTPCaller: Any + Send + Sync {
    fn call(&self, request: &Request) -> ResponseResult<Response>;
    fn as_any(&self) -> &dyn Any;
}
