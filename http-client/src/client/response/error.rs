use super::{
    super::super::{EndpointParseError, RetriedStatsInfo},
    X_LOG_HEADER_NAME, X_REQ_ID_HEADER_NAME,
};
use assert_impl::assert_impl;
use qiniu_http::{
    HeaderValue, Metrics, ResponseError as HttpResponseError, ResponseErrorKind as HttpResponseErrorKind,
    ResponseParts as HttpResponseParts, StatusCode as HttpStatusCode,
};
use serde_json::Error as JsonError;
use std::{
    error, fmt,
    io::{Error as IoError, Read, Result as IOResult},
    mem::take,
    net::IpAddr,
    num::NonZeroU16,
    time::Duration,
};

#[cfg(feature = "async")]
use futures::{AsyncRead, AsyncReadExt};

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

    /// 解析响应体错误
    ParseResponseError,

    /// 响应体提前结束
    UnexpectedEof,

    /// 疑似响应被劫持
    MaliciousResponse,

    /// 系统调用失败
    SystemCallError,

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
    x_headers: XHeaders,
    response_body_sample: Vec<u8>,
    retried: Option<RetriedStatsInfo>,
}

const RESPONSE_BODY_SAMPLE_LEN_LIMIT: u64 = 1024;

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
            x_headers: Default::default(),
            response_body_sample: Default::default(),
            retried: Default::default(),
        }
    }

    /// 设置重试信息
    #[inline]
    #[must_use]
    pub fn retried(mut self, retried: &RetriedStatsInfo) -> Self {
        self.retried = Some(retried.to_owned());
        self
    }

    /// 设置 HTTP 响应信息
    #[inline]
    #[must_use]
    pub fn response_parts(mut self, response_parts: &HttpResponseParts) -> Self {
        self.server_ip = response_parts.server_ip();
        self.server_port = response_parts.server_port();
        self.metrics = extract_metrics_from_response_parts(response_parts);
        self.x_headers = response_parts.into();
        self
    }

    /// 直接设置响应体样本
    #[inline]
    pub fn set_response_body_sample(mut self, body: Vec<u8>) -> Self {
        self.response_body_sample = body;
        self
    }

    /// 设置响应体样本
    ///
    /// 该方法的异步版本为 [`Error::async_read_response_body_sample`]。
    #[inline]
    pub fn read_response_body_sample<R: Read>(mut self, body: R) -> IOResult<Self> {
        body.take(RESPONSE_BODY_SAMPLE_LEN_LIMIT)
            .read_to_end(&mut self.response_body_sample)?;
        Ok(self)
    }

    /// 异步设置响应体样本
    #[inline]
    #[cfg(feature = "async")]
    pub async fn async_read_response_body_sample<R: AsyncRead + Unpin>(mut self, body: R) -> IOResult<Self> {
        body.take(RESPONSE_BODY_SAMPLE_LEN_LIMIT)
            .read_to_end(&mut self.response_body_sample)
            .await?;
        Ok(self)
    }

    /// 获取 HTTP 响应错误类型
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// 获取响应体样本
    #[inline]
    pub fn response_body_sample(&self) -> &[u8] {
        &self.response_body_sample
    }

    /// 获取服务器 IP 地址
    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.server_ip
    }

    /// 获取服务器端口号
    #[inline]
    pub fn server_port(&self) -> Option<NonZeroU16> {
        self.server_port
    }

    /// 获取 HTTP 响应指标信息
    #[inline]
    pub fn metrics(&self) -> Option<&dyn Metrics> {
        self.metrics.as_deref()
    }

    /// 获取 HTTP 响应的 X-Log 信息
    #[inline]
    pub fn x_log(&self) -> Option<&HeaderValue> {
        self.x_headers.x_log.as_ref()
    }

    /// 获取 HTTP 响应的 X-ReqId 信息
    #[inline]
    pub fn x_reqid(&self) -> Option<&HeaderValue> {
        self.x_headers.x_reqid.as_ref()
    }

    pub(in super::super) fn from_http_response_error(
        mut err: HttpResponseError,
        x_headers: XHeaders,
        kind: Option<ErrorKind>,
    ) -> Self {
        Self {
            x_headers,
            server_ip: err.server_ip(),
            server_port: err.server_port(),
            metrics: take(err.metrics_mut()),
            kind: kind.unwrap_or_else(|| err.kind().into()),
            error: err.into_inner(),
            response_body_sample: Default::default(),
            retried: Default::default(),
        }
    }

    pub(crate) fn from_endpoint_parse_error(error: EndpointParseError, parts: &HttpResponseParts) -> Self {
        Self::new(ErrorKind::ParseResponseError, error).response_parts(parts)
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

#[derive(Debug, Default)]
pub(in super::super) struct XHeaders {
    x_log: Option<HeaderValue>,
    x_reqid: Option<HeaderValue>,
}

impl From<&HttpResponseParts> for XHeaders {
    #[inline]
    fn from(parts: &HttpResponseParts) -> Self {
        Self {
            x_log: extract_x_log_from_response_parts(parts),
            x_reqid: extract_x_reqid_from_response_parts(parts),
        }
    }
}

fn extract_x_log_from_response_parts(parts: &HttpResponseParts) -> Option<HeaderValue> {
    parts.header(X_LOG_HEADER_NAME).cloned()
}

fn extract_x_reqid_from_response_parts(parts: &HttpResponseParts) -> Option<HeaderValue> {
    parts.header(X_REQ_ID_HEADER_NAME).cloned()
}

fn extract_metrics_from_response_parts(parts: &HttpResponseParts) -> Option<Box<dyn Metrics + 'static>> {
    parts
        .metrics()
        .map(ClonedMetrics::new)
        .map(|metrics| Box::new(metrics) as Box<dyn Metrics + 'static>)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{:?}]", self.kind)?;
        if let Some(retried) = self.retried.as_ref() {
            write!(f, "[{}]", retried)?;
        }
        if let Some(x_reqid) = self.x_headers.x_reqid.as_ref() {
            write!(f, "[{:?}]", x_reqid)?;
        }
        if let Some(x_log) = self.x_headers.x_log.as_ref() {
            write!(f, "[{:?}]", x_log)?;
        }
        write!(f, " {}", self.error)?;
        if !self.response_body_sample.is_empty() {
            write!(f, " [{}]", String::from_utf8_lossy(&self.response_body_sample))?;
        }
        Ok(())
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
        Self::from_http_response_error(error, Default::default(), None)
    }
}

impl From<HttpResponseErrorKind> for ErrorKind {
    #[inline]
    fn from(kind: HttpResponseErrorKind) -> Self {
        ErrorKind::HttpError(kind)
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
    #[must_use]
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
        Self::new(ErrorKind::HttpError(HttpResponseErrorKind::LocalIoError), error)
    }
}
