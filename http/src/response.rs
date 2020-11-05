use super::{HeaderNameOwned, HeaderValueOwned, HeadersOwned, ResponseError};
use assert_impl::assert_impl;
use std::{
    default::Default,
    fmt::Debug,
    fs::File,
    io::{Cursor, Read, Result as IOResult, Seek, SeekFrom},
    net::IpAddr,
    result,
    time::Duration,
};

/// HTTP 响应状态码
pub type StatusCode = u16;

trait ReadDebug: Read + Debug + Send + Sync {}
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
    use futures::{
        io::{AsyncRead, AsyncReadExt, AsyncSeek, Cursor, Result as IOResult},
        pin_mut,
    };
    use std::{
        fmt::Debug,
        pin::Pin,
        task::{Context, Poll},
    };

    pub(super) trait AsyncReadDebug: AsyncRead + Unpin + Debug + Send + Sync {}
    impl<T: AsyncRead + Unpin + Debug + Send + Sync> AsyncReadDebug for T {}

    pub(super) trait AsyncReadSeekDebug: AsyncSeek + AsyncReadDebug {}
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
                    pin_mut!(reader);
                    reader.poll_read(cx, buf)
                }
                AsyncBodyInner::SeekableReader(reader) => {
                    pin_mut!(reader);
                    reader.poll_read(cx, buf)
                }
                AsyncBodyInner::Bytes(bytes) => {
                    pin_mut!(bytes);
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
            pin_mut!(bytes);
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
use futures::io::{AsyncRead, AsyncSeek, AsyncSeekExt, Cursor as AsyncCursor};

/// HTTP 响应
///
/// 封装 HTTP 响应相关字段
#[derive(Debug)]
pub struct Response<B> {
    status_code: StatusCode,
    headers: HeadersOwned,
    body: B,
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
            status_code: 200,
            headers: Default::default(),
            body: Default::default(),
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
    /// HTTP 状态码
    #[inline]
    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    /// 修改 HTTP 状态码
    #[inline]
    pub fn status_code_mut(&mut self) -> &mut StatusCode {
        &mut self.status_code
    }

    /// HTTP Headers
    #[inline]
    pub fn headers(&self) -> &HeadersOwned {
        &self.headers
    }

    /// 修改 HTTP Headers
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeadersOwned {
        &mut self.headers
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
    pub fn header(&self, header_name: impl Into<HeaderNameOwned>) -> Option<&HeaderValueOwned> {
        self.headers.get(&header_name.into())
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
        &self.body
    }

    /// 直接获取 HTTP 响应体
    #[inline]
    pub fn into_body(self) -> B {
        self.body
    }

    /// 修改 HTTP 响应体
    #[inline]
    pub fn body_mut(&mut self) -> &mut B {
        &mut self.body
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
    pub fn fulfill(self) -> IOResult<Response<CachedBody>> {
        let Response {
            status_code,
            headers,
            body,
            server_ip,
            server_port,
            metrics,
        } = self;
        let body = body.fulfill()?;

        Ok(Response {
            status_code,
            headers,
            body,
            server_ip,
            server_port,
            metrics,
        })
    }
}

#[cfg(feature = "async")]
impl Response<AsyncBody> {
    pub async fn fulfill(self) -> IOResult<Response<AsyncCachedBody>> {
        let Response {
            status_code,
            headers,
            body,
            server_ip,
            server_port,
            metrics,
        } = self;
        let body = body.fulfill().await?;

        Ok(Response {
            status_code,
            headers,
            body,
            server_ip,
            server_port,
            metrics,
        })
    }
}

/// HTTP 响应体构建器
#[derive(Debug, Default)]
pub struct ResponseBuilder<B> {
    inner: Response<B>,
}

impl<B> ResponseBuilder<B> {
    /// 设置 HTTP 状态码
    #[inline]
    pub fn status_code(mut self, status_code: StatusCode) -> Self {
        self.inner.status_code = status_code;
        self
    }

    /// 设置 HTTP Headers
    #[inline]
    pub fn headers(mut self, headers: HeadersOwned) -> Self {
        self.inner.headers = headers;
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
    pub fn header(
        mut self,
        header_name: impl Into<HeaderNameOwned>,
        header_value: impl Into<HeaderValueOwned>,
    ) -> Self {
        self.inner
            .headers
            .insert(header_name.into(), header_value.into());
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
    pub fn stream_as_body(mut self, body: impl Read + Debug + Send + Sync + 'static) -> Self {
        self.inner.body = Body(BodyInner::Reader(Box::new(body)));
        self
    }

    /// 设置二进制字节数组为 HTTP 响应体
    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.inner.body = Body(BodyInner::Bytes(Cursor::new(body.into())));
        self
    }

    /// 设置文件为 HTTP 响应体
    #[inline]
    pub fn file_as_body(mut self, mut body: File) -> IOResult<Self> {
        body.seek(SeekFrom::Start(0))?;
        self.inner.body = Body(BodyInner::File(body));
        Ok(self)
    }
}

impl ResponseBuilder<CachedBody> {
    /// 设置二进制字节数组为 HTTP 响应体
    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.inner.body = CachedBody(Cursor::new(body.into()));
        self
    }
}

#[cfg(feature = "async")]
impl ResponseBuilder<AsyncBody> {
    /// 设置数据流为 HTTP 响应体
    #[inline]
    pub fn stream_as_body(
        mut self,
        body: impl AsyncRead + Unpin + Debug + Send + Sync + 'static,
    ) -> Self {
        self.inner.body = AsyncBody(AsyncBodyInner::Reader(Box::new(body)));
        self
    }

    /// 设置二进制字节数组为 HTTP 响应体
    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.inner.body = AsyncBody(AsyncBodyInner::Bytes(AsyncCursor::new(body.into())));
        self
    }

    /// 设置可定位的数据流（例如：文件）为 HTTP 响应体
    #[inline]
    pub async fn seekable_stream_as_body(
        mut self,
        mut body: impl AsyncRead + AsyncSeek + Unpin + Debug + Send + Sync + 'static,
    ) -> IOResult<Self> {
        body.seek(SeekFrom::Start(0)).await?;
        self.inner.body = AsyncBody(AsyncBodyInner::SeekableReader(Box::new(body)));
        Ok(self)
    }
}

#[cfg(feature = "async")]
impl ResponseBuilder<AsyncCachedBody> {
    /// 设置二进制字节数组为 HTTP 响应体
    #[inline]
    pub fn bytes_as_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.inner.body = AsyncCachedBody::new(AsyncCursor::new(body.into()));
        self
    }
}

/// HTTP 响应结果
pub type Result<B> = result::Result<Response<B>, ResponseError>;
