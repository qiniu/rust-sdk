use qiniu_http::{
    Metrics, ResponseError as HTTPResponseError, ResponseErrorKind as HTTPResponseErrorKind,
    StatusCode as HTTPStatusCode,
};
use std::{error, fmt, mem::take, net::IpAddr, num::NonZeroU16};

/// HTTP 响应错误类型
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// HTTP 客户端错误
    HTTPError(HTTPResponseErrorKind),

    /// 响应状态码错误
    StatusCodeError(HTTPStatusCode),

    /// 未预期的状态码（例如 0 - 199 或 300 - 399，理论上应该由 HTTPCaller 自动处理）
    UnexpectedStatusCode(HTTPStatusCode),

    /// 读取响应体时遭遇未预期的 EOF
    UnexpectedEof,

    /// 解析响应体错误
    ParseResponseError,

    /// 疑似响应被劫持
    MaliciousResponse,

    /// 没有尝试
    NoTry,
}

/// HTTP 响应错误
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    error: Box<dyn error::Error + Send + Sync>,
    server_ip: Option<IpAddr>,
    server_port: Option<NonZeroU16>,
    metrics: Option<Box<dyn Metrics>>,
}

impl Error {
    /// 创建 HTTP 响应错误
    #[inline]
    pub fn new(kind: ErrorKind, err: impl Into<Box<dyn error::Error + Send + Sync>>) -> Self {
        Error {
            kind,
            error: err.into(),
            server_ip: Default::default(),
            server_port: Default::default(),
            metrics: Default::default(),
        }
    }

    /// 获取 HTTP 响应错误类型
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.server_ip
    }

    #[inline]
    pub fn server_port(&self) -> Option<NonZeroU16> {
        self.server_port
    }

    #[inline]
    pub fn metrics(&self) -> Option<&dyn Metrics> {
        self.metrics.as_deref()
    }

    #[inline]
    pub(super) fn from_http_response_error(kind: ErrorKind, mut err: HTTPResponseError) -> Self {
        Self {
            kind,
            server_ip: err.server_ip(),
            server_port: err.server_port(),
            metrics: take(err.metrics_mut()),
            error: err.into_inner(),
        }
    }
}

impl fmt::Display for Error {
    #[inline]
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

impl From<HTTPResponseError> for Error {
    #[inline]
    fn from(error: HTTPResponseError) -> Self {
        Self::from_http_response_error(error.kind().into(), error)
    }
}

impl From<HTTPResponseErrorKind> for ErrorKind {
    #[inline]
    fn from(kind: HTTPResponseErrorKind) -> Self {
        ErrorKind::HTTPError(kind)
    }
}
