use super::response::{Metrics, ResponseInfo};
use http::uri::{Scheme, Uri};
use std::{error, fmt, net::IpAddr, num::NonZeroU16};

/// HTTP 响应错误类型
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// 协议错误，该协议不能支持
    ProtocolError,

    /// 非法的请求 / 响应错误
    InvalidRequestResponse,

    /// 非法的 URL
    InvalidURL,

    /// 非法的 HTTP 头
    InvalidHeader,

    /// 网络连接失败
    ConnectError,

    /// 代理连接失败
    ProxyError,

    /// DNS 服务器连接失败
    DNSServerError,

    /// 域名解析失败
    UnknownHostError,

    /// 发送失败
    SendError,

    /// 接受失败
    ReceiveError,

    /// 本地 IO 失败
    LocalIOError,

    /// 超时失败
    TimeoutError,

    /// SSL 错误
    SSLError,

    /// 重定向次数过多
    TooManyRedirect,

    /// 未知错误
    UnknownError,

    /// 用户取消
    UserCanceled,
}

/// HTTP 响应错误
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    error: Box<dyn error::Error + Send + Sync>,
    response_info: ResponseInfo,
}

impl Error {
    /// 获取 HTTP 响应错误类型
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    #[inline]
    pub fn into_inner(self) -> Box<dyn error::Error + Send + Sync> {
        self.error
    }

    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.response_info.server_ip()
    }

    #[inline]
    pub fn server_port(&self) -> Option<NonZeroU16> {
        self.response_info.server_port()
    }

    #[inline]
    pub fn metrics(&self) -> Option<&dyn Metrics> {
        self.response_info.metrics()
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

#[derive(Debug)]
pub struct ErrorBuilder {
    inner: Error,
}

impl ErrorBuilder {
    /// 创建 HTTP 响应错误
    #[inline]
    pub fn new(kind: ErrorKind, err: impl Into<Box<dyn error::Error + Send + Sync>>) -> Self {
        Self {
            inner: Error {
                kind,
                error: err.into(),
                response_info: Default::default(),
            },
        }
    }

    #[inline]
    pub fn build(self) -> Error {
        self.inner
    }

    #[inline]
    pub fn uri(mut self, uri: &Uri) -> Self {
        if let Some(host) = uri.host() {
            if let Ok(ip_addr) = host.parse::<IpAddr>() {
                *self.inner.response_info.server_ip_mut() = Some(ip_addr);
            }
        }
        if let Some(port) = uri.port_u16() {
            *self.inner.response_info.server_port_mut() = NonZeroU16::new(port);
        } else if let Some(scheme) = uri.scheme() {
            if scheme == &Scheme::HTTP {
                *self.inner.response_info.server_port_mut() = NonZeroU16::new(80);
            } else if scheme == &Scheme::HTTPS {
                *self.inner.response_info.server_port_mut() = NonZeroU16::new(443);
            }
        }
        self
    }

    #[inline]
    pub fn server_ip(mut self, server_ip: IpAddr) -> Self {
        *self.inner.response_info.server_ip_mut() = Some(server_ip);
        self
    }

    #[inline]
    pub fn server_port(mut self, server_port: NonZeroU16) -> Self {
        *self.inner.response_info.server_port_mut() = Some(server_port);
        self
    }

    #[inline]
    pub fn metrics(mut self, metrics: Box<dyn Metrics>) -> Self {
        *self.inner.response_info.metrics_mut() = Some(metrics);
        self
    }
}
