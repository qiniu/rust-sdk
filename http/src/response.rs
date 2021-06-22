use super::ResponseError;
use assert_impl::assert_impl;
use http::{
    header::{HeaderMap, HeaderName, HeaderValue},
    response::Response as HTTPResponse,
    status::StatusCode,
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

impl Body {
    pub fn fulfill(self) -> IOResult<CachedBody> {
        match self.0 {
            BodyInner::Reader(mut r) => {
                let mut bytes = Vec::new();
                r.read_to_end(&mut bytes)?;
                Ok(CachedBody(Cursor::new(bytes)))
            }
            BodyInner::File(mut r) => {
                let mut bytes = Vec::new();
                r.read_to_end(&mut bytes)?;
                Ok(CachedBody(Cursor::new(bytes)))
            }
            BodyInner::Bytes(b) => Ok(CachedBody(b)),
        }
    }
}

/// 经过缓存的 HTTP 响应体
#[derive(Debug)]
pub struct CachedBody(Cursor<Vec<u8>>);

impl Default for CachedBody {
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

impl Read for CachedBody {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
        self.0.read(buf)
    }
}

impl CachedBody {
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.get_ref()
    }

    #[inline]
    pub fn into_bytes(self) -> Vec<u8> {
        self.0.into_inner()
    }
}

#[cfg(feature = "async")]
mod async_body {
    use futures_lite::{
        io::{AsyncRead, AsyncReadExt, AsyncSeek, Cursor, Result as IOResult},
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

    impl AsyncBody {
        pub async fn fulfill(self) -> IOResult<AsyncCachedBody> {
            match self.0 {
                AsyncBodyInner::Reader(mut r) => {
                    let mut bytes = Vec::new();
                    r.read_to_end(&mut bytes).await?;
                    Ok(AsyncCachedBody(Cursor::new(bytes)))
                }
                AsyncBodyInner::SeekableReader(mut r) => {
                    let mut bytes = Vec::new();
                    r.read_to_end(&mut bytes).await?;
                    Ok(AsyncCachedBody(Cursor::new(bytes)))
                }
                AsyncBodyInner::Bytes(b) => Ok(AsyncCachedBody(b)),
            }
        }
    }

    /// 经过缓存的异步 HTTP 响应体
    #[derive(Debug)]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub struct AsyncCachedBody(Cursor<Vec<u8>>);

    impl Default for AsyncCachedBody {
        #[inline]
        fn default() -> Self {
            Self(Default::default())
        }
    }

    impl AsyncRead for AsyncCachedBody {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut [u8],
        ) -> Poll<IOResult<usize>> {
            let bytes = &mut self.as_mut().0;
            pin!(bytes);
            bytes.poll_read(cx, buf)
        }
    }

    impl AsyncCachedBody {
        #[inline]
        pub(super) fn new(bytes: Cursor<Vec<u8>>) -> Self {
            Self(bytes)
        }

        #[inline]
        pub fn as_bytes(&self) -> &[u8] {
            self.0.get_ref()
        }

        #[inline]
        pub fn into_bytes(self) -> Vec<u8> {
            self.0.into_inner()
        }
    }
}

#[cfg(feature = "async")]
pub use async_body::*;
#[cfg(feature = "async")]
use futures_lite::io::{AsyncSeekExt, Cursor as AsyncCursor};

/// HTTP 响应
///
/// 封装 HTTP 响应相关字段
#[derive(Debug)]
pub struct Response<B> {
    inner: HTTPResponse<B>,
    server_ip: Option<IpAddr>,
    server_port: u16,
    metrics: Metrics,
}

#[derive(Debug, Clone, Default)]
struct Metrics {
    total_duration: Option<Duration>,
    name_lookup_duration: Option<Duration>,
    connect_duration: Option<Duration>,
    secure_connect_duration: Option<Duration>,
    redirect_duration: Option<Duration>,
    transfer_duration: Option<Duration>,
}

impl<B: Default> Default for Response<B> {
    #[inline]
    fn default() -> Self {
        Self {
            inner: Default::default(),
            server_ip: None,
            server_port: 80,
            metrics: Default::default(),
        }
    }
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

