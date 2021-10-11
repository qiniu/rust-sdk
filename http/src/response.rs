use super::{MapError, ResponseError};
use assert_impl::assert_impl;
use http::{
    header::{HeaderMap, HeaderName, HeaderValue},
    response::Response as HTTPResponse,
    status::StatusCode,
    Extensions, Version,
};
use std::{
    default::Default,
    fmt::Debug,
    io::{Cursor, Read, Result as IOResult},
    net::IpAddr,
    num::NonZeroU16,
    result,
    time::Duration,
};

#[cfg(feature = "async")]
use futures_lite::Future;

trait ReadDebug: Read + Debug + Send {}
impl<T: Read + Debug + Send> ReadDebug for T {}

/// HTTP 响应体
#[derive(Debug)]
pub struct Body(BodyInner);

#[derive(Debug)]
enum BodyInner {
    Reader(Box<dyn ReadDebug>),
    Bytes(Cursor<Vec<u8>>),
}

impl Default for Body {
    #[inline]
    fn default() -> Self {
        Self(BodyInner::Bytes(Default::default()))
    }
}

impl Read for Body {
    fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
        match &mut self.0 {
            BodyInner::Reader(reader) => reader.read(buf),
            BodyInner::Bytes(bytes) => bytes.read(buf),
        }
    }
}
#[cfg(feature = "async")]
mod async_body {
    use futures_lite::{
        io::{AsyncRead, Cursor, Result as IOResult},
        pin,
    };
    use std::{
        fmt::Debug,
        pin::Pin,
        task::{Context, Poll},
    };

    pub(super) trait AsyncReadDebug: AsyncRead + Unpin + Debug + Send + Sync {}
    impl<T: AsyncRead + Unpin + Debug + Send + Sync> AsyncReadDebug for T {}

    /// 异步 HTTP 响应体
    #[derive(Debug)]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub struct AsyncBody(pub(super) AsyncBodyInner);

    #[derive(Debug)]
    pub(super) enum AsyncBodyInner {
        Reader(Box<dyn AsyncReadDebug>),
        Bytes(Cursor<Vec<u8>>),
    }

    impl Default for AsyncBody {
        #[inline]
        fn default() -> Self {
            Self(AsyncBodyInner::Bytes(Default::default()))
        }
    }

    impl AsyncRead for AsyncBody {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut [u8],
        ) -> Poll<IOResult<usize>> {
            match &mut self.as_mut().0 {
                AsyncBodyInner::Reader(reader) => {
                    pin!(reader);
                    reader.poll_read(cx, buf)
                }
                AsyncBodyInner::Bytes(bytes) => {
                    pin!(bytes);
                    bytes.poll_read(cx, buf)
                }
            }
        }
    }
}

#[cfg(feature = "async")]
pub use async_body::*;
#[cfg(feature = "async")]
use futures_lite::io::Cursor as AsyncCursor;

pub trait Metrics: Debug + Send + Sync {
    fn total_duration(&self) -> Option<Duration>;
    fn name_lookup_duration(&self) -> Option<Duration>;
    fn connect_duration(&self) -> Option<Duration>;
    fn secure_connect_duration(&self) -> Option<Duration>;
    fn redirect_duration(&self) -> Option<Duration>;
    fn transfer_duration(&self) -> Option<Duration>;
}

#[derive(Debug, Default)]
pub(super) struct ResponseInfo {
    server_ip: Option<IpAddr>,
    server_port: Option<NonZeroU16>,
    metrics: Option<Box<dyn Metrics>>,
}

impl ResponseInfo {
    #[inline]
    pub(super) fn server_ip(&self) -> Option<IpAddr> {
        self.server_ip
    }

    #[inline]
    pub(super) fn server_port(&self) -> Option<NonZeroU16> {
        self.server_port
    }

    #[inline]
    pub(super) fn metrics(&self) -> Option<&dyn Metrics> {
        self.metrics.as_deref()
    }

    #[inline]
    pub(super) fn server_ip_mut(&mut self) -> &mut Option<IpAddr> {
        &mut self.server_ip
    }

