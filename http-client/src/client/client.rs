use super::{
    callbacks::{OnBody, OnError, OnHeader, OnProgress, OnRequest, OnRetry, OnStatusCode},
    CachedResolver, Callbacks, CallbacksBuilder, Chooser, DefaultRetrier, RequestRetrier,
    SimpleChooser, SimpleResolver,
};
use qiniu_http::HTTPCaller;
use std::{sync::Arc, time::Duration};

pub struct Client {
    use_https: bool,
    appended_user_agent: Box<str>,
    http_caller: Arc<dyn HTTPCaller>,
    request_retrier: Arc<dyn RequestRetrier>,
    chooser: Arc<dyn Chooser>,
    callbacks: Callbacks,
    connect_timeout: Option<Duration>,
    request_timeout: Option<Duration>,
}

#[cfg(any(feature = "curl"))]
impl Default for Client {
    #[inline]
    fn default() -> Self {
        ClientBuilder::new().build()
    }
}

impl Client {
    #[inline]
    #[cfg(not(any(feature = "curl")))]
    pub fn new(http_caller: Arc<dyn HTTPCaller>) -> Self {
        ClientBuilder::new(http_caller).build()
    }

    #[inline]
    pub(super) fn use_https(&self) -> bool {
        self.use_https
    }

    #[inline]
    pub(super) fn appended_user_agent(&self) -> &str {
        &self.appended_user_agent
    }

    #[inline]
    pub(super) fn connect_timeout(&self) -> Option<Duration> {
        self.connect_timeout
    }

    #[inline]
    pub(super) fn request_timeout(&self) -> Option<Duration> {
        self.request_timeout
    }

    #[inline]
    pub(super) fn callbacks(&self) -> &Callbacks {
        &self.callbacks
    }
}

pub struct ClientBuilder {
    use_https: bool,
    appended_user_agent: Box<str>,
    http_caller: Arc<dyn HTTPCaller>, // TODO: 默认值与 是否启用 curl 相关
    request_retrier: Arc<dyn RequestRetrier>,
    chooser: Arc<dyn Chooser>,
    callbacks: CallbacksBuilder,
    connect_timeout: Option<Duration>,
    request_timeout: Option<Duration>,
}

#[cfg(feature = "curl")]
impl Default for ClientBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ClientBuilder {
    #[inline]
    #[cfg(feature = "curl")]
    pub fn new() -> Self {
        use qiniu_curl::CurlHTTPCaller;
        Self::_new(Arc::new(CurlHTTPCaller::default()))
    }

    #[inline]
    #[cfg(not(any(feature = "curl")))]
    pub fn new(http_caller: Arc<dyn HTTPCaller>) -> Self {
        Self::_new(http_caller)
    }

    #[inline]
    fn _new(http_caller: Arc<dyn HTTPCaller>) -> Self {
        ClientBuilder {
            http_caller,
            use_https: true,
            appended_user_agent: Default::default(),
            request_retrier: Arc::new(DefaultRetrier::default()),
            chooser: Arc::new(SimpleChooser::<CachedResolver<SimpleResolver>>::default()),
            callbacks: Default::default(),
            connect_timeout: Default::default(),
            request_timeout: Default::default(),
        }
    }

    #[inline]
    pub fn use_https(mut self, use_https: bool) -> Self {
        self.use_https = use_https;
        self
    }

    #[inline]
    pub fn appended_user_agent(mut self, appended_user_agent: impl Into<Box<str>>) -> Self {
        self.appended_user_agent = appended_user_agent.into();
        self
    }

    #[inline]
    pub fn http_caller(mut self, http_caller: impl Into<Arc<dyn HTTPCaller>>) -> Self {
        self.http_caller = http_caller.into();
        self
    }

    #[inline]
    pub fn request_retrier(mut self, request_retrier: impl Into<Arc<dyn RequestRetrier>>) -> Self {
        self.request_retrier = request_retrier.into();
        self
    }

    #[inline]
    pub fn chooser(mut self, chooser: impl Into<Arc<dyn Chooser>>) -> Self {
        self.chooser = chooser.into();
        self
    }

    #[inline]
    pub fn on_uploading_progress(mut self, callback: impl Into<OnProgress>) -> Self {
        self.callbacks = self.callbacks.on_uploading_progress(callback);
        self
    }

    #[inline]
    pub fn on_downloading_progress(mut self, callback: impl Into<OnProgress>) -> Self {
        self.callbacks = self.callbacks.on_downloading_progress(callback);
        self
    }

    #[inline]
    pub fn on_request(mut self, callback: impl Into<OnRequest>) -> Self {
        self.callbacks = self.callbacks.on_request(callback);
        self
    }

    #[inline]
    pub fn on_send_request_body(mut self, callback: impl Into<OnBody>) -> Self {
        self.callbacks = self.callbacks.on_send_request_body(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_status(mut self, callback: impl Into<OnStatusCode>) -> Self {
        self.callbacks = self.callbacks.on_receive_response_status(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_body(mut self, callback: impl Into<OnBody>) -> Self {
        self.callbacks = self.callbacks.on_receive_response_body(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_header(mut self, callback: impl Into<OnHeader>) -> Self {
        self.callbacks = self.callbacks.on_receive_response_header(callback);
        self
    }

    #[inline]
    pub fn on_error(mut self, callback: impl Into<OnError>) -> Self {
        self.callbacks = self.callbacks.on_error(callback);
        self
    }

    #[inline]
    pub fn on_retry(mut self, callback: impl Into<OnRetry>) -> Self {
        self.callbacks = self.callbacks.on_retry(callback);
        self
    }

    #[inline]
    pub fn connect_timeout(mut self, connect_timeout: Duration) -> Self {
        self.connect_timeout = Some(connect_timeout);
        self
    }

    #[inline]
    pub fn request_timeout(mut self, request_timeout: Duration) -> Self {
        self.request_timeout = Some(request_timeout);
        self
    }

    #[inline]
    pub fn build(self) -> Client {
        Client {
            use_https: self.use_https,
            appended_user_agent: self.appended_user_agent,
            http_caller: self.http_caller,
            request_retrier: self.request_retrier,
            chooser: self.chooser,
            callbacks: self.callbacks.build(),
            connect_timeout: self.connect_timeout,
            request_timeout: self.request_timeout,
        }
    }
}
