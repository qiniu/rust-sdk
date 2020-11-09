use super::{
    super::{IntoEndpoints, ServiceName},
    callbacks::{
        OnBody, OnDomainChosen, OnError, OnHeader, OnProgress, OnRequest, OnRetry, OnStatusCode,
        OnToChooseDomain,
    },
    CachedResolver, Callbacks, CallbacksBuilder, Chooser, DefaultRetrier,
    ExponentialRetryDelayPolicy, RandomizedRetryDelayPolicy, RequestBuilder, RequestRetrier,
    RetryDelayPolicy, ShuffledChooser, ShuffledResolver, SimpleChooser, SimpleResolver,
};
use qiniu_http::{HTTPCaller, Method};
use std::{sync::Arc, time::Duration};

#[derive(Debug, Clone)]
pub struct HTTPClient {
    inner: Arc<HTTPClientInner>,
}

#[derive(Debug)]
struct HTTPClientInner {
    use_https: bool,
    appended_user_agent: Box<str>,
    http_caller: Arc<dyn HTTPCaller>,
    request_retrier: Arc<dyn RequestRetrier>,
    retry_delay_policy: Arc<dyn RetryDelayPolicy>,
    chooser: Arc<dyn Chooser>,
    callbacks: Callbacks,
    connect_timeout: Option<Duration>,
    request_timeout: Option<Duration>,
}

#[cfg(any(feature = "curl"))]
impl Default for HTTPClient {
    #[inline]
    fn default() -> Self {
        HTTPClientBuilder::new().build()
    }
}

impl HTTPClient {
    #[inline]
    #[cfg(any(feature = "curl"))]
    pub fn builder() -> HTTPClientBuilder {
        HTTPClientBuilder::new()
    }

    #[inline]
    #[cfg(not(any(feature = "curl")))]
    pub fn new(http_caller: Arc<dyn HTTPCaller>) -> Self {
        HTTPClientBuilder::new(http_caller).build()
    }

    #[inline]
    #[cfg(not(any(feature = "curl")))]
    pub fn builder(http_caller: Arc<dyn HTTPCaller>) -> HTTPClientBuilder {
        HTTPClientBuilder::new(http_caller)
    }

    pub fn get<'r>(
        &'r self,
        service_name: ServiceName,
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> RequestBuilder<'r> {
        self.new_request(Method::GET, service_name, into_endpoints.into())
    }

    pub fn head<'r>(
        &'r self,
        service_name: ServiceName,
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> RequestBuilder<'r> {
        self.new_request(Method::HEAD, service_name, into_endpoints.into())
    }

    pub fn post<'r>(
        &'r self,
        service_name: ServiceName,
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> RequestBuilder<'r> {
        self.new_request(Method::POST, service_name, into_endpoints.into())
    }

    pub fn put<'r>(
        &'r self,
        service_name: ServiceName,
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> RequestBuilder<'r> {
        self.new_request(Method::PUT, service_name, into_endpoints.into())
    }

    fn new_request<'r>(
        &'r self,
        method: Method,
        service_name: ServiceName,
        into_endpoints: IntoEndpoints<'r>,
    ) -> RequestBuilder<'r> {
        RequestBuilder::new(self, method, into_endpoints, service_name)
    }

    #[inline]
    pub(super) fn use_https(&self) -> bool {
        self.inner.use_https
    }

    #[inline]
    pub(super) fn appended_user_agent(&self) -> &str {
        &self.inner.appended_user_agent
    }

    #[inline]
    pub(super) fn connect_timeout(&self) -> Option<Duration> {
        self.inner.connect_timeout
    }

    #[inline]
    pub(super) fn request_timeout(&self) -> Option<Duration> {
        self.inner.request_timeout
    }

    #[inline]
    pub(super) fn callbacks(&self) -> &Callbacks {
        &self.inner.callbacks
    }

    #[inline]
    pub(super) fn http_caller(&self) -> &dyn HTTPCaller {
        self.inner.http_caller.as_ref()
    }

    #[inline]
    pub(super) fn request_retrier(&self) -> &dyn RequestRetrier {
        self.inner.request_retrier.as_ref()
    }

    #[inline]
    pub(super) fn retry_delay_policy(&self) -> &dyn RetryDelayPolicy {
        self.inner.retry_delay_policy.as_ref()
    }

    #[inline]
    pub(super) fn chooser(&self) -> &dyn Chooser {
        self.inner.chooser.as_ref()
    }
}

