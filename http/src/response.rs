use super::{MapError, ResponseError};
use assert_impl::assert_impl;
use http::{
    header::{AsHeaderName, HeaderMap, HeaderValue, IntoHeaderName},
    response::{Parts as HttpResponseParts, Response as HttpResponse},
    status::StatusCode,
    Extensions, Version,
};
use std::{
    default::Default,
    fmt::Debug,
    mem::take,
    net::IpAddr,
    num::NonZeroU16,
    ops::{Deref, DerefMut},
    result,
    time::Duration,
};

#[cfg(feature = "async")]
use futures_lite::Future;

/// HTTP 响应的指标信息
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    total_duration: Option<Duration>,
    name_lookup_duration: Option<Duration>,
    connect_duration: Option<Duration>,
    secure_connect_duration: Option<Duration>,
    redirect_duration: Option<Duration>,
    transfer_duration: Option<Duration>,
}

impl Metrics {
    /// 创建 HTTP 响应的指标信息构建器
    #[inline]
    pub fn builder() -> MetricsBuilder {
        Default::default()
    }

    /// 获取总体请求耗时
    #[inline]
    pub fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    /// 获取总体请求耗时的可变引用
    #[inline]
    pub fn total_duration_mut(&mut self) -> &mut Option<Duration> {
        &mut self.total_duration
    }

    /// 获取域名查询的耗时
    #[inline]
    pub fn name_lookup_duration(&self) -> Option<Duration> {
        self.name_lookup_duration
    }

    /// 获取域名查询的耗时的可变引用
    #[inline]
    pub fn name_lookup_duration_mut(&mut self) -> &mut Option<Duration> {
        &mut self.name_lookup_duration
    }

    /// 获取建立连接的耗时
    #[inline]
    pub fn connect_duration(&self) -> Option<Duration> {
        self.connect_duration
    }

    /// 获取建立连接的耗时的可变引用
    #[inline]
    pub fn connect_duration_mut(&mut self) -> &mut Option<Duration> {
        &mut self.connect_duration
    }

    /// 获取建立安全连接的耗时
    #[inline]
    pub fn secure_connect_duration(&self) -> Option<Duration> {
        self.secure_connect_duration
    }

    /// 获取建立安全连接的耗时的可变引用
    #[inline]
    pub fn secure_connect_duration_mut(&mut self) -> &mut Option<Duration> {
        &mut self.secure_connect_duration
    }

    /// 获取重定向的耗时
    #[inline]
    pub fn redirect_duration(&self) -> Option<Duration> {
        self.redirect_duration
    }

    /// 获取重定向的耗时的可变引用
    #[inline]
    pub fn redirect_duration_mut(&mut self) -> &mut Option<Duration> {
        &mut self.redirect_duration
    }

    /// 获取请求和响应数据传输的耗时
    #[inline]
    pub fn transfer_duration(&self) -> Option<Duration> {
        self.transfer_duration
    }

    /// 获取请求和响应数据传输的耗时的可变引用
    #[inline]
    pub fn transfer_duration_mut(&mut self) -> &mut Option<Duration> {
        &mut self.transfer_duration
    }
}

/// HTTP 响应的指标信息构建器
#[derive(Debug, Clone, Default)]
pub struct MetricsBuilder(Metrics);

impl MetricsBuilder {
    /// 设置总体请求耗时
    #[inline]
    pub fn total_duration(&mut self, duration: Duration) -> &mut Self {
        self.0.total_duration = Some(duration);
        self
    }

    /// 设置域名查询的耗时
    #[inline]
    pub fn name_lookup_duration(&mut self, duration: Duration) -> &mut Self {
        self.0.name_lookup_duration = Some(duration);
        self
    }

    /// 设置建立连接的耗时
    #[inline]
    pub fn connect_duration(&mut self, duration: Duration) -> &mut Self {
        self.0.connect_duration = Some(duration);
        self
    }

    /// 设置建立安全连接的耗时
    #[inline]
    pub fn secure_connect_duration(&mut self, duration: Duration) -> &mut Self {
        self.0.secure_connect_duration = Some(duration);
        self
    }

    /// 设置重定向的耗时
    #[inline]
    pub fn redirect_duration(&mut self, duration: Duration) -> &mut Self {
        self.0.redirect_duration = Some(duration);
        self
    }

