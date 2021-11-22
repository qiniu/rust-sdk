use super::super::super::EndpointParseError;
use qiniu_http::{
    Metrics, ResponseError as HttpResponseError, ResponseErrorKind as HttpResponseErrorKind,
    ResponseParts as HttpResponseParts, StatusCode as HttpStatusCode,
};
use serde_json::Error as JsonError;
use std::{
    error, fmt, io::Error as IoError, mem::take, net::IpAddr, num::NonZeroU16, time::Duration,
};
use tap::Tap;

/// HTTP 响应错误类型
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// HTTP 客户端错误
    HttpError(HttpResponseErrorKind),

    /// 响应状态码错误
    StatusCodeError(HttpStatusCode),

    /// 未预期的状态码（例如 0 - 199 或 300 - 399，理论上应该由 HttpCaller 自动处理）
    UnexpectedStatusCode(HttpStatusCode),

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
    // TODO: 增加 x-log 作为可选错误信息
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
    pub(super) fn from_http_response_error(kind: ErrorKind, mut err: HttpResponseError) -> Self {
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

impl From<HttpResponseError> for Error {
    #[inline]
    fn from(error: HttpResponseError) -> Self {
        Self::from_http_response_error(error.kind().into(), error)
    }
}

impl From<HttpResponseErrorKind> for ErrorKind {
    #[inline]
    fn from(kind: HttpResponseErrorKind) -> Self {
        ErrorKind::HttpError(kind)
    }
}

impl Error {
    #[inline]
    pub(crate) fn from_endpoint_parse_error(
        error: EndpointParseError,
        parts: &HttpResponseParts,
    ) -> Self {
        Self::new(ErrorKind::ParseResponseError, error).tap_mut(|err| {
            err.server_ip = parts.server_ip();
            err.server_port = parts.server_port();
            err.metrics = parts
                .metrics()
                .map(ClonedMetrics::new)
                .map(|metrics| Box::new(metrics) as Box<dyn Metrics + 'static>);
        })
    }
}

#[derive(Clone, Debug)]
struct ClonedMetrics {
    total_duration: Option<Duration>,
    name_lookup_duration: Option<Duration>,
    connect_duration: Option<Duration>,
    secure_connect_duration: Option<Duration>,
    redirect_duration: Option<Duration>,
    transfer_duration: Option<Duration>,
}

impl ClonedMetrics {
    #[inline]
    fn new(metrics: &dyn Metrics) -> Self {
        Self {
            total_duration: metrics.total_duration(),
            name_lookup_duration: metrics.name_lookup_duration(),
            connect_duration: metrics.connect_duration(),
            secure_connect_duration: metrics.secure_connect_duration(),
            redirect_duration: metrics.redirect_duration(),
            transfer_duration: metrics.transfer_duration(),
        }
    }
}

impl Metrics for ClonedMetrics {
    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    #[inline]
    fn name_lookup_duration(&self) -> Option<Duration> {
        self.name_lookup_duration
    }

    #[inline]
    fn connect_duration(&self) -> Option<Duration> {
        self.connect_duration
    }

    #[inline]
    fn secure_connect_duration(&self) -> Option<Duration> {
        self.secure_connect_duration
    }

    #[inline]
    fn redirect_duration(&self) -> Option<Duration> {
        self.redirect_duration
    }

    #[inline]
    fn transfer_duration(&self) -> Option<Duration> {
        self.transfer_duration
    }
}

impl From<JsonError> for Error {
    #[inline]
    fn from(error: JsonError) -> Self {
        Self::new(ErrorKind::ParseResponseError, error)
    }
}

impl From<IoError> for Error {
    #[inline]
    fn from(error: IoError) -> Self {
        Self::new(
            ErrorKind::HttpError(HttpResponseErrorKind::LocalIoError),
            error,
        )
    }
}