#[derive(Debug)]
pub struct HTTPClientBuilder {
    use_https: bool,
    appended_user_agent: Box<str>,
    http_caller: Arc<dyn HTTPCaller>,
    request_retrier: Arc<dyn RequestRetrier>,
    retry_delay_policy: Arc<dyn RetryDelayPolicy>,
    chooser: Arc<dyn Chooser>,
    callbacks: CallbacksBuilder,
    connect_timeout: Option<Duration>,
    request_timeout: Option<Duration>,
}

#[cfg(feature = "curl")]
impl Default for HTTPClientBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl HTTPClientBuilder {
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
        type DefaultResolver = ShuffledResolver<CachedResolver<SimpleResolver>>;
        type DefaultRetryDelayPolicy = RandomizedRetryDelayPolicy<ExponentialRetryDelayPolicy>;
        type DefaultChooser = ShuffledChooser<SimpleChooser<DefaultResolver>>;

        HTTPClientBuilder {
            http_caller,
            use_https: true,
            appended_user_agent: Default::default(),
            request_retrier: Arc::new(DefaultRetrier::default()),
            retry_delay_policy: Arc::new(DefaultRetryDelayPolicy::default()),
            chooser: Arc::new(DefaultChooser::default()),
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
    pub fn http_caller(mut self, http_caller: Arc<dyn HTTPCaller>) -> Self {
        self.http_caller = http_caller;
        self
    }

    #[inline]
    pub fn request_retrier(mut self, request_retrier: Arc<dyn RequestRetrier>) -> Self {
        self.request_retrier = request_retrier;
        self
    }

    #[inline]
    pub fn retry_delay_policy(mut self, retry_delay_policy: Arc<dyn RetryDelayPolicy>) -> Self {
        self.retry_delay_policy = retry_delay_policy;
        self
    }

    #[inline]
    pub fn chooser(mut self, chooser: Arc<dyn Chooser>) -> Self {
        self.chooser = chooser;
        self
    }

    #[inline]
    pub fn on_uploading_progress(mut self, callback: OnProgress) -> Self {
        self.callbacks = self.callbacks.on_uploading_progress(callback);
        self
    }

    #[inline]
    pub fn on_downloading_progress(mut self, callback: OnProgress) -> Self {
        self.callbacks = self.callbacks.on_downloading_progress(callback);
        self
    }

    #[inline]
    pub fn on_send_request_body(mut self, callback: OnBody) -> Self {
        self.callbacks = self.callbacks.on_send_request_body(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_status(mut self, callback: OnStatusCode) -> Self {
        self.callbacks = self.callbacks.on_receive_response_status(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_body(mut self, callback: OnBody) -> Self {
        self.callbacks = self.callbacks.on_receive_response_body(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_header(mut self, callback: OnHeader) -> Self {
        self.callbacks = self.callbacks.on_receive_response_header(callback);
        self
    }

    #[inline]
    pub fn on_to_choose_domain(mut self, callback: OnToChooseDomain) -> Self {
        self.callbacks = self.callbacks.on_to_choose_domain(callback);
        self
    }

    #[inline]
    pub fn on_domain_chosen(mut self, callback: OnDomainChosen) -> Self {
        self.callbacks = self.callbacks.on_domain_chosen(callback);
        self
    }

    #[inline]
    pub fn on_before_request_signed(mut self, callback: OnRequest) -> Self {
        self.callbacks = self.callbacks.on_before_request_signed(callback);
        self
    }

    #[inline]
    pub fn on_after_request_signed(mut self, callback: OnRequest) -> Self {
        self.callbacks = self.callbacks.on_after_request_signed(callback);
        self
    }

    #[inline]
    pub fn on_error(mut self, callback: OnError) -> Self {
        self.callbacks = self.callbacks.on_error(callback);
        self
    }

    #[inline]
    pub fn on_before_retry_delay(mut self, callback: OnRetry) -> Self {
        self.callbacks = self.callbacks.on_before_retry_delay(callback);
        self
    }

    #[inline]
    pub fn on_after_retry_delay(mut self, callback: OnRetry) -> Self {
        self.callbacks = self.callbacks.on_after_retry_delay(callback);
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
    pub fn build(self) -> HTTPClient {
        HTTPClient {
            inner: Arc::new(HTTPClientInner {
                use_https: self.use_https,
                appended_user_agent: self.appended_user_agent,
                http_caller: self.http_caller,
                request_retrier: self.request_retrier,
                retry_delay_policy: self.retry_delay_policy,
                chooser: self.chooser,
                callbacks: self.callbacks.build(),
                connect_timeout: self.connect_timeout,
                request_timeout: self.request_timeout,
            }),
        }
    }
}
