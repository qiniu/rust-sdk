use assert_impl::assert_impl;
use http::{
    header::{HeaderMap, HeaderName, HeaderValue},
    method::Method,
    request::Request as HTTPRequest,
    status::StatusCode,
    uri::Uri,
    Extensions, Version,
};
use once_cell::sync::Lazy;
use qiniu_utils::{smallstr::SmallString, wrap_smallstr};
use serde::{
    de::{Deserialize, Deserializer, Error, Visitor},
    ser::{Serialize, Serializer},
};
use std::{
    borrow::{Borrow, BorrowMut, Cow},
    fmt::{self, Debug},
    iter::FromIterator,
    mem::take,
    net::IpAddr,
    ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo},
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserAgent {
    inner: SmallString<[u8; 256]>,
}
wrap_smallstr!(UserAgent);

static FULL_USER_AGENT: Lazy<Box<str>> = Lazy::new(|| {
    format!(
        "QiniuRust/qiniu-http-{}/rust-{}",
        env!("CARGO_PKG_VERSION"),
        rustc_version_runtime::version(),
    )
    .into()
});

type OnProgress<'r> = &'r (dyn Fn(&TransferProgressInfo) -> bool + Send + Sync);
type OnStatusCode<'r> = &'r (dyn Fn(StatusCode) -> bool + Send + Sync);
type OnHeader<'r> = &'r (dyn Fn(&HeaderName, &HeaderValue) -> bool + Send + Sync);

/// HTTP 请求
///
/// 封装 HTTP 请求相关字段
#[derive(Default)]
pub struct Request<'r, B: 'r> {
    inner: HTTPRequest<B>,

    // 请求配置属性
    appended_user_agent: UserAgent,
    resolved_ip_addrs: Option<Cow<'r, [IpAddr]>>,
    on_uploading_progress: Option<OnProgress<'r>>,
    on_receive_response_status: Option<OnStatusCode<'r>>,
    on_receive_response_header: Option<OnHeader<'r>>,
}

impl<'r, B: Default + 'r> Request<'r, B> {
    // 返回 HTTP 响应构建器
    #[inline]
    pub fn builder() -> RequestBuilder<'r, B> {
        RequestBuilder::default()
    }
}

impl<'r, B: 'r> Request<'r, B> {
    /// 获取 HTTP 请求
    #[inline]
    pub fn http(&self) -> &HTTPRequest<B> {
        &self.inner
    }

    /// 修改 HTTP 请求
    #[inline]
    pub fn http_mut(&mut self) -> &mut HTTPRequest<B> {
        &mut self.inner
    }

    /// 请求 URL
    #[inline]
    pub fn url(&self) -> &Uri {
        self.inner.uri()
    }

    /// 修改请求 URL
    #[inline]
    pub fn url_mut(&mut self) -> &mut Uri {
        self.inner.uri_mut()
    }

    /// 请求 HTTP 版本
    #[inline]
    pub fn version(&self) -> Version {
        self.inner.version()
    }

    /// 修改请求 HTTP 版本
    #[inline]
    pub fn version_mut(&mut self) -> &mut Version {
        self.inner.version_mut()
    }

    /// 请求 HTTP 方法
    #[inline]
    pub fn method(&self) -> &Method {
        self.inner.method()
    }

    /// 修改请求 HTTP 方法
    #[inline]
    pub fn method_mut(&mut self) -> &mut Method {
        self.inner.method_mut()
    }

    /// 请求 HTTP Headers
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    /// 修改请求 HTTP Headers
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.inner.headers_mut()
    }

    /// 请求体
    #[inline]
    pub fn body(&self) -> &B {
        self.inner.body()
    }

    /// 修改请求体
    #[inline]
    pub fn body_mut(&mut self) -> &mut B {
        self.inner.body_mut()
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

    /// 用户代理
    #[inline]
    pub fn user_agent(&self) -> UserAgent {
        let mut user_agent = UserAgent::from(FULL_USER_AGENT.as_ref());
        user_agent.push_str(self.appended_user_agent().as_str());
        user_agent
    }

    /// 追加的用户代理
    #[inline]
    pub fn appended_user_agent(&self) -> &UserAgent {
        &self.appended_user_agent
    }

    /// 修改追加的用户代理
    #[inline]
    pub fn appended_user_agent_mut(&mut self) -> &mut UserAgent {
        &mut self.appended_user_agent
    }

    /// 预解析的服务器套接字地址
    #[inline]
    pub fn resolved_ip_addrs(&self) -> Option<&[IpAddr]> {
        self.resolved_ip_addrs.as_deref()
    }

    /// 修改预解析的服务器套接字地址
    #[inline]
    pub fn resolved_ip_addrs_mut(&mut self) -> &mut Option<Cow<'r, [IpAddr]>> {
        &mut self.resolved_ip_addrs
    }

    /// 上传进度回调
    #[inline]
    pub fn on_uploading_progress(&self) -> Option<OnProgress<'r>> {
        self.on_uploading_progress
    }

    /// 修改上传进度回调
    #[inline]
    pub fn on_uploading_progress_mut(&mut self) -> &mut Option<OnProgress<'r>> {
        &mut self.on_uploading_progress
    }

    /// 接受到响应状态回调
    #[inline]
    pub fn on_receive_response_status(&self) -> Option<OnStatusCode> {
        self.on_receive_response_status
    }

    /// 修改接受到响应状态回调
    #[inline]
    pub fn on_receive_response_status_mut(&mut self) -> &mut Option<OnStatusCode<'r>> {
        &mut self.on_receive_response_status
    }

    /// 接受到响应 Header 回调
    #[inline]
    pub fn on_receive_response_header(&self) -> Option<OnHeader> {
        self.on_receive_response_header
    }

    /// 修改接受到响应 Header 回调
    #[inline]
    pub fn on_receive_response_header_mut(&mut self) -> &mut Option<OnHeader<'r>> {
        &mut self.on_receive_response_header
    }
}