    /// HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.server_ip
    }

    /// 修改 HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip_mut(&mut self) -> Option<&mut IpAddr> {
        self.server_ip.as_mut()
    }

    /// HTTP 服务器端口号
    #[inline]
    pub fn server_port(&self) -> u16 {
        self.server_port
    }

    /// 修改 HTTP 服务器端口号
    #[inline]
    pub fn server_port_mut(&mut self) -> &mut u16 {
        &mut self.server_port
    }

    /// 获取 HTTP 响应 Header
    #[inline]
    pub fn header(&self, header_name: HeaderName) -> Option<&HeaderValue> {
        self.headers().get(&header_name)
    }

    #[inline]
    pub fn total_duration(&self) -> Option<Duration> {
        self.metrics.total_duration
    }

    #[inline]
    pub fn name_lookup_duration(&self) -> Option<Duration> {
        self.metrics.name_lookup_duration
    }

    #[inline]
    pub fn connect_duration(&self) -> Option<Duration> {
        self.metrics.connect_duration
    }

    #[inline]
    pub fn secure_connect_duration(&self) -> Option<Duration> {
        self.metrics.secure_connect_duration
    }

    #[inline]
    pub fn redirect_duration(&self) -> Option<Duration> {
        self.metrics.redirect_duration
    }

    #[inline]
    pub fn transfer_duration(&self) -> Option<Duration> {
        self.metrics.transfer_duration
    }

    #[inline]
    pub fn total_duration_mut(&mut self) -> &mut Option<Duration> {
        &mut self.metrics.total_duration
    }

    #[inline]
    pub fn name_lookup_duration_mut(&mut self) -> &mut Option<Duration> {
        &mut self.metrics.name_lookup_duration
    }

    #[inline]
    pub fn connect_duration_mut(&mut self) -> &mut Option<Duration> {
        &mut self.metrics.connect_duration
    }

    #[inline]
    pub fn secure_connect_duration_mut(&mut self) -> &mut Option<Duration> {
        &mut self.metrics.secure_connect_duration
    }

    #[inline]
    pub fn redirect_duration_mut(&mut self) -> &mut Option<Duration> {
        &mut self.metrics.redirect_duration
    }

    #[inline]
    pub fn transfer_duration_mut(&mut self) -> &mut Option<Duration> {
        &mut self.metrics.transfer_duration
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

    pub fn map_body<B2>(self, f: impl FnOnce(B) -> B2) -> Response<B2> {
        let Response {
            inner,
            server_ip,
            server_port,
            metrics,
        } = self;
        let inner = inner.map(f);
        Response {
            inner,
            server_ip,
            server_port,
            metrics,
        }
    }

    pub fn try_map_body<B2, E>(
        self,
        f: impl FnOnce(B) -> result::Result<B2, E>,
    ) -> result::Result<Response<B2>, E> {
        let Response {
            inner,
            server_ip,
            server_port,
            metrics,
        } = self;
        let (parts, body) = inner.into_parts();
        let body = f(body)?;
        Ok(Response {
            inner: HTTPResponse::from_parts(parts, body),
            server_ip,
            server_port,
            metrics,
        })
    }
}

#[cfg(feature = "async")]
use std::{future::Future, pin::Pin};

#[cfg(feature = "async")]
type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

impl<B> Response<B> {
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub async fn async_map_body<B2>(self, f: impl FnOnce(B) -> BoxFuture<B2>) -> Response<B2> {
        let Response {
            inner,
            server_ip,
            server_port,
            metrics,
        } = self;
        let (parts, body) = inner.into_parts();
        let body = f(body).await;
        Response {
            inner: HTTPResponse::from_parts(parts, body),
            server_ip,
            server_port,
            metrics,
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub async fn async_try_map_body<B2, E>(
        self,
        f: impl FnOnce(B) -> BoxFuture<result::Result<B2, E>>,
    ) -> result::Result<Response<B2>, E> {
        let Response {
            inner,
            server_ip,
            server_port,
            metrics,
        } = self;
        let (parts, body) = inner.into_parts();
        let body = f(body).await?;
        Ok(Response {
            inner: HTTPResponse::from_parts(parts, body),
            server_ip,
            server_port,
            metrics,
        })
    }
}

impl<B: Send + Sync> Response<B> {
    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl Response<Body> {
    #[inline]
    pub fn fulfill(self) -> IOResult<Response<CachedBody>> {
        self.try_map_body(|b| b.fulfill())
    }
}

#[cfg(feature = "async")]
impl Response<AsyncBody> {
    pub async fn fulfill(self) -> IOResult<Response<AsyncCachedBody>> {
        self.async_try_map_body(|b| Box::pin(async { b.fulfill().await }))
            .await
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

    /// 设置 HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip(mut self, server_ip: Option<IpAddr>) -> Self {
        self.inner.server_ip = server_ip;
        self
    }

    /// 设置 HTTP 服务器端口号
    #[inline]
    pub fn server_port(mut self, server_port: u16) -> Self {
        self.inner.server_port = server_port;
        self
    }

    /// 添加 HTTP Header
    #[inline]
    pub fn header(mut self, header_name: HeaderName, header_value: HeaderValue) -> Self {
        self.inner.headers_mut().insert(header_name, header_value);
        self
    }

    #[inline]
    pub fn total_duration(mut self, duration: Duration) -> Self {
        self.inner.metrics.total_duration = Some(duration);
        self
    }

    #[inline]
    pub fn name_lookup_duration(mut self, duration: Duration) -> Self {
        self.inner.metrics.name_lookup_duration = Some(duration);
        self
    }

    #[inline]
    pub fn connect_duration(mut self, duration: Duration) -> Self {
        self.inner.metrics.connect_duration = Some(duration);
        self
    }

    #[inline]
    pub fn secure_connect_duration(mut self, duration: Duration) -> Self {
        self.inner.metrics.secure_connect_duration = Some(duration);
        self
    }

    #[inline]
    pub fn redirect_duration(mut self, duration: Duration) -> Self {
        self.inner.metrics.redirect_duration = Some(duration);
        self
    }

    #[inline]
    pub fn transfer_duration(mut self, duration: Duration) -> Self {
        self.inner.metrics.transfer_duration = Some(duration);
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

impl ResponseBuilder<CachedBody> {
    /// 设置二进制字节数组为 HTTP 响应体
    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        *self.inner.body_mut() = CachedBody(Cursor::new(body.into()));
        self
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

#[cfg(feature = "async")]
impl ResponseBuilder<AsyncCachedBody> {
    /// 设置二进制字节数组为 HTTP 响应体
    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        *self.inner.body_mut() = AsyncCachedBody::new(AsyncCursor::new(body.into()));
        self
    }
}

/// HTTP 响应结果
pub type Result<B> = result::Result<Response<B>, ResponseError>;
