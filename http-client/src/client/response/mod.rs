use qiniu_http::{Response as HTTPResponse, ResponseBody};
use std::{ops::Deref, result};

#[cfg(feature = "async")]
pub use qiniu_http::{AsyncCachedResponseBody, AsyncResponseBody};

mod error;
pub use error::{Error as ResponseError, ErrorKind as ResponseErrorKind};

pub type APIResult<T> = result::Result<T, ResponseError>;

#[derive(Default, Debug)]
pub struct Response<B> {
    inner: HTTPResponse<B>,
}

impl<B> Response<B> {
    #[inline]
    pub(super) fn new(inner: HTTPResponse<B>) -> Self {
        Self { inner }
    }
}

impl<B> Deref for Response<B> {
    type Target = HTTPResponse<B>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// 同步 HTTP 响应
pub type SyncResponse = Response<ResponseBody>;

/// 异步 HTTP 响应
#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(r#async)))]
pub type AsyncResponse = Response<AsyncResponseBody>;
