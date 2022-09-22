use super::{
    callback::{OnHeader, OnHeaderCallback, OnProgress, OnProgressCallback, OnStatusCode, OnStatusCodeCallback},
    LIBRARY_USER_AGENT,
};
use assert_impl::assert_impl;
use http::{
    header::{HeaderMap, IntoHeaderName},
    method::Method,
    request::{Parts as HttpRequestParts, Request as HttpRequest},
    uri::Uri,
    Extensions, HeaderValue, Version,
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

/// UserAgent 信息
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserAgent {
    inner: SmallString<[u8; 256]>,
}
wrap_smallstr!(UserAgent);

static FULL_USER_AGENT: Lazy<Box<str>> = Lazy::new(|| {
    format!(
        "QiniuRust/qiniu-http-{}/rust-{}",
        env!("CARGO_PKG_VERSION"),
        env!("RUSTC_VERSION"),
    )
    .into()
});

/// HTTP 请求信息
///
/// 不包含请求体信息
pub struct RequestParts<'r> {
    inner: HttpRequestParts,

    // 请求配置属性
    appended_user_agent: UserAgent,
    resolved_ip_addrs: Option<Cow<'r, [IpAddr]>>,
    on_uploading_progress: Option<OnProgressCallback<'r>>,
    on_receive_response_status: Option<OnStatusCodeCallback<'r>>,
    on_receive_response_header: Option<OnHeaderCallback<'r>>,
}

impl<'r> RequestParts<'r> {
    /// 创建 HTTP 请求信息构建器
    #[inline]
    pub fn builder() -> RequestPartsBuilder<'r> {
        RequestPartsBuilder::default()
    }

    /// 获取 HTTP 请求 URL
    #[inline]
    pub fn url(&self) -> &Uri {
        &self.inner.uri
    }

    /// 获取 HTTP 请求 URL 的可变引用
    #[inline]
    pub fn url_mut(&mut self) -> &mut Uri {
        &mut self.inner.uri
    }

    /// 获取请求 HTTP 版本
    #[inline]
    pub fn version(&self) -> Version {
        self.inner.version
    }

    /// 获取请求 HTTP 版本的可变引用
    #[inline]
    pub fn version_mut(&mut self) -> &mut Version {
        &mut self.inner.version
    }

    /// 获取请求 HTTP 方法
    #[inline]
    pub fn method(&self) -> &Method {
        &self.inner.method
    }

    /// 获取请求 HTTP 方法的可变引用
    #[inline]
    pub fn method_mut(&mut self) -> &mut Method {
        &mut self.inner.method
    }

    /// 获取请求 HTTP Headers
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.inner.headers
    }

    /// 获取请求 HTTP Headers 的可变引用
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.inner.headers
    }

    /// 获取扩展信息
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.inner.extensions
    }

    /// 获取扩展信息的可变引用
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.inner.extensions
    }

    /// 获取 UserAgent
    #[inline]
    pub fn user_agent(&self) -> UserAgent {
        let mut user_agent = UserAgent::from(FULL_USER_AGENT.as_ref());
        if let Some(lib_user_agent) = LIBRARY_USER_AGENT.get() {
            user_agent.push_str(lib_user_agent);
        }
        user_agent.push_str(self.appended_user_agent().as_str());
        user_agent
    }

    /// 获取追加的 UserAgent
    #[inline]
    pub fn appended_user_agent(&self) -> &UserAgent {
        &self.appended_user_agent
    }

    /// 获取追加的 UserAgent 的可变引用
    #[inline]
    pub fn appended_user_agent_mut(&mut self) -> &mut UserAgent {
        &mut self.appended_user_agent
    }

    /// 获取预解析的服务器套接字地址
    #[inline]
    pub fn resolved_ip_addrs(&self) -> Option<&[IpAddr]> {
        self.resolved_ip_addrs.as_deref()
    }

    /// 获取预解析的服务器套接字地址的可变引用
    #[inline]
    pub fn resolved_ip_addrs_mut(&mut self) -> &mut Option<Cow<'r, [IpAddr]>> {
        &mut self.resolved_ip_addrs
    }

    /// 获取上传进度回调
    #[inline]
    pub fn on_uploading_progress(&'r self) -> Option<OnProgress<'r>> {
        self.on_uploading_progress.as_deref()
    }

    /// 获取上传进度回调的可变引用
    #[inline]
    pub fn on_uploading_progress_mut(&mut self) -> &mut Option<OnProgressCallback<'r>> {
        &mut self.on_uploading_progress
    }

    /// 获取接受到响应状态回调
    #[inline]
    pub fn on_receive_response_status(&'r self) -> Option<OnStatusCode<'r>> {
        self.on_receive_response_status.as_deref()
    }

    /// 获取接受到响应状态回调的可变引用
    #[inline]
    pub fn on_receive_response_status_mut(&mut self) -> &mut Option<OnStatusCodeCallback<'r>> {
        &mut self.on_receive_response_status
    }

    /// 获取接受到响应 Header 回调
    #[inline]
    pub fn on_receive_response_header(&'r self) -> Option<OnHeader<'r>> {
        self.on_receive_response_header.as_deref()
    }

    /// 获取接受到响应 Header 回调的可变引用
    #[inline]
    pub fn on_receive_response_header_mut(&mut self) -> &mut Option<OnHeaderCallback<'r>> {
        &mut self.on_receive_response_header
    }
}