    /// 设置请求和响应数据传输的耗时
    #[inline]
    pub fn transfer_duration(&mut self, duration: Duration) -> &mut Self {
        self.0.transfer_duration = Some(duration);
        self
    }

    /// 构建 HTTP 响应的指标信息
    #[inline]
    pub fn build(&mut self) -> Metrics {
        take(&mut self.0)
    }
}

#[derive(Debug, Default, Clone)]
pub(super) struct ResponseInfo {
    server_ip: Option<IpAddr>,
    server_port: Option<NonZeroU16>,
    metrics: Option<Metrics>,
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
    pub(super) fn metrics(&self) -> Option<&Metrics> {
        self.metrics.as_ref()
    }

    pub(super) fn server_ip_mut(&mut self) -> &mut Option<IpAddr> {
        &mut self.server_ip
    }

    pub(super) fn server_port_mut(&mut self) -> &mut Option<NonZeroU16> {
        &mut self.server_port
    }

    pub(super) fn metrics_mut(&mut self) -> &mut Option<Metrics> {
        &mut self.metrics
    }
}

/// HTTP 响应信息
///
/// 不包含响应体信息
#[derive(Debug)]
pub struct ResponseParts {
    inner: HttpResponseParts,
    info: ResponseInfo,
}

impl ResponseParts {
    /// 获取 HTTP 状态码
    #[inline]
    pub fn status_code(&self) -> StatusCode {
        self.inner.status
    }

    /// 获取 HTTP 状态码 的可变引用
    #[inline]
    pub fn status_code_mut(&mut self) -> &mut StatusCode {
        &mut self.inner.status
    }

    /// 获取 HTTP Headers
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.inner.headers
    }

    /// 获取 HTTP Headers 的可变引用
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.inner.headers
    }

    /// 获取 HTTP 版本
    #[inline]
    pub fn version(&self) -> Version {
        self.inner.version
    }

    /// 获取 HTTP 版本的可变引用
    #[inline]
    pub fn version_mut(&mut self) -> &mut Version {
        &mut self.inner.version
    }

    /// 获取 扩展信息
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.inner.extensions
    }

    /// 获取扩展信息的可变引用
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.inner.extensions
    }

    /// 获取 HTTP 响应 Header
    #[inline]
    pub fn header(&self, header_name: impl AsHeaderName) -> Option<&HeaderValue> {
        self.headers().get(header_name)
    }

    pub(super) fn into_response_info(self) -> ResponseInfo {
        self.info
    }

    /// 获取 HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.info.server_ip()
    }

    /// 获取 HTTP 服务器 IP 地址的可变引用
    #[inline]
    pub fn server_ip_mut(&mut self) -> &mut Option<IpAddr> {
        self.info.server_ip_mut()
    }

    /// 获取 HTTP 服务器端口号
    #[inline]
    pub fn server_port(&self) -> Option<NonZeroU16> {
        self.info.server_port()
    }

    /// 获取 HTTP 服务器端口号的可变引用
    #[inline]
    pub fn server_port_mut(&mut self) -> &mut Option<NonZeroU16> {
        self.info.server_port_mut()
    }

    /// 获取 HTTP 响应的指标信息
    #[inline]
    pub fn metrics(&self) -> Option<&Metrics> {
        self.info.metrics()
    }

    /// 获取 HTTP 响应的指标信息的可变引用
    #[inline]
    pub fn metrics_mut(&mut self) -> &mut Option<Metrics> {
        self.info.metrics_mut()
    }
}

impl Default for ResponseParts {
    #[inline]
    fn default() -> Self {
        let (parts, _) = HttpResponse::new(()).into_parts();
        Self {
            inner: parts,
            info: Default::default(),
        }
    }
}

/// HTTP 响应
///
/// 封装 HTTP 响应相关字段
#[derive(Debug, Default)]
pub struct Response<B> {
    parts: ResponseParts,
    body: B,
}

impl<B: Default> Response<B> {
    /// 返回 HTTP 响应构建器
    #[inline]
    pub fn builder() -> ResponseBuilder<B> {
        ResponseBuilder::<B>::default()
    }
}

impl<B> Response<B> {
    /// 获取 HTTP 响应体
    #[inline]
    pub fn body(&self) -> &B {
        &self.body
    }

