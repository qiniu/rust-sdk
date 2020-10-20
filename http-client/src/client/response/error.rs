use qiniu_http::{ResponseErrorKind as HTTPResponseErrorKind, StatusCode as HTTPStatusCode};
use std::{error, fmt};

/// HTTP 响应错误类型
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// HTTP 客户端错误
    HTTPError(HTTPResponseErrorKind),

    /// 响应状态码错误
    StatusCodeError(HTTPStatusCode),

    /// 解析响应体错误
    ParseResponseError,

    /// 疑似响应被劫持
    MaliciousResponse,
}

/// HTTP 响应错误
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    error: Box<dyn error::Error + Send + Sync>,
}

impl Error {
    /// 创建 HTTP 响应错误
    #[inline]
    pub fn new(kind: ErrorKind, err: impl Into<Box<dyn error::Error + Send + Sync>>) -> Self {
        Error {
            kind,
            error: err.into(),
        }
    }

    /// 获取 HTTP 响应错误类型
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.error.fmt(f)
    }
}

impl error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(self.error.as_ref())
    }
}