impl Default for RequestParts<'_> {
    #[inline]
    fn default() -> Self {
        let (parts, _) = HttpRequest::new(()).into_parts();
        Self {
            inner: parts,
            appended_user_agent: Default::default(),
            resolved_ip_addrs: Default::default(),
            on_uploading_progress: Default::default(),
            on_receive_response_status: Default::default(),
            on_receive_response_header: Default::default(),
        }
    }
}

impl Debug for RequestParts<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        macro_rules! field {
            ($ctx:ident, $method_name:expr, $method:ident) => {
                $ctx.field($method_name, &self.$method)
            };
        }
        macro_rules! closure_field {
            ($ctx:ident, $method_name:expr, $method:ident) => {
                $ctx.field(
                    $method_name,
                    &self
                        .$method
                        .as_ref()
                        .map_or_else(|| Cow::Borrowed("Uninstalled"), |_| Cow::Borrowed("Installed")),
                )
            };
        }
        let s = &mut f.debug_struct("Request");
        field!(s, "inner", inner);
        field!(s, "appended_user_agent", appended_user_agent);
        field!(s, "resolved_ip_addrs", resolved_ip_addrs);
        closure_field!(s, "on_uploading_progress", on_uploading_progress);
        closure_field!(s, "on_receive_response_status", on_receive_response_status);
        closure_field!(s, "on_receive_response_header", on_receive_response_header);
        s.finish()
    }
}

/// HTTP 请求信息构建器
///
/// 不包含请求体信息
#[derive(Debug, Default)]
pub struct RequestPartsBuilder<'r>(RequestParts<'r>);

impl<'r> RequestPartsBuilder<'r> {
    /// 创建 HTTP 请求信息构建器
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    /// 设置 HTTP 请求 URL
    #[inline]
    pub fn url(&mut self, url: Uri) -> &mut Self {
        self.0.inner.uri = url;
        self
    }

