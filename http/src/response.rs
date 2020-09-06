use super::{HeaderNameOwned, HeaderValueOwned, HeadersOwned, ResponseError};
use assert_impl::assert_impl;
use std::{
    default::Default,
    fmt::Debug,
    fs::File,
    io::{Cursor, Read, Result as IOResult},
    net::IpAddr,
    result,
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

#[cfg(feature = "async")]
mod async_body {
    use async_fs::File;
    use futures::{
        io::{AsyncRead, Cursor, Result as IOResult},
        pin_mut,
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
        File(File),
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
                AsyncBodyInner::File(file) => {
                    pin_mut!(file);
                    file.poll_read(cx, buf)
                }
                AsyncBodyInner::Bytes(bytes) => {
                    pin_mut!(bytes);
                    bytes.poll_read(cx, buf)
                }
            }
        }
    }
}

#[cfg(feature = "async")]
pub use {
    async_body::*,
    async_fs::File as AsyncFile,
    futures::io::{AsyncRead, Cursor as AsyncCursor},
};

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
}

impl Response<Body> {
    /// HTTP 响应体
    #[inline]
    pub fn body(&self) -> &Body {
        &self.body
    }

    /// 修改 HTTP 响应体
    #[inline]
    pub fn body_mut(&mut self) -> &mut Body {
        &mut self.body
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

#[cfg(feature = "async")]
impl Response<AsyncBody> {
    /// HTTP 响应体
    #[inline]
    pub fn body(&self) -> &AsyncBody {
        &self.body
    }

    /// 修改 HTTP 响应体
    #[inline]
    pub fn body_mut(&mut self) -> &mut AsyncBody {
        &mut self.body
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
        assert_impl!(Unpin: Self);
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
        use std::io::{Seek, SeekFrom};

        body.seek(SeekFrom::Start(0))?;
        self.inner.body = Body(BodyInner::File(body));
        Ok(self)
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

    /// 设置文件为 HTTP 响应体
    #[inline]
    pub async fn file_as_body(mut self, body: File) -> IOResult<Self> {
        use futures::io::{AsyncSeekExt, SeekFrom};

        let mut body = AsyncFile::from(body);
        body.seek(SeekFrom::Start(0)).await?;
        self.inner.body = AsyncBody(AsyncBodyInner::File(body));
        Ok(self)
    }
}

/// HTTP 响应结果
pub type Result<B> = result::Result<Response<B>, ResponseError>;
