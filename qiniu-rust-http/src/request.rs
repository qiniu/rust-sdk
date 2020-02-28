use super::{HeaderName, HeaderValue, Headers, Method};
use getset::{CopyGetters, Getters, MutGetters};
use std::{borrow::Cow, ffi::c_void, fmt, net::SocketAddr, ptr::null_mut, time::Duration};

/// 请求 URL
pub type URL<'b> = Cow<'b, str>;

/// 请求体
pub type Body<'b> = Cow<'b, [u8]>;

/// 进度回调闭包
#[derive(Copy, Clone)]
pub struct ProgressCallback<'b>(_ProgressCallback<'b>);

#[derive(Copy, Clone)]
enum _ProgressCallback<'b> {
    Closure(&'b dyn Fn(u64, u64)),
    Fn(fn(u64, u64)),
}

/// HTTP 请求
///
/// 封装 HTTP 请求相关字段
#[derive(Getters, CopyGetters, MutGetters)]
pub struct Request<'b> {
    /// 请求 URL
    #[get_mut = "pub"]
    url: URL<'b>,

    /// 请求 HTTP 方法
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    method: Method,

    /// 请求 HTTP Headers
    #[get = "pub"]
    #[get_mut = "pub"]
    headers: Headers<'b>,

    /// 请求体
    #[get = "pub"]
    #[get_mut = "pub"]
    body: Option<Body<'b>>,

    /// 用户代理
    #[get_mut = "pub"]
    user_agent: Option<Cow<'b, str>>,

    /// 是否自动跟踪重定向
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    follow_redirection: bool,

    /// 预解析的服务器套接字地址
    #[get = "pub"]
    #[get_mut = "pub"]
    resolved_socket_addrs: Cow<'b, [SocketAddr]>,

    /// 自定义数据指针
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    custom_data: *mut c_void,

    /// 上传进度回调闭包
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    on_uploading_progress: Option<ProgressCallback<'b>>,

    /// 下载进度回调闭包
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    on_downloading_progress: Option<ProgressCallback<'b>>,

    /// 连接超时时长
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    connect_timeout: Duration,

    /// 请求超时时长
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    request_timeout: Duration,

    /// TCP KeepAlive 空闲时长
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    tcp_keepalive_idle_timeout: Duration,

    /// TCP KeepAlive 探测包的发送间隔
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    tcp_keepalive_probe_interval: Duration,

    /// 最低传输速度
    ///
    /// 与 `low_transfer_speed_timeout` 配合使用。
    /// 当 HTTP 传输速度低于最低传输速度 `low_transfer_speed_timeout` 并维持超过 `low_transfer_speed` 的时长，则出错。
    /// SDK 应该重试，或出错退出
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    low_transfer_speed: u32,

    /// 最低传输速度维持时长
    #[get_copy = "pub"]
    #[get_mut = "pub"]
    low_transfer_speed_timeout: Duration,
}

impl<'b> Request<'b> {
    /// 请求 URL
    pub fn url(&self) -> &str {
        self.url.as_ref()
    }

    /// 用户代理
    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_ref().map(|user_agent| user_agent.as_ref())
    }
}

/// HTTP 请求生成器
pub struct RequestBuilder<'r> {
    request: Request<'r>,
}

