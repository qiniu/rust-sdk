use super::response::{Metrics, ResponseInfo, ResponseParts};
use anyhow::Error as AnyError;
use http::uri::{Scheme, Uri};
use std::{
    error::Error as StdError,
    fmt::{self, Debug, Display},
    net::IpAddr,
    num::NonZeroU16,
};

/// HTTP 响应错误类型
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// 协议错误，该协议不能支持
    ProtocolError,

    /// 非法的请求 / 响应错误
    InvalidRequestResponse,

    /// 非法的 URL
    InvalidUrl,

    /// 非法的 HTTP 头
    InvalidHeader,

    /// 网络连接失败
    ConnectError,

    /// 代理连接失败
    ProxyError,

    /// DNS 服务器连接失败
    DnsServerError,

    /// 域名解析失败
    UnknownHostError,

    /// 发送失败
    SendError,

    /// 接受失败
    ReceiveError,

    /// 本地 IO 失败
    LocalIoError,

    /// 超时失败
    TimeoutError,

    /// SSL 客户端证书错误
    ClientCertError,

    /// SSL 服务器端证书错误
    ServerCertError,

    /// SSL 错误
    SslError,

    /// 重定向次数过多
    TooManyRedirect,

    /// 未知错误
    UnknownError,

    /// 回调函数返回错误
    CallbackError,
}

/// HTTP 响应错误
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    error: AnyError,
    response_info: ResponseInfo,
}

impl Error {
    /// 创建 HTTP 响应错误构建器
    #[inline]
    pub fn builder(kind: ErrorKind, err: impl Into<AnyError>) -> ErrorBuilder {
        ErrorBuilder::new(kind, err)
    }

    /// 创建 HTTP 响应错误构建器
    #[inline]
    pub fn builder_with_msg(kind: ErrorKind, err: impl Display + Debug + Send + Sync + 'static) -> ErrorBuilder {
        ErrorBuilder::new_with_msg(kind, err)
    }

    /// 获取 HTTP 响应错误类型
    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// 转换为内部存储的实际响应错误
    #[inline]
    pub fn into_inner(self) -> AnyError {
        self.error
    }

    /// 获取服务器 IP 地址
    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.response_info.server_ip()
    }

    /// 获取服务器端口号
    #[inline]
    pub fn server_port(&self) -> Option<NonZeroU16> {
        self.response_info.server_port()
    }

    /// 获取响应指标信息
    #[inline]
    pub fn metrics(&self) -> Option<&Metrics> {
        self.response_info.metrics()
    }

    /// 获取服务器 IP 地址的可变引用
    #[inline]
    pub fn server_ip_mut(&mut self) -> &mut Option<IpAddr> {
        self.response_info.server_ip_mut()
    }

    /// 获取服务器端口号的可变引用
    #[inline]
    pub fn server_port_mut(&mut self) -> &mut Option<NonZeroU16> {
        self.response_info.server_port_mut()
    }

    /// 获取响应指标信息的可变引用
    #[inline]
    pub fn metrics_mut(&mut self) -> &mut Option<Metrics> {
        self.response_info.metrics_mut()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl StdError for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.error.as_ref())
    }
}

/// HTTP 响应错误构建器
#[derive(Debug)]
pub struct ErrorBuilder {
    inner: Error,
}

impl ErrorBuilder {
    /// 创建 HTTP 响应错误构建器
    #[inline]
    fn new(kind: ErrorKind, err: impl Into<AnyError>) -> Self {
        Self {
            inner: Error {
                kind,
                error: err.into(),
                response_info: Default::default(),
            },
        }
    }

    /// 创建 HTTP 响应错误构建器
    #[inline]
    fn new_with_msg(kind: ErrorKind, msg: impl Display + Debug + Send + Sync + 'static) -> Self {
        Self {
            inner: Error {
                kind,
                error: AnyError::msg(msg),
                response_info: Default::default(),
            },
        }
    }

    /// 构建 HTTP 响应错误
    #[inline]
    pub fn build(self) -> Error {
        self.inner
    }

    /// 设置 HTTP 响应的相关 URI 信息
    #[must_use]
    pub fn uri(mut self, uri: &Uri) -> Self {
        if let Some(host) = uri.host() {
            if let Ok(ip_addr) = host.parse::<IpAddr>() {
                *self.inner.server_ip_mut() = Some(ip_addr);
            }
        }
        if let Some(port) = uri.port_u16() {
            *self.inner.server_port_mut() = NonZeroU16::new(port);
        } else if let Some(scheme) = uri.scheme() {
            if scheme == &Scheme::HTTP {
                *self.inner.server_port_mut() = NonZeroU16::new(80);
            } else if scheme == &Scheme::HTTPS {
                *self.inner.server_port_mut() = NonZeroU16::new(443);
            }
        }
        self
    }

    /// 设置 HTTP 响应的服务器 IP 地址
    #[inline]
    #[must_use]
    pub fn server_ip(mut self, server_ip: IpAddr) -> Self {
        *self.inner.server_ip_mut() = Some(server_ip);
        self
    }

    /// 设置 HTTP 响应的服务器端口号
    #[inline]
    #[must_use]
    pub fn server_port(mut self, server_port: NonZeroU16) -> Self {
        *self.inner.server_port_mut() = Some(server_port);
        self
    }

    /// 设置 HTTP 响应指标信息
    #[inline]
    #[must_use]
    pub fn metrics(mut self, metrics: Metrics) -> Self {
        *self.inner.metrics_mut() = Some(metrics);
        self
    }
}

/// 响应映射错误
#[derive(Debug)]
pub struct MapError<E> {
    error: E,
    parts: ResponseParts,
}

impl<E> MapError<E> {
    pub(super) fn new(error: E, parts: ResponseParts) -> Self {
        Self { error, parts }
    }

    /// 转换为响应映射错误的原始错误
    #[inline]
    pub fn into_inner(self) -> E {
        self.error
    }
}

impl<E: Into<AnyError>> MapError<E> {
    /// 将响应映射错误转换为 HTTP 响应错误
    pub fn into_response_error(self, kind: ErrorKind) -> Error {
        Error {
            kind,
            error: self.error.into(),
            response_info: self.parts.into_response_info(),
        }
    }
}