    /// 设置请求 HTTP 版本
    #[inline]
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.0.inner.version = version;
        self
    }

    /// 设置请求 HTTP 方法
    #[inline]
    pub fn method(&mut self, method: Method) -> &mut Self {
        self.0.inner.method = method;
        self
    }

    /// 设置请求 HTTP Headers
    #[inline]
    pub fn headers(&mut self, headers: HeaderMap) -> &mut Self {
        self.0.inner.headers = headers;
        self
    }

    /// 插入请求 HTTP Header
    #[inline]
    pub fn header(&mut self, header_name: impl IntoHeaderName, header_value: impl Into<HeaderValue>) -> &mut Self {
        self.0.inner.headers.insert(header_name, header_value.into());
        self
    }

    /// 设置扩展信息
    #[inline]
    pub fn extensions(&mut self, extensions: Extensions) -> &mut Self {
        self.0.inner.extensions = extensions;
        self
    }

    /// 追加扩展信息
    #[inline]
    pub fn add_extension<T: Sync + Send + 'static>(&mut self, val: T) -> &mut Self {
        self.0.inner.extensions.insert(val);
        self
    }

    /// 设置 UserAgent
    #[inline]
    pub fn appended_user_agent(&mut self, user_agent: impl Into<UserAgent>) -> &mut Self {
        self.0.appended_user_agent = user_agent.into();
        self
    }

    /// 设置预解析的服务器套接字地址
    #[inline]
    pub fn resolved_ip_addrs(&mut self, resolved_ip_addrs: impl Into<Cow<'r, [IpAddr]>>) -> &mut Self {
        self.0.resolved_ip_addrs = Some(resolved_ip_addrs.into());
        self
    }

    /// 设置上传进度回调
    #[inline]
    pub fn on_uploading_progress(&mut self, f: impl Into<OnProgressCallback<'r>>) -> &mut Self {
        self.0.on_uploading_progress = Some(f.into());
        self
    }

    /// 设置接受到响应状态回调
    #[inline]
    pub fn on_receive_response_status(&mut self, f: impl Into<OnStatusCodeCallback<'r>>) -> &mut Self {
        self.0.on_receive_response_status = Some(f.into());
        self
    }

    /// 设置接受到响应 Header 回调
    #[inline]
    pub fn on_receive_response_header(&mut self, f: impl Into<OnHeaderCallback<'r>>) -> &mut Self {
        self.0.on_receive_response_header = Some(f.into());
        self
    }

    /// 创建 HTTP 请求信息
    #[inline]
    pub fn build(&mut self) -> RequestParts<'r> {
        take(&mut self.0)
    }

    /// 创建 HTTP 请求
    #[inline]
    pub fn build_with_body<B: 'r>(&mut self, body: B) -> Request<'r, B> {
        let parts = self.build();
        Request { parts, body }
    }
}

/// HTTP 请求
///
/// 封装 HTTP 请求相关字段
#[derive(Default, Debug)]
pub struct Request<'r, B: 'r> {
    parts: RequestParts<'r>,
    body: B,
}

impl<'r, B: Default + 'r> Request<'r, B> {
    /// 创建 HTTP 请求构建器
    #[inline]
    pub fn builder() -> RequestBuilder<'r, B> {
        RequestBuilder::default()
    }
}

impl<'r, B: 'r> Request<'r, B> {
    /// 获取请求体
    #[inline]
    pub fn body(&self) -> &B {
        &self.body
    }

    /// 获取请求体的可变引用
    #[inline]
    pub fn body_mut(&mut self) -> &mut B {
        &mut self.body
    }

    /// 转换为 HTTP 请求体
    #[inline]
    pub fn into_body(self) -> B {
        self.body
    }

    /// 获取请求信息
    #[inline]
    pub fn parts(&self) -> &RequestParts<'r> {
        &self.parts
    }

    /// 获取请求信息的可变引用
    #[inline]
    pub fn parts_mut(&mut self) -> &mut RequestParts<'r> {
        &mut self.parts
    }

    /// 转换为请求信息和请求体
    #[inline]
    pub fn into_parts_and_body(self) -> (RequestParts<'r>, B) {
        let Self { parts, body } = self;
        (parts, body)
    }

    /// 通过请求信息和请求体创建 HTTP 请求
    #[inline]
    pub fn from_parts_and_body(parts: RequestParts<'r>, body: B) -> Self {
        Self { parts, body }
    }
}

impl<'r, B: Send + Sync + 'r> Request<'r, B> {
    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl<'r, B: 'r> Deref for Request<'r, B> {
    type Target = RequestParts<'r>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.parts
    }
}

impl<'r, B: 'r> DerefMut for Request<'r, B> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.parts
    }
}

/// HTTP 请求构建器
#[derive(Default, Debug)]
pub struct RequestBuilder<'r, B> {
    inner: Request<'r, B>,
}