    #[inline]
    pub(super) fn server_port_mut(&mut self) -> &mut Option<NonZeroU16> {
        &mut self.server_port
    }

    #[inline]
    pub(super) fn metrics_mut(&mut self) -> &mut Option<Box<dyn Metrics>> {
        &mut self.metrics
    }
}

/// HTTP 响应
///
/// 封装 HTTP 响应相关字段
#[derive(Debug, Default)]
pub struct Response<B> {
    inner: HTTPResponse<B>,
    info: ResponseInfo,
}

impl<B: Default> Response<B> {
    /// 返回 HTTP 响应构建器
    #[inline]
    pub fn builder() -> ResponseBuilder<B> {
        ResponseBuilder::<B>::default()
    }
}

impl<B> Response<B> {
    /// 获取 HTTP 请求
    #[inline]
    pub fn http(&self) -> &HTTPResponse<B> {
        &self.inner
    }

    /// 修改 HTTP 请求
    #[inline]
    pub fn http_mut(&mut self) -> &mut HTTPResponse<B> {
        &mut self.inner
    }

    /// HTTP 状态码
    #[inline]
    pub fn status_code(&self) -> StatusCode {
        self.inner.status()
    }

    /// 修改 HTTP 状态码
    #[inline]
    pub fn status_code_mut(&mut self) -> &mut StatusCode {
        self.inner.status_mut()
    }

    /// HTTP Headers
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    /// 修改 HTTP Headers
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.inner.headers_mut()
    }

    /// HTTP 版本
    #[inline]
    pub fn version(&self) -> Version {
        self.inner.version()
    }

    /// 修改 HTTP 版本
    #[inline]
    pub fn version_mut(&mut self) -> &mut Version {
        self.inner.version_mut()
    }

    /// HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.info.server_ip()
    }

    /// 修改 HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip_mut(&mut self) -> Option<&mut IpAddr> {
        self.info.server_ip.as_mut()
    }

    /// HTTP 服务器端口号
    #[inline]
    pub fn server_port(&self) -> Option<NonZeroU16> {
        self.info.server_port()
    }

    /// 修改 HTTP 服务器端口号
    #[inline]
    pub fn server_port_mut(&mut self) -> &mut Option<NonZeroU16> {
        self.info.server_port_mut()
    }

    /// 扩展字段
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        self.inner.extensions()
    }

    /// 修改扩展字段
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        self.inner.extensions_mut()
    }

    /// 获取 HTTP 响应 Header
    #[inline]
    pub fn header(&self, header_name: HeaderName) -> Option<&HeaderValue> {
        self.headers().get(&header_name)
    }

    #[inline]
    pub fn metrics(&self) -> Option<&dyn Metrics> {
        self.info.metrics()
    }

    #[inline]
    pub fn metrics_mut(&mut self) -> &mut Option<Box<dyn Metrics>> {
        self.info.metrics_mut()
    }

    /// HTTP 响应体
    #[inline]
    pub fn body(&self) -> &B {
        self.inner.body()
    }

    /// 直接获取 HTTP 响应体
    #[inline]
    pub fn into_body(self) -> B {
        self.inner.into_body()
    }

    /// 修改 HTTP 响应体
    #[inline]
    pub fn body_mut(&mut self) -> &mut B {
        self.inner.body_mut()
    }

    /// 对 HTTP 响应体进行映射
    #[inline]
    pub fn map_body<B2>(self, f: impl FnOnce(B) -> B2) -> Response<B2> {
        let Self { inner, info } = self;
        let (parts, body) = inner.into_parts();
        let body = f(body);
        let inner = HTTPResponse::from_parts(parts, body);
        Response { inner, info }
    }

    /// 尝试对 HTTP 响应体进行映射
    #[inline]
    pub fn try_map_body<B2, E>(
        self,
        f: impl FnOnce(B) -> result::Result<B2, E>,
    ) -> result::Result<Response<B2>, MapError<E>> {
        let Self { inner, info } = self;
        let (parts, body) = inner.into_parts();
        match f(body) {
            Ok(body) => {
                let inner = HTTPResponse::from_parts(parts, body);
                Ok(Response { inner, info })
            }
            Err(err) => Err(MapError::new(err, info)),
        }
    }

    /// 尝试对 HTTP 响应体进行异步映射
    #[inline]
    #[cfg(feature = "async")]
    pub async fn try_async_map_body<B2, E, F, Fut>(
        self,
        f: F,
    ) -> result::Result<Response<B2>, MapError<E>>
    where
        F: FnOnce(B) -> Fut,
        Fut: Future<Output = result::Result<B2, E>>,
    {
        let Self { inner, info } = self;
        let (parts, body) = inner.into_parts();
        match f(body).await {
            Ok(body) => {
                let inner = HTTPResponse::from_parts(parts, body);
                Ok(Response { inner, info })
            }
            Err(err) => Err(MapError::new(err, info)),
        }
    }
}

