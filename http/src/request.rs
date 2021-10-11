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
    fmt,
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

/// 请求体
pub type Body<'b> = Cow<'b, [u8]>;

type OnProgress<'r> = &'r (dyn Fn(&TransferProgressInfo) -> bool + Send + Sync);
type OnStatusCode<'r> = &'r (dyn Fn(StatusCode) -> bool + Send + Sync);
type OnHeader<'r> = &'r (dyn Fn(&HeaderName, &HeaderValue) -> bool + Send + Sync);

/// HTTP 请求
///
/// 封装 HTTP 请求相关字段
#[derive(Default)]
pub struct Request<'r> {
    inner: HTTPRequest<Body<'r>>,

    // 请求配置属性
    appended_user_agent: UserAgent,
    resolved_ip_addrs: Option<Cow<'r, [IpAddr]>>,
    on_uploading_progress: Option<OnProgress<'r>>,
    on_receive_response_status: Option<OnStatusCode<'r>>,
    on_receive_response_header: Option<OnHeader<'r>>,
}

impl<'r> Request<'r> {
    // 返回 HTTP 响应构建器
    #[inline]
    pub fn builder() -> RequestBuilder<'r> {
        RequestBuilder::default()
    }

    /// 获取 HTTP 请求
    #[inline]
    pub fn http(&self) -> &HTTPRequest<Body<'r>> {
        &self.inner
    }

    /// 修改 HTTP 请求
    #[inline]
    pub fn http_mut(&mut self) -> &mut HTTPRequest<Body<'r>> {
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
    pub fn body(&self) -> &[u8] {
        self.inner.body()
    }

    /// 修改请求体
    #[inline]
    pub fn body_mut(&mut self) -> &mut Body<'r> {
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

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl fmt::Debug for Request<'_> {
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
pub struct RequestBuilder<'r> {
    inner: Request<'r>,
}

impl<'r> RequestBuilder<'r> {
    /// 设置 HTTP 请求
    #[inline]
    pub fn http(&mut self, request: HTTPRequest<Body<'r>>) -> &mut Self {
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

    /// 设置请求体
    #[inline]
    pub fn body(&mut self, body: impl Into<Body<'r>>) -> &mut Self {
        *self.inner.body_mut() = body.into();
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

    /// 构建 HTTP 请求，同时构建器被重置
    #[inline]
    pub fn build(&mut self) -> Request<'r> {
        take(&mut self.inner)
    }

    /// 重置构建器
    #[inline]
    pub fn reset(&mut self) {
        self.inner = Default::default();
    }
}

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