impl<'r, B: Default + 'r> RequestBuilder<'r, B> {
    /// 创建 HTTP 请求构建器
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<'r, B: 'r> RequestBuilder<'r, B> {
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

    /// 插入请求 HTTP Header
    #[inline]
    pub fn header(&mut self, header_name: impl IntoHeaderName, header_value: impl Into<HeaderValue>) -> &mut Self {
        self.inner.headers_mut().insert(header_name, header_value.into());
        self
    }

    /// 设置请求 HTTP 请求体
    #[inline]
    pub fn body(&mut self, body: B) -> &mut Self {
        *self.inner.body_mut() = body;
        self
    }

    /// 设置扩展信息
    #[inline]
    pub fn extensions(&mut self, extensions: Extensions) -> &mut Self {
        *self.inner.extensions_mut() = extensions;
        self
    }

    /// 追加扩展信息
    #[inline]
    pub fn add_extension<T: Sync + Send + 'static>(&mut self, val: T) -> &mut Self {
        self.inner.extensions_mut().insert(val);
        self
    }

    /// 设置 UserAgent
    #[inline]
    pub fn appended_user_agent(&mut self, user_agent: impl Into<UserAgent>) -> &mut Self {
        *self.inner.appended_user_agent_mut() = user_agent.into();
        self
    }

    /// 设置预解析的服务器套接字地址
    #[inline]
    pub fn resolved_ip_addrs(&mut self, resolved_ip_addrs: impl Into<Cow<'r, [IpAddr]>>) -> &mut Self {
        *self.inner.resolved_ip_addrs_mut() = Some(resolved_ip_addrs.into());
        self
    }

    /// 设置上传进度回调
    #[inline]
    pub fn on_uploading_progress(&mut self, f: impl Into<OnProgressCallback<'r>>) -> &mut Self {
        *self.inner.on_uploading_progress_mut() = Some(f.into());
        self
    }

    /// 设置接受到响应状态回调
    #[inline]
    pub fn on_receive_response_status(&mut self, f: impl Into<OnStatusCodeCallback<'r>>) -> &mut Self {
        *self.inner.on_receive_response_status_mut() = Some(f.into());
        self
    }

    /// 设置接受到响应 Header 回调
    #[inline]
    pub fn on_receive_response_header(&mut self, f: impl Into<OnHeaderCallback<'r>>) -> &mut Self {
        *self.inner.on_receive_response_header_mut() = Some(f.into());
        self
    }
}

impl<'r, B: Default + 'r> RequestBuilder<'r, B> {
    /// 构建 HTTP 请求，同时构建器被重置
    #[inline]
    pub fn build(&mut self) -> Request<'r, B> {
        take(&mut self.inner)
    }

    /// 重置 HTTP 请求构建器
    #[inline]
    pub fn reset(&mut self) {
        self.inner = Default::default();
    }
}

mod body {
    use super::super::Reset;
    use assert_impl::assert_impl;
    use std::{
        default::Default,
        fmt::Debug,
        io::{Cursor, Read, Result as IoResult},
    };

    trait ReadDebug: Read + Reset + Debug + Send + Sync {}
    impl<T: Read + Reset + Debug + Send + Sync> ReadDebug for T {}

    #[derive(Debug)]
    struct OwnedRequestBody(OwnedRequestBodyInner);

    #[derive(Debug)]
    enum OwnedRequestBodyInner {
        Reader { reader: Box<dyn ReadDebug>, size: u64 },
        Bytes(Cursor<Vec<u8>>),
    }

    impl OwnedRequestBody {
        fn from_reader(reader: impl Read + Reset + Debug + Send + Sync + 'static, size: u64) -> Self {
            Self(OwnedRequestBodyInner::Reader {
                reader: Box::new(reader),
                size,
            })
        }