impl<B: Send + Sync> Response<B> {
    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// HTTP 响应体构建器
#[derive(Debug, Default)]
pub struct ResponseBuilder<B> {
    inner: Response<B>,
}

impl<B> ResponseBuilder<B> {
    /// 设置 HTTP 请求
    #[inline]
    pub fn http(&mut self, response: HTTPResponse<B>) -> &mut Self {
        self.inner.inner = response;
        self
    }

    /// 设置 HTTP 状态码
    #[inline]
    pub fn status_code(mut self, status_code: StatusCode) -> Self {
        *self.inner.status_code_mut() = status_code;
        self
    }

    /// 设置 HTTP Headers
    #[inline]
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        *self.inner.headers_mut() = headers;
        self
    }

    /// 设置 HTTP 版本
    #[inline]
    pub fn version(mut self, version: Version) -> Self {
        *self.inner.version_mut() = version;
        self
    }

    /// 设置 HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip(mut self, server_ip: IpAddr) -> Self {
        *self.inner.info.server_ip_mut() = Some(server_ip);
        self
    }

    /// 设置 HTTP 服务器端口号
    #[inline]
    pub fn server_port(mut self, server_port: NonZeroU16) -> Self {
        *self.inner.info.server_port_mut() = Some(server_port);
        self
    }

    /// 设置扩展字段
    #[inline]
    pub fn extensions(mut self, extensions: Extensions) -> Self {
        *self.inner.extensions_mut() = extensions;
        self
    }

    /// 添加 HTTP Header
    #[inline]
    pub fn header(mut self, header_name: HeaderName, header_value: HeaderValue) -> Self {
        self.inner.headers_mut().insert(header_name, header_value);
        self
    }

    #[inline]
    pub fn metrics(mut self, metrics: Box<dyn Metrics>) -> Self {
        *self.inner.info.metrics_mut() = Some(metrics);
        self
    }

    /// 构建 HTTP 请求
    #[inline]
    pub fn build(self) -> Response<B> {
        self.inner
    }
}

impl ResponseBuilder<Body> {
    /// 设置数据流为 HTTP 响应体
    #[inline]
    pub fn stream_as_body(mut self, body: impl Read + Debug + Send + 'static) -> Self {
        *self.inner.body_mut() = Body(BodyInner::Reader(Box::new(body)));
        self
    }

    /// 设置二进制字节数组为 HTTP 响应体
    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        *self.inner.body_mut() = Body(BodyInner::Bytes(Cursor::new(body.into())));
        self
    }
}

#[cfg(feature = "async")]
use futures_lite::io::AsyncRead;

#[cfg(feature = "async")]
impl ResponseBuilder<AsyncBody> {
    /// 设置数据流为 HTTP 响应体
    #[inline]
    pub fn stream_as_body(
        mut self,
        body: impl AsyncRead + Unpin + Debug + Send + Sync + 'static,
    ) -> Self {
        *self.inner.body_mut() = AsyncBody(AsyncBodyInner::Reader(Box::new(body)));
        self
    }

    /// 设置二进制字节数组为 HTTP 响应体
    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        *self.inner.body_mut() = AsyncBody(AsyncBodyInner::Bytes(AsyncCursor::new(body.into())));
        self
    }
}

/// HTTP 响应结果
pub type Result<B> = result::Result<Response<B>, ResponseError>;
