use super::ResponseError;
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
    fs::File,
    io::{Cursor, Read, Result as IOResult, Seek, SeekFrom},
    net::IpAddr,
    result,
    time::Duration,
};

pub trait ReadDebug: Read + Debug + Send + Sync {}
impl<T: Read + Debug + Send + Sync> ReadDebug for T {}

/// HTTP 响应体
#[derive(Debug)]
pub struct Body(BodyInner);

#[derive(Debug)]
enum BodyInner {
    Reader(Box<dyn ReadDebug>),
    File(File),
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
            BodyInner::File(file) => file.read(buf),
            BodyInner::Bytes(bytes) => bytes.read(buf),
        }
    }
}
#[cfg(feature = "async")]
mod async_body {
    use futures_lite::{
        io::{AsyncRead, AsyncSeek, Cursor, Result as IOResult},
        pin,
    };
    use std::{
        fmt::Debug,
        pin::Pin,
        task::{Context, Poll},
    };

    pub trait AsyncReadDebug: AsyncRead + Unpin + Debug + Send + Sync {}
    impl<T: AsyncRead + Unpin + Debug + Send + Sync> AsyncReadDebug for T {}

    pub trait AsyncReadSeekDebug: AsyncSeek + AsyncReadDebug {}
    impl<T: AsyncSeek + AsyncReadDebug> AsyncReadSeekDebug for T {}

    /// 异步 HTTP 响应体
    #[derive(Debug)]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub struct AsyncBody(pub(super) AsyncBodyInner);

    #[derive(Debug)]
    pub(super) enum AsyncBodyInner {
        Reader(Box<dyn AsyncReadDebug>),
        SeekableReader(Box<dyn AsyncReadSeekDebug>),
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
                AsyncBodyInner::SeekableReader(reader) => {
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
use futures_lite::io::{AsyncSeekExt, Cursor as AsyncCursor};

pub trait Metrics: Debug + Send + Sync {
    fn total_duration(&self) -> Option<Duration>;
    fn name_lookup_duration(&self) -> Option<Duration>;
    fn connect_duration(&self) -> Option<Duration>;
    fn secure_connect_duration(&self) -> Option<Duration>;
    fn redirect_duration(&self) -> Option<Duration>;
    fn transfer_duration(&self) -> Option<Duration>;
}

#[derive(Debug)]
pub(super) struct ResponseInfo {
    server_ip: Option<IpAddr>,
    server_port: u16,
    metrics: Option<Box<dyn Metrics>>,
}

impl Default for ResponseInfo {
    #[inline]
    fn default() -> Self {
        Self {
            server_ip: Default::default(),
            server_port: 80,
            metrics: Default::default(),
        }
    }
}

impl ResponseInfo {
    #[inline]
    pub(super) fn server_ip(&self) -> Option<IpAddr> {
        self.server_ip
    }

    #[inline]
    pub(super) fn server_port(&self) -> u16 {
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
    pub(super) fn server_port_mut(&mut self) -> &mut u16 {
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
    pub fn server_port(&self) -> u16 {
        self.info.server_port()
    }

    /// 修改 HTTP 服务器端口号
    #[inline]
    pub fn server_port_mut(&mut self) -> &mut u16 {
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
    pub fn metrics(&mut self) -> Option<&dyn Metrics> {
        self.info.metrics()
    }

    #[inline]
    pub fn metrics_mut(&mut self) -> &mut Option<Box<dyn Metrics>> {
        self.info.metrics_mut()
    }

    #[inline]
    pub fn total_duration(&self) -> Option<Duration> {
        self.info
            .metrics
            .as_ref()
            .and_then(|metrics| metrics.total_duration())
    }

    #[inline]
    pub fn name_lookup_duration(&self) -> Option<Duration> {
        self.info
            .metrics
            .as_ref()
            .and_then(|metrics| metrics.name_lookup_duration())
    }

    #[inline]
    pub fn connect_duration(&self) -> Option<Duration> {
        self.info
            .metrics
            .as_ref()
            .and_then(|metrics| metrics.connect_duration())
    }

    #[inline]
    pub fn secure_connect_duration(&self) -> Option<Duration> {
        self.info
            .metrics
            .as_ref()
            .and_then(|metrics| metrics.secure_connect_duration())
    }

    #[inline]
    pub fn redirect_duration(&self) -> Option<Duration> {
        self.info
            .metrics
            .as_ref()
            .and_then(|metrics| metrics.redirect_duration())
    }

    #[inline]
    pub fn transfer_duration(&self) -> Option<Duration> {
        self.info
            .metrics
            .as_ref()
            .and_then(|metrics| metrics.transfer_duration())
    }

    /// HTTP 响应体
    #[inline]
    pub fn body(&self) -> &B {
        &self.inner.body()
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
    pub fn server_port(mut self, server_port: u16) -> Self {
        *self.inner.info.server_port_mut() = server_port;
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
    pub fn stream_as_body(mut self, body: Box<dyn ReadDebug>) -> Self {
        *self.inner.body_mut() = Body(BodyInner::Reader(body));
        self
    }

    /// 设置二进制字节数组为 HTTP 响应体
    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        *self.inner.body_mut() = Body(BodyInner::Bytes(Cursor::new(body.into())));
        self
    }

    /// 设置文件为 HTTP 响应体
    #[inline]
    pub fn file_as_body(mut self, mut body: File) -> IOResult<Self> {
        body.seek(SeekFrom::Start(0))?;
        *self.inner.body_mut() = Body(BodyInner::File(body));
        Ok(self)
    }
}

#[cfg(feature = "async")]
impl ResponseBuilder<AsyncBody> {
    /// 设置数据流为 HTTP 响应体
    #[inline]
    pub fn stream_as_body(mut self, body: Box<dyn AsyncReadDebug>) -> Self {
        *self.inner.body_mut() = AsyncBody(AsyncBodyInner::Reader(body));
        self
    }

    /// 设置二进制字节数组为 HTTP 响应体
    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        *self.inner.body_mut() = AsyncBody(AsyncBodyInner::Bytes(AsyncCursor::new(body.into())));
        self
    }

    /// 设置可定位的数据流（例如：文件）为 HTTP 响应体
    #[inline]
    pub async fn seekable_stream_as_body(
        mut self,
        mut body: Box<dyn AsyncReadSeekDebug>,
    ) -> IOResult<Self> {
        body.seek(SeekFrom::Start(0)).await?;
        *self.inner.body_mut() = AsyncBody(AsyncBodyInner::SeekableReader(body));
        Ok(self)
    }
}

/// HTTP 响应结果
pub type Result<B> = result::Result<Response<B>, ResponseError>;
