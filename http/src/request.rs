use super::{HeaderName, HeaderValue, Headers, Method, StatusCode};
use assert_impl::assert_impl;
use once_cell::sync::Lazy;
use std::{borrow::Cow, fmt, mem::take, net::IpAddr, path::Path, time::Duration};

static FULL_USER_AGENT: Lazy<Box<str>> = Lazy::new(|| {
    format!(
        "QiniuRust/qiniu-http-{}/rust-{}",
        env!("CARGO_PKG_VERSION"),
        rustc_version_runtime::version(),
    )
    .into()
});

/// 请求 URL
pub type URL<'s> = Cow<'s, str>;

/// 请求体
pub type Body<'b> = Cow<'b, [u8]>;

type OnProgress<'r> = Option<&'r (dyn Fn(u64, u64) -> bool + Send + Sync)>;
type OnBody<'r> = Option<&'r (dyn Fn(&[u8]) -> bool + Send + Sync)>;
type OnStatusCode<'r> = Option<&'r (dyn Fn(StatusCode) -> bool + Send + Sync)>;
type OnHeader<'r> = Option<&'r (dyn Fn(&HeaderName, &HeaderValue) -> bool + Send + Sync)>;

/// HTTP 请求
///
/// 封装 HTTP 请求相关字段
pub struct Request<'r> {
    url: URL<'r>,
    method: Method,
    headers: Headers<'r>,
    body: Body<'r>,

    // 请求配置属性
    response_body_buffer_path: Option<Cow<'r, Path>>,
    appended_user_agent: Cow<'r, str>,
    follow_redirection: bool,
    resolved_ip_addrs: Cow<'r, [IpAddr]>,
    on_uploading_progress: OnProgress<'r>,
    on_downloading_progress: OnProgress<'r>,
    on_send_request_body: OnBody<'r>,
    on_receive_response_status: OnStatusCode<'r>,
    on_receive_response_body: OnBody<'r>,
    on_receive_response_header: OnHeader<'r>,
    connect_timeout: Duration,
    request_timeout: Duration,
    tcp_keepalive_idle_timeout: Duration,
    tcp_keepalive_probe_interval: Duration,
    low_transfer_speed: u32,
    low_transfer_speed_timeout: Duration,
}