impl<'r, B: Send + Sync + 'r> Request<'r, B> {
    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl<'r, B: Debug + 'r> Debug for Request<'r, B> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        macro_rules! field {
            ($ctx:ident, $method_name:expr, $method:ident) => {
                $ctx.field($method_name, &self.$method)
            };
        }
        macro_rules! closure_field {
            ($ctx:ident, $method_name:expr, $method:ident) => {
                $ctx.field(
                    $method_name,
                    &self.$method.map_or_else(
                        || Cow::Borrowed("Uninstalled"),
                        |_| Cow::Borrowed("Installed"),
                    ),
                )
            };
        }
        let s = &mut f.debug_struct("Request");
        field!(s, "http", inner);
        field!(s, "appended_user_agent", appended_user_agent);
        field!(s, "resolved_ip_addrs", resolved_ip_addrs);
        closure_field!(s, "on_uploading_progress", on_uploading_progress);
        closure_field!(s, "on_receive_response_status", on_receive_response_status);
        closure_field!(s, "on_receive_response_header", on_receive_response_header);
        s.finish()
    }
}

/// HTTP 请求生成器
#[derive(Default, Debug)]
pub struct RequestBuilder<'r, B> {
    inner: Request<'r, B>,
}

impl<'r, B: 'r> RequestBuilder<'r, B> {
    /// 设置 HTTP 请求
    #[inline]
    pub fn http(&mut self, request: HTTPRequest<B>) -> &mut Self {
        self.inner.inner = request;
        self
    }

    /// 设置请求 URL
    #[inline]
    pub fn url(&mut self, url: Uri) -> &mut Self {
        *self.inner.url_mut() = url;
        self
    }

    /// 设置请求 HTTP 方法
    #[inline]
    pub fn method(&mut self, method: Method) -> &mut Self {
        *self.inner.method_mut() = method;
        self
    }

    /// 设置请求 HTTP 版本
    #[inline]
    pub fn version(&mut self, version: Version) -> &mut Self {
        *self.inner.version_mut() = version;
        self
    }

    /// 设置请求 HTTP Headers
    #[inline]
    pub fn headers(&mut self, headers: HeaderMap) -> &mut Self {
        *self.inner.headers_mut() = headers;
        self
    }

    #[inline]
    pub fn body(&mut self, body: B) -> &mut Self {
        *self.inner.body_mut() = body;
        self
    }

    /// 扩展字段
    #[inline]
    pub fn extensions(&mut self, extensions: Extensions) -> &mut Self {
        *self.inner.extensions_mut() = extensions;
        self
    }

    /// 设置用户代理
    #[inline]
    pub fn appended_user_agent(&mut self, user_agent: impl Into<UserAgent>) -> &mut Self {
        self.inner.appended_user_agent = user_agent.into();
        self
    }

    /// 设置预解析的服务器套接字地址
    #[inline]
    pub fn resolved_ip_addrs(
        &mut self,
        resolved_ip_addrs: impl Into<Cow<'r, [IpAddr]>>,
    ) -> &mut Self {
        self.inner.resolved_ip_addrs = Some(resolved_ip_addrs.into());
        self
    }

    /// 设置上传进度回调
    #[inline]
    pub fn on_uploading_progress(&mut self, f: OnProgress<'r>) -> &mut Self {
        self.inner.on_uploading_progress = Some(f);
        self
    }

    /// 接受到响应状态回调
    #[inline]
    pub fn on_receive_response_status(&mut self, f: OnStatusCode<'r>) -> &mut Self {
        self.inner.on_receive_response_status = Some(f);
        self
    }

    /// 接受到响应 Header 回调
    #[inline]
    pub fn on_receive_response_header(&mut self, f: OnHeader<'r>) -> &mut Self {
        self.inner.on_receive_response_header = Some(f);
        self
    }
}