    /// 获取 HTTP 响应体的可变引用
    #[inline]
    pub fn body_mut(&mut self) -> &mut B {
        &mut self.body
    }

    /// HTTP 响应信息
    #[inline]
    pub fn parts(&self) -> &ResponseParts {
        &self.parts
    }

    /// 获取 HTTP 响应信息的可变引用
    #[inline]
    pub fn parts_mut(&mut self) -> &mut ResponseParts {
        &mut self.parts
    }

    /// 转换为 HTTP 响应体
    #[inline]
    pub fn into_body(self) -> B {
        self.body
    }

    /// 转换为响应信息和响应体
    #[inline]
    pub fn into_parts_and_body(self) -> (ResponseParts, B) {
        let Self { parts, body } = self;
        (parts, body)
    }

    /// 通过响应信息和响应体创建 HTTP 响应
    #[inline]
    pub fn from_parts_and_body(parts: ResponseParts, body: B) -> Self {
        Response { parts, body }
    }

    /// 对 HTTP 响应体进行映射
    #[inline]
    pub fn map_body<B2>(self, f: impl FnOnce(B) -> B2) -> Response<B2> {
        let (parts, body) = self.into_parts_and_body();
        let body = f(body);
        Response::from_parts_and_body(parts, body)
    }

    /// 尝试对 HTTP 响应体进行映射
    #[inline]
    pub fn try_map_body<B2, E>(
        self,
        f: impl FnOnce(B) -> result::Result<B2, E>,
    ) -> result::Result<Response<B2>, MapError<E>> {
        let (parts, body) = self.into_parts_and_body();
        match f(body) {
            Ok(body) => Ok(Response::from_parts_and_body(parts, body)),
            Err(err) => Err(MapError::new(err, parts)),
        }
    }

    /// 尝试对 HTTP 响应体进行异步映射
    #[inline]
    #[cfg(feature = "async")]
    pub async fn try_async_map_body<B2, E, F, Fut>(self, f: F) -> result::Result<Response<B2>, MapError<E>>
    where
        F: FnOnce(B) -> Fut,
        Fut: Future<Output = result::Result<B2, E>>,
    {
        let (parts, body) = self.into_parts_and_body();
        match f(body).await {
            Ok(body) => Ok(Response::from_parts_and_body(parts, body)),
            Err(err) => Err(MapError::new(err, parts)),
        }
    }
}

impl<B> Deref for Response<B> {
    type Target = ResponseParts;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.parts
    }
}

impl<B> DerefMut for Response<B> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.parts
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
    /// 设置 HTTP 状态码
    #[inline]
    pub fn status_code(&mut self, status_code: StatusCode) -> &mut Self {
        *self.inner.status_code_mut() = status_code;
        self
    }

    /// 设置 HTTP Headers
    #[inline]
    pub fn headers(&mut self, headers: HeaderMap) -> &mut Self {
        *self.inner.headers_mut() = headers;
        self
    }

    /// 设置 HTTP 版本
    #[inline]
    pub fn version(&mut self, version: Version) -> &mut Self {
        *self.inner.version_mut() = version;
        self
    }

    /// 设置 HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip(&mut self, server_ip: IpAddr) -> &mut Self {
        *self.inner.info.server_ip_mut() = Some(server_ip);
        self
    }

    /// 设置 HTTP 服务器端口号
    #[inline]
    pub fn server_port(&mut self, server_port: NonZeroU16) -> &mut Self {
        *self.inner.info.server_port_mut() = Some(server_port);
        self
    }

    /// 设置扩展信息
    #[inline]
    pub fn extensions(&mut self, extensions: Extensions) -> &mut Self {
        *self.inner.extensions_mut() = extensions;
        self
    }

    /// 添加 HTTP Header
    #[inline]
    pub fn header(&mut self, header_name: impl IntoHeaderName, header_value: impl Into<HeaderValue>) -> &mut Self {
        self.inner.headers_mut().insert(header_name, header_value.into());
        self
    }

    /// 添加 HTTP 响应体
    #[inline]
    pub fn body(&mut self, body: B) -> &mut Self {
        *self.inner.body_mut() = body;
        self
    }

    /// 设置 HTTP 响应的指标信息
    #[inline]
    pub fn metrics(&mut self, metrics: Metrics) -> &mut Self {
        *self.inner.metrics_mut() = Some(metrics);
        self
    }
}