impl<'r> Request<'r> {
    // 返回 HTTP 响应构建器
    #[inline]
    pub fn builder() -> RequestBuilder<'r> {
        RequestBuilder::default()
    }

    /// 请求 URL
    #[inline]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// 修改请求 URL
    #[inline]
    pub fn url_mut(&mut self) -> &mut URL<'r> {
        &mut self.url
    }

    /// 请求 HTTP 方法
    #[inline]
    pub fn method(&self) -> Method {
        self.method
    }

    /// 修改请求 HTTP 方法
    #[inline]
    pub fn method_mut(&mut self) -> &mut Method {
        &mut self.method
    }

    /// 请求 HTTP Headers
    #[inline]
    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    /// 修改请求 HTTP Headers
    #[inline]
    pub fn headers_mut(&mut self) -> &mut Headers<'r> {
        &mut self.headers
    }

    /// 请求体
    #[inline]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// 修改请求体
    #[inline]
    pub fn body_mut(&mut self) -> &mut Body<'r> {
        &mut self.body
    }

    /// 请求体
    #[inline]
    pub fn response_body_buffer_path(&self) -> Option<&Path> {
        self.response_body_buffer_path.as_deref()
    }

    /// 修改请求体
    #[inline]
    pub fn response_body_buffer_path_mut(&mut self) -> &mut Option<Cow<'r, Path>> {
        &mut self.response_body_buffer_path
    }

    /// 用户代理
    #[inline]
    pub fn user_agent(&self) -> String {
        FULL_USER_AGENT.to_string() + self.appended_user_agent()
    }

    /// 追加的用户代理
    #[inline]
    pub fn appended_user_agent(&self) -> &str {
        &self.appended_user_agent
    }

    /// 修改追加的用户代理
    #[inline]
    pub fn appended_user_agent_mut(&mut self) -> &mut Cow<'r, str> {
        &mut self.appended_user_agent
    }

    /// 是否自动跟踪重定向
    #[inline]
    pub fn follow_redirection(&self) -> bool {
        self.follow_redirection
    }

    /// 修改自动跟踪重定向
    #[inline]
    pub fn follow_redirection_mut(&mut self) -> &mut bool {
        &mut self.follow_redirection
    }

    /// 预解析的服务器套接字地址
    #[inline]
    pub fn resolved_ip_addrs(&self) -> &[IpAddr] {
        &self.resolved_ip_addrs
    }

    /// 修改预解析的服务器套接字地址
    #[inline]
    pub fn resolved_ip_addrs_mut(&mut self) -> &mut Cow<'r, [IpAddr]> {
        &mut self.resolved_ip_addrs
    }

    /// 上传进度回调
    #[inline]
    pub fn on_uploading_progress(&self) -> OnProgress {
        self.on_uploading_progress
    }

    /// 修改上传进度回调
    #[inline]
    pub fn on_uploading_progress_mut(&mut self) -> &mut OnProgress<'r> {
        &mut self.on_uploading_progress
    }

    /// 下载进度回调
    #[inline]
    pub fn on_downloading_progress(&self) -> OnProgress {
        self.on_downloading_progress
    }

    /// 修改下载进度回调
    #[inline]
    pub fn on_downloading_progress_mut(&mut self) -> &mut OnProgress<'r> {
        &mut self.on_downloading_progress
    }

    /// 发送请求体回调
    #[inline]
    pub fn on_send_request_body(&self) -> OnBody {
        self.on_send_request_body
    }

    /// 修改发送请求体回调
    #[inline]
    pub fn on_send_request_body_mut(&mut self) -> &mut OnBody<'r> {
        &mut self.on_send_request_body
    }

    /// 接受到响应状态回调
    #[inline]
    pub fn on_receive_response_status(&self) -> OnStatusCode {
        self.on_receive_response_status
    }

    /// 修改接受到响应状态回调
    #[inline]
    pub fn on_receive_response_status_mut(&mut self) -> &mut OnStatusCode<'r> {
        &mut self.on_receive_response_status
    }

    /// 接受到响应体回调
    #[inline]
    pub fn on_receive_response_body(&self) -> OnBody {
        self.on_receive_response_body
    }

    /// 修改接受到响应体回调
    #[inline]
    pub fn on_receive_response_body_mut(&mut self) -> &mut OnBody<'r> {
        &mut self.on_receive_response_body
    }

    /// 接受到响应 Header 回调
    #[inline]
    pub fn on_receive_response_header(&self) -> OnHeader {
        self.on_receive_response_header
    }

    /// 修改接受到响应 Header 回调
    #[inline]
    pub fn on_receive_response_header_mut(&mut self) -> &mut OnHeader<'r> {
        &mut self.on_receive_response_header
    }

    /// 连接超时时长
    #[inline]
    pub fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    /// 修改连接超时时长
    #[inline]
    pub fn connect_timeout_mut(&mut self) -> &mut Duration {
        &mut self.connect_timeout
    }

    /// 请求超时时长
    #[inline]
    pub fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    /// 修改请求超时时长
    #[inline]
    pub fn request_timeout_mut(&mut self) -> &mut Duration {
        &mut self.request_timeout
    }

    /// TCP KeepAlive 空闲时长
    #[inline]
    pub fn tcp_keepalive_idle_timeout(&self) -> Duration {
        self.tcp_keepalive_idle_timeout
    }

    /// 修改 TCP KeepAlive 空闲时长
    #[inline]
    pub fn tcp_keepalive_idle_timeout_mut(&mut self) -> &mut Duration {
        &mut self.tcp_keepalive_idle_timeout
    }

    /// TCP KeepAlive 探测包的发送间隔
    #[inline]
    pub fn tcp_keepalive_probe_interval(&self) -> Duration {
        self.tcp_keepalive_probe_interval
    }

    /// 修改 TCP KeepAlive 探测包的发送间隔
    #[inline]
    pub fn tcp_keepalive_probe_interval_mut(&mut self) -> &mut Duration {
        &mut self.tcp_keepalive_probe_interval
    }

    /// 最低传输速度和维持时长
    #[inline]
    pub fn low_transfer_speed(&self) -> (u32, Duration) {
        (self.low_transfer_speed, self.low_transfer_speed_timeout)
    }

    /// 修改最低传输速度和维持时长
    #[inline]
    pub fn low_transfer_speed_mut(&mut self) -> (&mut u32, &mut Duration) {
        (
            &mut self.low_transfer_speed,
            &mut self.low_transfer_speed_timeout,
        )
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl Default for Request<'_> {
    fn default() -> Self {
        Self {
            url: "http://localhost".into(),
            method: Method::GET,
            headers: Default::default(),
            body: Default::default(),
            response_body_buffer_path: Default::default(),
            appended_user_agent: Default::default(),
            follow_redirection: false,
            resolved_ip_addrs: Default::default(),
            on_uploading_progress: None,
            on_downloading_progress: None,
            on_send_request_body: None,
            on_receive_response_status: None,
            on_receive_response_body: None,
            on_receive_response_header: None,
            connect_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(300),
            tcp_keepalive_idle_timeout: Duration::from_secs(300),
            tcp_keepalive_probe_interval: Duration::from_secs(5),
            low_transfer_speed: Default::default(),
            low_transfer_speed_timeout: Default::default(),
        }
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
        field!(s, "url", url);
        field!(s, "method", method);
        field!(s, "headers", headers);
        field!(s, "body", body);
        field!(s, "appended_user_agent", appended_user_agent);
        field!(s, "follow_redirection", follow_redirection);
        field!(s, "resolved_ip_addrs", resolved_ip_addrs);
        field!(s, "connect_timeout", connect_timeout);
        field!(s, "request_timeout", request_timeout);
        field!(s, "tcp_keepalive_idle_timeout", tcp_keepalive_idle_timeout);
        field!(
            s,
            "tcp_keepalive_probe_interval",
            tcp_keepalive_probe_interval
        );
        field!(s, "low_transfer_speed", low_transfer_speed);
        field!(s, "low_transfer_speed_timeout", low_transfer_speed_timeout);
        closure_field!(s, "on_uploading_progress", on_uploading_progress);
        closure_field!(s, "on_downloading_progress", on_downloading_progress);
        closure_field!(s, "on_send_request_body", on_send_request_body);
        closure_field!(s, "on_receive_response_status", on_receive_response_status);
        closure_field!(s, "on_receive_response_body", on_receive_response_body);
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
    /// 设置请求 URL
    #[inline]
    pub fn url(&mut self, url: impl Into<URL<'r>>) -> &mut Self {
        self.inner.url = url.into();
        self
    }

    /// 设置请求 HTTP 方法
    #[inline]
    pub fn method(&mut self, method: Method) -> &mut Self {
        self.inner.method = method;
        self
    }

    /// 设置请求 HTTP Headers
    #[inline]
    pub fn headers(&mut self, headers: Headers<'r>) -> &mut Self {
        self.inner.headers = headers;
        self
    }

    /// 设置请求体
    #[inline]
    pub fn body(&mut self, body: impl Into<Body<'r>>) -> &mut Self {
        self.inner.body = body.into();
        self
    }

    /// 请求体
    #[inline]
    pub fn response_body_buffer_path(&mut self, path: impl Into<Cow<'r, Path>>) -> &mut Self {
        self.inner.response_body_buffer_path = Some(path.into());
        self
    }

    /// 设置用户代理
    #[inline]
    pub fn appended_user_agent(&mut self, user_agent: impl Into<Cow<'r, str>>) -> &mut Self {
        self.inner.appended_user_agent = user_agent.into();
        self
    }

    /// 设置是否自动跟踪重定向
    #[inline]
    pub fn follow_redirection(&mut self, follow_redirection: bool) -> &mut Self {
        self.inner.follow_redirection = follow_redirection;
        self
    }

    /// 设置预解析的服务器套接字地址
    #[inline]
    pub fn resolved_ip_addrs(
        &mut self,
        resolved_ip_addrs: impl Into<Cow<'r, [IpAddr]>>,
    ) -> &mut Self {
        self.inner.resolved_ip_addrs = resolved_ip_addrs.into();
        self
    }

    /// 设置上传进度回调
    #[inline]
    pub fn on_uploading_progress(&mut self, f: OnProgress<'r>) -> &mut Self {
        self.inner.on_uploading_progress = f;
        self
    }

    /// 设置下载进度回调
    #[inline]
    pub fn on_downloading_progress(&mut self, f: OnProgress<'r>) -> &mut Self {
        self.inner.on_downloading_progress = f;
        self
    }

    /// 发送请求体回调
    #[inline]
    pub fn on_send_request_body(&mut self, f: OnBody<'r>) -> &mut Self {
        self.inner.on_send_request_body = f;
        self
    }

    /// 接受到响应状态回调
    #[inline]
    pub fn on_receive_response_status(&mut self, f: OnStatusCode<'r>) -> &mut Self {
        self.inner.on_receive_response_status = f;
        self
    }

    /// 接受到响应体回调
    #[inline]
    pub fn on_receive_response_body(&mut self, f: OnBody<'r>) -> &mut Self {
        self.inner.on_receive_response_body = f;
        self
    }

    /// 接受到响应 Header 回调
    #[inline]
    pub fn on_receive_response_header(&mut self, f: OnHeader<'r>) -> &mut Self {
        self.inner.on_receive_response_header = f;
        self
    }

    /// 设置连接超时时长
    #[inline]
    pub fn connect_timeout(&mut self, connect_timeout: Duration) -> &mut Self {
        self.inner.connect_timeout = connect_timeout;
        self
    }

    /// 设置请求超时时长
    #[inline]
    pub fn request_timeout(&mut self, request_timeout: Duration) -> &mut Self {
        self.inner.request_timeout = request_timeout;
        self
    }

    /// 设置 TCP KeepAlive 空闲时长
    #[inline]
    pub fn tcp_keepalive_idle_timeout(
        &mut self,
        tcp_keepalive_idle_timeout: Duration,
    ) -> &mut Self {
        self.inner.tcp_keepalive_idle_timeout = tcp_keepalive_idle_timeout;
        self
    }

    /// 设置 TCP KeepAlive 探测包的发送间隔
    #[inline]
    pub fn tcp_keepalive_probe_interval(
        &mut self,
        tcp_keepalive_probe_interval: Duration,
    ) -> &mut Self {
        self.inner.tcp_keepalive_probe_interval = tcp_keepalive_probe_interval;
        self
    }

    /// 设置最低传输速度和维持时长
    ///
    /// 当 HTTP 传输速度低于最低传输速度 `low_transfer_speed_timeout` 并维持超过 `low_transfer_speed` 的时长，则出错。
    /// SDK 应该重试，或出错退出
    #[inline]
    pub fn low_transfer_speed(&mut self, low_transfer_speed: u32, timeout: Duration) -> &mut Self {
        self.inner.low_transfer_speed = low_transfer_speed;
        self.inner.low_transfer_speed_timeout = timeout;
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