impl<'r, B: Default + 'r> RequestBuilder<'r, B> {
    /// 构建 HTTP 请求，同时构建器被重置
    #[inline]
    pub fn build(&mut self) -> Request<'r, B> {
        take(&mut self.inner)
    }

    /// 重置构建器
    #[inline]
    pub fn reset(&mut self) {
        self.inner = Default::default();
    }
}

mod body {
    use super::super::Reset;
    use std::{
        default::Default,
        fmt::Debug,
        io::{Cursor, Read, Result as IOResult},
    };

    trait ReadDebug: Read + Reset + Debug + Send + Sync {}
    impl<T: Read + Reset + Debug + Send + Sync> ReadDebug for T {}

    #[derive(Debug)]
    struct OwnedRequestBody(OwnedRequestBodyInner);

    #[derive(Debug)]
    enum OwnedRequestBodyInner {
        Reader {
            reader: Box<dyn ReadDebug>,
            size: u64,
        },
        Bytes(Cursor<Vec<u8>>),
    }

    impl OwnedRequestBody {
        #[inline]
        fn from_reader(
            reader: impl Read + Reset + Debug + Send + Sync + 'static,
            size: u64,
        ) -> Self {
            Self(OwnedRequestBodyInner::Reader {
                reader: Box::new(reader),
                size,
            })
        }

        #[inline]
        fn from_bytes(bytes: Vec<u8>) -> Self {
            Self(OwnedRequestBodyInner::Bytes(Cursor::new(bytes)))
        }

        #[inline]
        fn size(&self) -> u64 {
            match &self.0 {
                OwnedRequestBodyInner::Reader { size, .. } => *size,
                OwnedRequestBodyInner::Bytes(bytes) => bytes.get_ref().len() as u64,
            }
        }
    }

    impl Default for OwnedRequestBody {
        #[inline]
        fn default() -> Self {
            Self::from_bytes(Default::default())
        }
    }

