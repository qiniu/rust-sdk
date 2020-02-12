use super::{HeaderName, HeaderValue, Headers, Method};
use getset::{CopyGetters, Getters, MutGetters};
use std::{borrow::Cow, ffi::c_void, fmt, net::SocketAddr, ptr::null_mut, time::Duration};

pub type URL<'b> = Cow<'b, str>;
pub type Body<'b> = Cow<'b, [u8]>;

#[derive(Copy, Clone)]
pub enum ProgressCallback<'b> {
    Closure(&'b dyn Fn(u64, u64)),
    Fn(fn(u64, u64)),
}

#[derive(Getters, CopyGetters, MutGetters)]
pub struct Request<'b> {
    #[get_mut = "pub"]
    url: URL<'b>,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    method: Method,

    #[get = "pub"]
    #[get_mut = "pub"]
    headers: Headers<'b>,

    #[get = "pub"]
    #[get_mut = "pub"]
    body: Option<Body<'b>>,

    #[get_mut = "pub"]
    user_agent: Option<Cow<'b, str>>,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    follow_redirection: bool,

    #[get = "pub"]
    #[get_mut = "pub"]
    resolved_socket_addrs: Cow<'b, [SocketAddr]>,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    custom_data: *mut c_void,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    on_uploading_progress: Option<ProgressCallback<'b>>,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    on_downloading_progress: Option<ProgressCallback<'b>>,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    connect_timeout: Duration,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    request_timeout: Duration,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    tcp_keepalive_idle_timeout: Duration,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    tcp_keepalive_probe_interval: Duration,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    low_transfer_speed: u32,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    low_transfer_speed_timeout: Duration,
}

impl<'b> Request<'b> {
    pub fn url(&self) -> &str {
        self.url.as_ref()
    }

    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_ref().map(|user_agent| user_agent.as_ref())
    }
}

pub struct RequestBuilder<'r> {
    request: Request<'r>,
}

impl<'r> RequestBuilder<'r> {
    pub fn default() -> RequestBuilder<'r> {
        RequestBuilder {
            request: Default::default(),
        }
    }

    pub fn method(mut self, method: impl Into<Method>) -> RequestBuilder<'r> {
        self.request.method = method.into();
        self
    }

    pub fn url(mut self, url: impl Into<URL<'r>>) -> RequestBuilder<'r> {
        self.request.url = url.into();
        self
    }

    pub fn header(
        mut self,
        header_name: impl Into<HeaderName<'r>>,
        header_value: impl Into<HeaderValue<'r>>,
    ) -> RequestBuilder<'r> {
        self.request.headers.insert(header_name.into(), header_value.into());
        self
    }

    pub fn headers(mut self, headers: Headers<'r>) -> RequestBuilder<'r> {
        self.request.headers = headers;
        self
    }

    pub fn body(mut self, body: impl Into<Body<'r>>) -> RequestBuilder<'r> {
        self.request.body = Some(body.into());
        self
    }

    pub fn user_agent(mut self, user_agent: impl Into<Cow<'r, str>>) -> RequestBuilder<'r> {
        self.request.user_agent = Some(user_agent.into());
        self
    }

    pub fn follow_redirection(mut self, follow_redirection: bool) -> RequestBuilder<'r> {
        self.request.follow_redirection = follow_redirection;
        self
    }

    pub fn resolved_socket_addrs(mut self, socket_addrs: impl Into<Cow<'r, [SocketAddr]>>) -> RequestBuilder<'r> {
        self.request.resolved_socket_addrs = socket_addrs.into();
        self
    }

    pub fn on_uploading_progress(mut self, callback: impl Into<ProgressCallback<'r>>) -> RequestBuilder<'r> {
        self.request.on_uploading_progress = Some(callback.into());
        self
    }

    pub fn on_downloading_progress(mut self, callback: impl Into<ProgressCallback<'r>>) -> RequestBuilder<'r> {
        self.request.on_downloading_progress = Some(callback.into());
        self
    }

    pub fn connect_timeout(mut self, timeout: Duration) -> RequestBuilder<'r> {
        self.request.connect_timeout = timeout;
        self
    }

    pub fn request_timeout(mut self, timeout: Duration) -> RequestBuilder<'r> {
        self.request.request_timeout = timeout;
        self
    }

    pub fn tcp_keepalive_idle_timeout(mut self, timeout: Duration) -> RequestBuilder<'r> {
        self.request.tcp_keepalive_idle_timeout = timeout;
        self
    }

    pub fn tcp_keepalive_probe_interval(mut self, timeout: Duration) -> RequestBuilder<'r> {
        self.request.tcp_keepalive_probe_interval = timeout;
        self
    }

    pub fn low_transfer_speed(mut self, speed: u32) -> RequestBuilder<'r> {
        self.request.low_transfer_speed = speed;
        self
    }

    pub fn low_transfer_speed_timeout(mut self, timeout: Duration) -> RequestBuilder<'r> {
        self.request.low_transfer_speed_timeout = timeout;
        self
    }

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
    pub fn call(&self, uploaded: u64, total: u64) {
        match self {
            ProgressCallback::Closure(closure) => (closure)(uploaded, total),
            ProgressCallback::Fn(f) => (f)(uploaded, total),
        }
    }
}

impl<'a> From<&'a dyn Fn(u64, u64)> for ProgressCallback<'a> {
    fn from(f: &'a dyn Fn(u64, u64)) -> Self {
        ProgressCallback::Closure(f)
    }
}

impl From<fn(u64, u64)> for ProgressCallback<'_> {
    fn from(f: fn(u64, u64)) -> Self {
        ProgressCallback::Fn(f)
    }
}