impl<'r> RequestBuilder<'r> {
    /// 创建默认 HTTP 请求
    pub fn default() -> RequestBuilder<'r> {
        RequestBuilder {
            request: Default::default(),
        }
    }

    /// 设置 HTTP 方法
    pub fn method(mut self, method: impl Into<Method>) -> RequestBuilder<'r> {
        self.request.method = method.into();
        self
    }

    /// 设置请求 URL
    pub fn url(mut self, url: impl Into<URL<'r>>) -> RequestBuilder<'r> {
        self.request.url = url.into();
        self
    }

    /// 设置请求 Header
    pub fn header(
        mut self,
        header_name: impl Into<HeaderName<'r>>,
        header_value: impl Into<HeaderValue<'r>>,
    ) -> RequestBuilder<'r> {
        self.request.headers.insert(header_name.into(), header_value.into());
        self
    }

    /// 设置请求 Headers
    pub fn headers(mut self, headers: Headers<'r>) -> RequestBuilder<'r> {
        self.request.headers = headers;
        self
    }

    /// 设置请求体
    pub fn body(mut self, body: impl Into<Body<'r>>) -> RequestBuilder<'r> {
        self.request.body = Some(body.into());
        self
    }

    /// 设置用户代理
    pub fn user_agent(mut self, user_agent: impl Into<Cow<'r, str>>) -> RequestBuilder<'r> {
        self.request.user_agent = Some(user_agent.into());
        self
    }

    /// 设置是否自动追踪重定向
    pub fn follow_redirection(mut self, follow_redirection: bool) -> RequestBuilder<'r> {
        self.request.follow_redirection = follow_redirection;
        self
    }

    /// 设置预解析服务器套接字地址
    pub fn resolved_socket_addrs(mut self, socket_addrs: impl Into<Cow<'r, [SocketAddr]>>) -> RequestBuilder<'r> {
        self.request.resolved_socket_addrs = socket_addrs.into();
        self
    }

    /// 设置上传进度回调
    pub fn on_uploading_progress(mut self, callback: impl Into<ProgressCallback<'r>>) -> RequestBuilder<'r> {
        self.request.on_uploading_progress = Some(callback.into());
        self
    }

    /// 设置下载进度回调
    pub fn on_downloading_progress(mut self, callback: impl Into<ProgressCallback<'r>>) -> RequestBuilder<'r> {
        self.request.on_downloading_progress = Some(callback.into());
        self
    }

    /// 设置连接超时时长
    pub fn connect_timeout(mut self, timeout: Duration) -> RequestBuilder<'r> {
        self.request.connect_timeout = timeout;
        self
    }

    /// 设置请求超时时长
    pub fn request_timeout(mut self, timeout: Duration) -> RequestBuilder<'r> {
        self.request.request_timeout = timeout;
        self
    }

    /// 设置 TCP KeepAlive 空闲时长
    pub fn tcp_keepalive_idle_timeout(mut self, timeout: Duration) -> RequestBuilder<'r> {
        self.request.tcp_keepalive_idle_timeout = timeout;
        self
    }

    /// 设置 TCP KeepAlive 探测包的发送间隔
    pub fn tcp_keepalive_probe_interval(mut self, timeout: Duration) -> RequestBuilder<'r> {
        self.request.tcp_keepalive_probe_interval = timeout;
        self
    }

    /// 设置最低传输速度
    pub fn low_transfer_speed(mut self, speed: u32) -> RequestBuilder<'r> {
        self.request.low_transfer_speed = speed;
        self
    }

    /// 设置最低传输速度维持时长
    pub fn low_transfer_speed_timeout(mut self, timeout: Duration) -> RequestBuilder<'r> {
        self.request.low_transfer_speed_timeout = timeout;
        self
    }

    /// 生成 HTTP 请求
    pub fn build(self) -> Request<'r> {
        self.request
    }
}

impl Default for Request<'_> {
    fn default() -> Self {
        Request {
            url: "http://localhost".into(),
            method: Method::GET,
            headers: Headers::new(),
            body: None,
            user_agent: None,
            follow_redirection: false,
            resolved_socket_addrs: Cow::Borrowed(&[]),
            on_uploading_progress: None,
            on_downloading_progress: None,
            custom_data: null_mut(),
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(300),
            tcp_keepalive_idle_timeout: Duration::from_secs(300),
            tcp_keepalive_probe_interval: Duration::from_secs(5),
            low_transfer_speed: 1024,
            low_transfer_speed_timeout: Duration::from_secs(30),
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
            .field(
                "on_uploading_progress",
                if self.on_uploading_progress.is_some() {
                    &"Installed"
                } else {
                    &"Not Installed"
                },
            )
            .field(
                "on_downloading_progress",
                if self.on_downloading_progress.is_some() {
                    &"Installed"
                } else {
                    &"Not Installed"
                },
            )
            .field("connect_timeout", &self.connect_timeout)
            .field("request_timeout", &self.request_timeout)
            .field("tcp_keepalive_idle_timeout", &self.tcp_keepalive_idle_timeout)
            .field("tcp_keepalive_probe_interval", &self.tcp_keepalive_probe_interval)
            .field("low_transfer_speed", &self.low_transfer_speed)
            .field("low_transfer_speed_timeout", &self.low_transfer_speed_timeout)
            .finish()
    }
}

impl ProgressCallback<'_> {
    // 调用进度回调闭包
    pub fn call(&self, uploaded: u64, total: u64) {
        match self.0 {
            _ProgressCallback::Closure(closure) => (closure)(uploaded, total),
            _ProgressCallback::Fn(f) => (f)(uploaded, total),
        }
    }
}

impl<'a> From<&'a dyn Fn(u64, u64)> for ProgressCallback<'a> {
    fn from(f: &'a dyn Fn(u64, u64)) -> Self {
        Self(_ProgressCallback::Closure(f))
    }
}

impl From<fn(u64, u64)> for ProgressCallback<'_> {
    fn from(f: fn(u64, u64)) -> Self {
        Self(_ProgressCallback::Fn(f))
    }
}