        fn from_bytes(bytes: Vec<u8>) -> Self {
            Self(OwnedRequestBodyInner::Bytes(Cursor::new(bytes)))
        }

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
        fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
            match &mut self.0 {
                OwnedRequestBodyInner::Reader { reader, .. } => reader.read(buf),
                OwnedRequestBodyInner::Bytes(bytes) => bytes.read(buf),
            }
        }
    }

    impl Reset for OwnedRequestBody {
        #[inline]
        fn reset(&mut self) -> IoResult<()> {
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
        ReaderRef { reader: &'a mut dyn ReadDebug, size: u64 },
        BytesRef(Cursor<&'a [u8]>),
        Owned(OwnedRequestBody),
    }

    impl<'a> RequestBody<'a> {
        /// 通过输入流的可变引用创建 HTTP 请求体
        #[inline]
        pub fn from_referenced_reader<T: Read + Reset + Debug + Send + Sync + 'a>(
            reader: &'a mut T,
            size: u64,
        ) -> Self {
            Self(RequestBodyInner::ReaderRef { reader, size })
        }

        /// 通过二进制数据的引用创建 HTTP 请求体
        #[inline]
        pub fn from_referenced_bytes(bytes: &'a [u8]) -> Self {
            Self(RequestBodyInner::BytesRef(Cursor::new(bytes)))
        }

        /// 通过输入流创建 HTTP 请求体
        #[inline]
        pub fn from_reader(reader: impl Read + Reset + Debug + Send + Sync + 'static, size: u64) -> Self {
            Self(RequestBodyInner::Owned(OwnedRequestBody::from_reader(reader, size)))
        }

        /// 通过二进制数据创建 HTTP 请求体
        #[inline]
        pub fn from_bytes(bytes: Vec<u8>) -> Self {
            Self(RequestBodyInner::Owned(OwnedRequestBody::from_bytes(bytes)))
        }

        /// 获取请求体大小
        ///
        /// 单位为字节
        #[inline]
        pub fn size(&self) -> u64 {
            match &self.0 {
                RequestBodyInner::ReaderRef { size, .. } => *size,
                RequestBodyInner::BytesRef(bytes) => bytes.get_ref().len() as u64,
                RequestBodyInner::Owned(owned) => owned.size(),
            }
        }

        #[allow(dead_code)]
        fn ignore() {
            assert_impl!(Send: Self);
            assert_impl!(Sync: Self);
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
        fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
            match &mut self.0 {
                RequestBodyInner::ReaderRef { reader, .. } => reader.read(buf),
                RequestBodyInner::BytesRef(bytes) => bytes.read(buf),
                RequestBodyInner::Owned(owned) => owned.read(buf),
            }
        }
    }

    impl Reset for RequestBody<'_> {
        #[inline]
        fn reset(&mut self) -> IoResult<()> {
            match &mut self.0 {
                RequestBodyInner::ReaderRef { reader, .. } => reader.reset(),
                RequestBodyInner::BytesRef(bytes) => bytes.reset(),
                RequestBodyInner::Owned(owned) => owned.reset(),
            }
        }
    }

    impl<'a> From<&'a mut RequestBody<'_>> for RequestBody<'a> {
        #[inline]
        fn from(body: &'a mut RequestBody<'_>) -> Self {
            Self::from_referenced_reader(body, body.size())
        }
    }

    impl<'a> From<&'a [u8]> for RequestBody<'a> {
        #[inline]
        fn from(body: &'a [u8]) -> Self {
            Self::from_referenced_bytes(body)
        }
    }

    impl<'a> From<&'a str> for RequestBody<'a> {
        #[inline]
        fn from(body: &'a str) -> Self {
            Self::from_referenced_bytes(body.as_bytes())
        }
    }

    impl From<Vec<u8>> for RequestBody<'_> {
        #[inline]
        fn from(body: Vec<u8>) -> Self {
            Self::from_bytes(body)
        }
    }

    impl From<String> for RequestBody<'_> {
        #[inline]
        fn from(body: String) -> Self {
            Self::from_bytes(body.into_bytes())
        }
    }

    #[cfg(feature = "async")]
    mod async_body {
        use super::super::super::{AsyncReset, BoxFuture};
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

        trait AsyncReadDebug: AsyncRead + AsyncReset + Unpin + Debug + Send + Sync {}
        impl<T: AsyncRead + AsyncReset + Unpin + Debug + Send + Sync> AsyncReadDebug for T {}

        #[derive(Debug)]
        #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
        struct OwnedAsyncRequestBody(OwnedAsyncRequestBodyInner);

        #[derive(Debug)]
        enum OwnedAsyncRequestBodyInner {
            Reader { reader: Box<dyn AsyncReadDebug>, size: u64 },
            Bytes(Cursor<Vec<u8>>),
        }

        impl OwnedAsyncRequestBody {
            fn from_reader(
                reader: impl AsyncRead + AsyncReset + Unpin + Debug + Send + Sync + 'static,
                size: u64,
            ) -> Self {
                Self(OwnedAsyncRequestBodyInner::Reader {
                    reader: Box::new(reader),
                    size,
                })
            }

            fn from_bytes(bytes: Vec<u8>) -> Self {
                Self(OwnedAsyncRequestBodyInner::Bytes(Cursor::new(bytes)))
            }

            #[inline]
            fn size(&self) -> u64 {
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
            fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<IoResult<usize>> {
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
            fn reset(&mut self) -> BoxFuture<IoResult<()>> {
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
        #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
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
            /// 通过异步输入流的可变引用创建异步 HTTP 请求体
            #[inline]
            pub fn from_referenced_reader<T: AsyncRead + AsyncReset + Unpin + Debug + Send + Sync + 'a>(
                reader: &'a mut T,
                size: u64,
            ) -> Self {
                Self(AsyncRequestBodyInner::ReaderRef { reader, size })
            }

            /// 通过二进制数据的引用创建异步 HTTP 请求体
            #[inline]
            pub fn from_referenced_bytes(bytes: &'a [u8]) -> Self {
                Self(AsyncRequestBodyInner::BytesRef(Cursor::new(bytes)))
            }

            /// 通过异步输入流创建异步 HTTP 请求体
            #[inline]
            pub fn from_reader(
                reader: impl AsyncRead + AsyncReset + Unpin + Debug + Send + Sync + 'static,
                size: u64,
            ) -> Self {
                Self(AsyncRequestBodyInner::Owned(OwnedAsyncRequestBody::from_reader(
                    reader, size,
                )))
            }

            /// 通过二进制数据创建异步 HTTP 请求体
            #[inline]
            pub fn from_bytes(bytes: Vec<u8>) -> Self {
                Self(AsyncRequestBodyInner::Owned(OwnedAsyncRequestBody::from_bytes(bytes)))
            }

            /// 获取请求体大小
            ///
            /// 单位为字节
            #[inline]
            pub fn size(&self) -> u64 {
                match &self.0 {
                    AsyncRequestBodyInner::ReaderRef { size, .. } => *size,
                    AsyncRequestBodyInner::BytesRef(bytes) => bytes.get_ref().len() as u64,
                    AsyncRequestBodyInner::Owned(owned) => owned.size(),
                }
            }

            #[allow(dead_code)]
            fn ignore() {
                assert_impl!(Send: Self);
                assert_impl!(Sync: Self);
            }
        }

        impl Default for AsyncRequestBody<'_> {
            #[inline]
            fn default() -> Self {
                Self::from_bytes(Default::default())
            }
        }

        impl AsyncRead for AsyncRequestBody<'_> {
            fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<IoResult<usize>> {
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
            fn reset(&mut self) -> BoxFuture<IoResult<()>> {
                Box::pin(async move {
                    match &mut self.0 {
                        AsyncRequestBodyInner::ReaderRef { reader, .. } => reader.reset().await,
                        AsyncRequestBodyInner::BytesRef(bytes) => bytes.reset().await,
                        AsyncRequestBodyInner::Owned(owned) => owned.reset().await,
                    }
                })
            }
        }

        impl<'a> From<&'a mut AsyncRequestBody<'_>> for AsyncRequestBody<'a> {
            #[inline]
            fn from(body: &'a mut AsyncRequestBody<'_>) -> Self {
                Self::from_referenced_reader(body, body.size())
            }
        }

        impl<'a> From<&'a [u8]> for AsyncRequestBody<'a> {
            #[inline]
            fn from(body: &'a [u8]) -> Self {
                Self::from_referenced_bytes(body)
            }
        }

        impl<'a> From<&'a str> for AsyncRequestBody<'a> {
            #[inline]
            fn from(body: &'a str) -> Self {
                Self::from_referenced_bytes(body.as_bytes())
            }
        }

        impl From<Vec<u8>> for AsyncRequestBody<'_> {
            #[inline]
            fn from(body: Vec<u8>) -> Self {
                Self::from_bytes(body)
            }
        }

        impl From<String> for AsyncRequestBody<'_> {
            #[inline]
            fn from(body: String) -> Self {
                Self::from_bytes(body.into_bytes())
            }
        }
    }

    #[cfg(feature = "async")]
    pub use async_body::*;
}

pub use body::RequestBody;

#[cfg(feature = "async")]
pub use body::AsyncRequestBody;