impl<B: Default> ResponseBuilder<B> {
    /// 构建 HTTP 请求
    #[inline]
    pub fn build(&mut self) -> Response<B> {
        take(&mut self.inner)
    }
}

mod body {
    use assert_impl::assert_impl;
    use std::{
        default::Default,
        fmt::Debug,
        io::{Cursor, Read, Result as IoResult},
    };

    trait ReadDebug: Read + Debug + Send {}
    impl<T: Read + Debug + Send> ReadDebug for T {}

    /// HTTP 响应体
    #[derive(Debug)]
    pub struct ResponseBody(ResponseBodyInner);

    #[derive(Debug)]
    enum ResponseBodyInner {
        Reader(Box<dyn ReadDebug>),
        Bytes(Cursor<Vec<u8>>),
    }

    impl ResponseBody {
        /// 通过输入流创建 HTTP 响应体
        #[inline]
        pub fn from_reader(reader: impl Read + Debug + Send + 'static) -> Self {
            Self(ResponseBodyInner::Reader(Box::new(reader)))
        }

        /// 通过二进制数据创建 HTTP 响应体
        #[inline]
        pub fn from_bytes(bytes: Vec<u8>) -> Self {
            Self(ResponseBodyInner::Bytes(Cursor::new(bytes)))
        }

        #[allow(dead_code)]
        fn ignore() {
            assert_impl!(Send: Self);
            // assert_impl!(Sync: Self);
        }
    }

    impl Default for ResponseBody {
        #[inline]
        fn default() -> Self {
            Self::from_bytes(Default::default())
        }
    }

    impl Read for ResponseBody {
        fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
            match &mut self.0 {
                ResponseBodyInner::Reader(reader) => reader.read(buf),
                ResponseBodyInner::Bytes(bytes) => bytes.read(buf),
            }
        }
    }

    #[cfg(feature = "async")]
    mod async_body {
        use assert_impl::assert_impl;
        use futures_lite::{
            io::{AsyncRead, Cursor, Result as IoResult},
            pin,
        };
        use std::{
            fmt::Debug,
            pin::Pin,
            task::{Context, Poll},
        };

        trait AsyncReadDebug: AsyncRead + Unpin + Debug + Send + Sync {}
        impl<T: AsyncRead + Unpin + Debug + Send + Sync> AsyncReadDebug for T {}

        /// 异步 HTTP 响应体
        #[derive(Debug)]
        #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
        pub struct AsyncResponseBody(AsyncResponseBodyInner);

        #[derive(Debug)]
        enum AsyncResponseBodyInner {
            Reader(Box<dyn AsyncReadDebug>),
            Bytes(Cursor<Vec<u8>>),
        }

        impl AsyncResponseBody {
            /// 通过异步输入流创建异步 HTTP 响应体
            #[inline]
            pub fn from_reader(reader: impl AsyncRead + Unpin + Debug + Send + Sync + 'static) -> Self {
                Self(AsyncResponseBodyInner::Reader(Box::new(reader)))
            }

            /// 通过二进制数据创建异步 HTTP 响应体
            #[inline]
            pub fn from_bytes(bytes: Vec<u8>) -> Self {
                Self(AsyncResponseBodyInner::Bytes(Cursor::new(bytes)))
            }

            #[allow(dead_code)]
            fn ignore() {
                assert_impl!(Send: Self);
                assert_impl!(Sync: Self);
            }
        }

        impl Default for AsyncResponseBody {
            #[inline]
            fn default() -> Self {
                Self::from_bytes(Default::default())
            }
        }

        impl AsyncRead for AsyncResponseBody {
            fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<IoResult<usize>> {
                match &mut self.as_mut().0 {
                    AsyncResponseBodyInner::Reader(reader) => {
                        pin!(reader);
                        reader.poll_read(cx, buf)
                    }
                    AsyncResponseBodyInner::Bytes(bytes) => {
                        pin!(bytes);
                        bytes.poll_read(cx, buf)
                    }
                }
            }
        }
    }

    #[cfg(feature = "async")]
    pub use async_body::*;
}

pub use body::ResponseBody;

#[cfg(feature = "async")]
pub use body::AsyncResponseBody;

/// HTTP 响应结果
pub type Result<B> = result::Result<Response<B>, ResponseError>;