    impl Read for OwnedRequestBody {
        fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
            match &mut self.0 {
                OwnedRequestBodyInner::Reader { reader, .. } => reader.read(buf),
                OwnedRequestBodyInner::Bytes(bytes) => bytes.read(buf),
            }
        }
    }

    impl Reset for OwnedRequestBody {
        #[inline]
        fn reset(&mut self) -> IOResult<()> {
            match &mut self.0 {
                OwnedRequestBodyInner::Reader { reader, .. } => reader.reset(),
                OwnedRequestBodyInner::Bytes(bytes) => bytes.reset(),
            }
        }
    }

    /// HTTP 请求体
    #[derive(Debug)]
    pub struct RequestBody<'a>(RequestBodyInner<'a>);

    #[derive(Debug)]
    enum RequestBodyInner<'a> {
        ReaderRef {
            reader: &'a mut dyn ReadDebug,
            size: u64,
        },
        BytesRef(Cursor<&'a [u8]>),
        Owned(OwnedRequestBody),
    }

    impl<'a> RequestBody<'a> {
        #[inline]
        pub fn from_referenced_reader<T: Read + Reset + Debug + Send + Sync>(
            reader: &'a mut T,
            size: u64,
        ) -> Self {
            Self(RequestBodyInner::ReaderRef { reader, size })
        }

        #[inline]
        pub fn from_referenced_bytes(bytes: &'a [u8]) -> Self {
            Self(RequestBodyInner::BytesRef(Cursor::new(bytes)))
        }

        #[inline]
        pub fn from_reader(
            reader: impl Read + Reset + Debug + Send + Sync + 'static,
            size: u64,
        ) -> Self {
            Self(RequestBodyInner::Owned(OwnedRequestBody::from_reader(
                reader, size,
            )))
        }

        #[inline]
        pub fn from_bytes(bytes: Vec<u8>) -> Self {
            Self(RequestBodyInner::Owned(OwnedRequestBody::from_bytes(bytes)))
        }

        #[inline]
        pub fn size(&self) -> u64 {
            match &self.0 {
                RequestBodyInner::ReaderRef { size, .. } => *size,
                RequestBodyInner::BytesRef(bytes) => bytes.get_ref().len() as u64,
                RequestBodyInner::Owned(owned) => owned.size(),
            }
        }
    }

    impl Default for RequestBody<'_> {
        #[inline]
        fn default() -> Self {
            Self::from_bytes(Default::default())
        }
    }

    impl Read for RequestBody<'_> {
        #[inline]
        fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
            match &mut self.0 {
                RequestBodyInner::ReaderRef { reader, .. } => reader.read(buf),
                RequestBodyInner::BytesRef(bytes) => bytes.read(buf),
                RequestBodyInner::Owned(owned) => owned.read(buf),
            }
        }
    }

    impl Reset for RequestBody<'_> {
        #[inline]
        fn reset(&mut self) -> IOResult<()> {
            match &mut self.0 {
                RequestBodyInner::ReaderRef { reader, .. } => reader.reset(),
                RequestBodyInner::BytesRef(bytes) => bytes.reset(),
                RequestBodyInner::Owned(owned) => owned.reset(),
            }
        }
    }

    #[cfg(feature = "async")]
    mod async_body {
        use super::super::super::{AsyncReset, BoxFuture};
        use futures_lite::{
            io::{AsyncRead, Cursor, Result as IOResult},
            pin,
        };
        use std::{
            fmt::Debug,
            pin::Pin,
            task::{Context, Poll},
        };

        trait AsyncReadDebug: AsyncRead + AsyncReset + Unpin + Debug + Send + Sync {}
        impl<T: AsyncRead + AsyncReset + Unpin + Debug + Send + Sync> AsyncReadDebug for T {}

        #[derive(Debug)]
        #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
        struct OwnedAsyncRequestBody(OwnedAsyncRequestBodyInner);

        #[derive(Debug)]
        enum OwnedAsyncRequestBodyInner {
            Reader {
                reader: Box<dyn AsyncReadDebug>,
                size: u64,
            },
            Bytes(Cursor<Vec<u8>>),
        }

        impl OwnedAsyncRequestBody {
            #[inline]
            fn from_reader(
                reader: impl AsyncRead + AsyncReset + Unpin + Debug + Send + Sync + 'static,
                size: u64,
            ) -> Self {
                Self(OwnedAsyncRequestBodyInner::Reader {
                    reader: Box::new(reader),
                    size,
                })
            }

            #[inline]
            fn from_bytes(bytes: Vec<u8>) -> Self {
                Self(OwnedAsyncRequestBodyInner::Bytes(Cursor::new(bytes)))
            }

            #[inline]
            pub fn size(&self) -> u64 {
                match &self.0 {
                    OwnedAsyncRequestBodyInner::Reader { size, .. } => *size,
                    OwnedAsyncRequestBodyInner::Bytes(bytes) => bytes.get_ref().len() as u64,
                }
            }
        }

        impl Default for OwnedAsyncRequestBody {
            #[inline]
            fn default() -> Self {
                Self::from_bytes(Default::default())
            }
        }

        impl AsyncRead for OwnedAsyncRequestBody {
            fn poll_read(
                mut self: Pin<&mut Self>,
                cx: &mut Context,
                buf: &mut [u8],
            ) -> Poll<IOResult<usize>> {
                match &mut self.as_mut().0 {
                    OwnedAsyncRequestBodyInner::Reader { reader, .. } => {
                        pin!(reader);
                        reader.poll_read(cx, buf)
                    }
                    OwnedAsyncRequestBodyInner::Bytes(bytes) => {
                        pin!(bytes);
                        bytes.poll_read(cx, buf)
                    }
                }
            }
        }

        impl AsyncReset for OwnedAsyncRequestBody {
            #[inline]
            fn reset(&mut self) -> BoxFuture<IOResult<()>> {
                Box::pin(async move {
                    match &mut self.0 {
                        OwnedAsyncRequestBodyInner::Reader { reader, .. } => reader.reset().await,
                        OwnedAsyncRequestBodyInner::Bytes(bytes) => bytes.reset().await,
                    }
                })
            }
        }

        /// 异步 HTTP 请求体
        #[derive(Debug)]
        #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
        pub struct AsyncRequestBody<'a>(AsyncRequestBodyInner<'a>);

        #[derive(Debug)]
        enum AsyncRequestBodyInner<'a> {
            ReaderRef {
                reader: &'a mut dyn AsyncReadDebug,
                size: u64,
            },
            BytesRef(Cursor<&'a [u8]>),
            Owned(OwnedAsyncRequestBody),
        }

        impl<'a> AsyncRequestBody<'a> {
            #[inline]
            pub fn from_referenced_reader<
                T: AsyncRead + AsyncReset + Unpin + Debug + Send + Sync,
            >(
                reader: &'a mut T,
                size: u64,
            ) -> Self {
                Self(AsyncRequestBodyInner::ReaderRef { reader, size })
            }

            #[inline]
            pub fn from_referenced_bytes(bytes: &'a [u8]) -> Self {
                Self(AsyncRequestBodyInner::BytesRef(Cursor::new(bytes)))
            }

            #[inline]
            pub fn from_reader(
                reader: impl AsyncRead + AsyncReset + Unpin + Debug + Send + Sync + 'static,
                size: u64,
            ) -> Self {
                Self(AsyncRequestBodyInner::Owned(
                    OwnedAsyncRequestBody::from_reader(reader, size),
                ))
            }

            #[inline]
            pub fn from_bytes(bytes: Vec<u8>) -> Self {
                Self(AsyncRequestBodyInner::Owned(
                    OwnedAsyncRequestBody::from_bytes(bytes),
                ))
            }

            #[inline]
            pub fn size(&self) -> u64 {
                match &self.0 {
                    AsyncRequestBodyInner::ReaderRef { size, .. } => *size,
                    AsyncRequestBodyInner::BytesRef(bytes) => bytes.get_ref().len() as u64,
                    AsyncRequestBodyInner::Owned(owned) => owned.size(),
                }
            }
        }

        impl Default for AsyncRequestBody<'_> {
            #[inline]
            fn default() -> Self {
                Self::from_bytes(Default::default())
            }
        }

        impl AsyncRead for AsyncRequestBody<'_> {
            fn poll_read(
                mut self: Pin<&mut Self>,
                cx: &mut Context,
                buf: &mut [u8],
            ) -> Poll<IOResult<usize>> {
                match &mut self.as_mut().0 {
                    AsyncRequestBodyInner::ReaderRef { reader, .. } => {
                        pin!(reader);
                        reader.poll_read(cx, buf)
                    }
                    AsyncRequestBodyInner::BytesRef(bytes) => {
                        pin!(bytes);
                        bytes.poll_read(cx, buf)
                    }
                    AsyncRequestBodyInner::Owned(owned) => {
                        pin!(owned);
                        owned.poll_read(cx, buf)
                    }
                }
            }
        }

        impl AsyncReset for AsyncRequestBody<'_> {
            #[inline]
            fn reset(&mut self) -> BoxFuture<IOResult<()>> {
                Box::pin(async move {
                    match &mut self.0 {
                        AsyncRequestBodyInner::ReaderRef { reader, .. } => reader.reset().await,
                        AsyncRequestBodyInner::BytesRef(bytes) => bytes.reset().await,
                        AsyncRequestBodyInner::Owned(owned) => owned.reset().await,
                    }
                })
            }
        }
    }

    #[cfg(feature = "async")]
    pub use async_body::*;
}

pub use body::RequestBody;

#[cfg(feature = "async")]
pub use body::AsyncRequestBody;

/// 上传进度信息
pub struct TransferProgressInfo<'b> {
    transferred_bytes: u64,
    total_bytes: u64,
    body: &'b [u8],
}

impl<'b> TransferProgressInfo<'b> {
    #[inline]
    pub fn new(transferred_bytes: u64, total_bytes: u64, body: &'b [u8]) -> Self {
        Self {
            transferred_bytes,
            total_bytes,
            body,
        }
    }

    #[inline]
    pub fn transferred_bytes(&self) -> u64 {
        self.transferred_bytes
    }

    #[inline]
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    #[inline]
    pub fn body(&self) -> &[u8] {
        self.body
    }
}
