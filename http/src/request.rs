use super::{Headers, Method};
use once_cell::sync::Lazy;
use std::{borrow::Cow, ffi::c_void, fmt, net::SocketAddr, ptr::null_mut, time::Duration};

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

/// HTTP 请求
///
/// 封装 HTTP 请求相关字段
#[derive(Clone)]
pub struct Request<'r> {
    url: URL<'r>,
    method: Method,
    headers: Headers<'r>,
    body: Body<'r>,

    // 请求配置属性
    user_agent: Cow<'r, str>,
    follow_redirection: bool,
    resolved_socket_addrs: Cow<'r, [SocketAddr]>,
    custom_data: *mut c_void,
    on_uploading_progress: Option<&'r dyn Fn(u64, u64, *mut c_void)>,
    on_downloading_progress: Option<&'r dyn Fn(u64, u64, *mut c_void)>,
    on_send_request_body: Option<&'r dyn Fn(&mut Request<'_>)>,
    on_receive_response_body: Option<&'r dyn Fn(&mut Request<'_>)>,
    on_receive_response_header: Option<&'r dyn Fn(&mut Request<'_>, &mut str, &mut str)>,
    connect_timeout: Duration,
    request_timeout: Duration,
    tcp_keepalive_idle_timeout: Duration,
    tcp_keepalive_probe_interval: Duration,
    low_transfer_speed: u32,
    low_transfer_speed_timeout: Duration,
}

impl<'r> Request<'r> {
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

    /// 用户代理
    #[inline]
    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    /// 修改用户代理
    #[inline]
    pub fn user_agent_mut(&mut self) -> &mut Cow<'r, str> {
        &mut self.user_agent
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
    pub fn resolved_socket_addrs(&self) -> &[SocketAddr] {
        &self.resolved_socket_addrs
    }

    /// 修改预解析的服务器套接字地址
    #[inline]
    pub fn resolved_socket_addrs_mut(&mut self) -> &mut Cow<'r, [SocketAddr]> {
        &mut self.resolved_socket_addrs
    }

    /// 自定义数据指针
    #[inline]
    pub fn custom_data(&self) -> *mut c_void {
        self.custom_data
    }

    /// 修改自定义数据指针
    #[inline]
    pub fn custom_data_mut(&mut self) -> &mut *mut c_void {
        &mut self.custom_data
    }

    /// 上传进度回调
    #[inline]
    pub fn on_uploading_progress(&self) -> Option<&dyn Fn(u64, u64, *mut c_void)> {
        self.on_uploading_progress
    }

    /// 修改上传进度回调
    #[inline]
    pub fn on_uploading_progress_mut(&mut self) -> &mut Option<&'r dyn Fn(u64, u64, *mut c_void)> {
        &mut self.on_uploading_progress
    }

    /// 下载进度回调
    #[inline]
    pub fn on_downloading_progress(&self) -> Option<&dyn Fn(u64, u64, *mut c_void)> {
        self.on_downloading_progress
    }

    /// 修改下载进度回调
    #[inline]
    pub fn on_downloading_progress_mut(
        &mut self,
    ) -> &mut Option<&'r dyn Fn(u64, u64, *mut c_void)> {
        &mut self.on_downloading_progress
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
}

impl Default for Request<'_> {
    fn default() -> Self {
        Self {
            url: "http://localhost".into(),
            method: Method::GET,
            headers: Default::default(),
            body: Default::default(),
            user_agent: Cow::Borrowed(&FULL_USER_AGENT),
            follow_redirection: false,
            resolved_socket_addrs: Default::default(),
            custom_data: null_mut(),
            on_uploading_progress: None,
            on_downloading_progress: None,
            on_send_request_body: None,
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
        f.debug_struct("Request")
            .field("url", &self.url)
            .field("method", &self.method)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .field("user_agent", &self.user_agent)
            .field("follow_redirection", &self.follow_redirection)
            .field("resolved_socket_addrs", &self.resolved_socket_addrs)
            .field("custom_data", &self.custom_data)
            .field("connect_timeout", &self.connect_timeout)
            .field("request_timeout", &self.request_timeout)
            .field(
                "on_uploading_progress",
                &self.on_uploading_progress.map_or_else(
                    || Cow::Borrowed("Uninstalled"),
                    |_| Cow::Borrowed("Installed"),
                ),
            )
            .field(
                "on_downloading_progress",
                &self.on_downloading_progress.map_or_else(
                    || Cow::Borrowed("Uninstalled"),
                    |_| Cow::Borrowed("Installed"),
                ),
            )
            .field(
                "tcp_keepalive_idle_timeout",
                &self.tcp_keepalive_idle_timeout,
            )
            .field(
                "tcp_keepalive_probe_interval",
                &self.tcp_keepalive_probe_interval,
            )
            .field("low_transfer_speed", &self.low_transfer_speed)
            .field(
                "low_transfer_speed_timeout",
                &self.low_transfer_speed_timeout,
            )
            .finish()
    }
}

/// HTTP 请求生成器
#[derive(Default, Debug, Clone)]
pub struct RequestBuilder<'r> {
    inner: Request<'r>,
}

impl<'r> RequestBuilder<'r> {
    /// 设置请求 URL
    #[inline]
    pub fn url(&mut self, url: URL<'r>) -> &mut Self {
        self.inner.url = url;
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
    pub fn body(&mut self, body: Body<'r>) -> &mut Self {
        self.inner.body = body;
        self
    }

    /// 设置用户代理
    #[inline]
    pub fn user_agent(&mut self, user_agent: impl Into<Cow<'r, str>>) -> &mut Self {
        self.inner.user_agent = user_agent.into();
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
    pub fn resolved_socket_addrs(
        &mut self,
        resolved_socket_addrs: impl Into<Cow<'r, [SocketAddr]>>,
    ) -> &mut Self {
        self.inner.resolved_socket_addrs = resolved_socket_addrs.into();
        self
    }

    /// 设置自定义数据指针
    #[inline]
    pub fn custom_data(&mut self, custom_data: *mut c_void) -> &mut Self {
        self.inner.custom_data = custom_data;
        self
    }

    /// 设置上传进度回调
    #[inline]
    pub fn on_uploading_progress(
        &mut self,
        on_uploading_progress: Option<&'r dyn Fn(u64, u64, *mut c_void)>,
    ) -> &mut Self {
        self.inner.on_uploading_progress = on_uploading_progress;
        self
    }

    /// 设置下载进度回调
    #[inline]
    pub fn on_downloading_progress(
        &mut self,
        on_downloading_progress: Option<&'r dyn Fn(u64, u64, *mut c_void)>,
    ) -> &mut Self {
        self.inner.on_downloading_progress = on_downloading_progress;
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

    /// 构建 HTTP 请求
    #[inline]
    pub fn build(&self) -> Request<'r> {
        self.inner.clone()
    }

    /// 重置构建器
    #[inline]
    pub fn reset(&mut self) {
        self.inner = Default::default();
    }
}
