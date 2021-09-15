use super::{
    super::{IntoEndpoints, ServiceName},
    callbacks::{
        OnDomainResolved, OnError, OnHeader, OnIPsChosen, OnProgress, OnRequest, OnRetry,
        OnStatusCode, OnToChooseIPs, OnToResolveDomain,
    },
    CachedResolver, Callbacks, CallbacksBuilder, ChainedResolver, Chooser, ErrorRetrier,
    ExponentialRetryDelayPolicy, LimitedRetrier, NeverChooseNoneChooser,
    RandomizedRetryDelayPolicy, RequestBuilder, RequestRetrier, Resolver, RetryDelayPolicy,
    ShuffledChooser, ShuffledResolver, SimpleResolver, SubnetChooser, TimeoutResolver,
};
use qiniu_http::{HTTPCaller, Method};
use std::sync::Arc;

#[cfg(feature = "isahc")]
use qiniu_isahc::isahc::error::Error as IsahcError;

#[cfg(feature = "isahc")]
pub use qiniu_isahc;

#[cfg(feature = "reqwest")]
pub use qiniu_reqwest;

#[derive(Debug, Clone)]
pub struct HTTPClient {
    inner: Arc<HTTPClientInner>,
}

#[derive(Debug)]
struct HTTPClientInner {
    use_https: bool,
    appended_user_agent: Box<str>,
    http_caller: Box<dyn HTTPCaller>,
    request_retrier: Box<dyn RequestRetrier>,
    retry_delay_policy: Box<dyn RetryDelayPolicy>,
    chooser: Box<dyn Chooser>,
    resolver: Box<dyn Resolver>,
    callbacks: Callbacks,
}

impl HTTPClient {
    #[inline]
    #[cfg(feature = "isahc")]
    pub fn isahc() -> Result<Self, IsahcError> {
        Ok(Self::build_isahc()?.build())
    }

    #[inline]
    #[cfg(feature = "reqwest")]
    pub fn reqwest_sync() -> Self {
        Self::build_reqwest_sync().build()
    }

    #[inline]
    #[cfg(all(feature = "reqwest", feature = "async"))]
    pub fn reqwest_async() -> Self {
        Self::build_reqwest_async().build()
    }

    #[inline]
    #[cfg(feature = "isahc")]
    pub fn build_isahc() -> Result<HTTPClientBuilder, IsahcError> {
        HTTPClientBuilder::isahc()
    }

    #[inline]
    #[cfg(feature = "reqwest")]
    pub fn build_reqwest_sync() -> HTTPClientBuilder {
        HTTPClientBuilder::reqwest_sync()
    }

    #[inline]
    #[cfg(all(feature = "reqwest", feature = "async"))]
    pub fn build_reqwest_async() -> HTTPClientBuilder {
        HTTPClientBuilder::reqwest_async()
    }

    #[inline]
    pub fn new(http_caller: Box<dyn HTTPCaller>) -> Self {
        HTTPClientBuilder::new(http_caller).build()
    }

    #[inline]
    pub fn builder(http_caller: Box<dyn HTTPCaller>) -> HTTPClientBuilder {
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

    #[inline]
    pub(super) fn resolver(&self) -> &dyn Resolver {
        self.inner.resolver.as_ref()
    }
}

#[derive(Debug)]
pub struct HTTPClientBuilder {
    use_https: bool,
    appended_user_agent: Box<str>,
    http_caller: Box<dyn HTTPCaller>,
    request_retrier: Box<dyn RequestRetrier>,
    retry_delay_policy: Box<dyn RetryDelayPolicy>,
    chooser: Box<dyn Chooser>,
    resolver: Box<dyn Resolver>,
    callbacks: CallbacksBuilder,
}

impl HTTPClientBuilder {
    #[inline]
    #[cfg(feature = "isahc")]
    pub fn isahc() -> Result<Self, IsahcError> {
        Ok(Self::_new(Box::new(qiniu_isahc::Client::default_client()?)))
    }

    #[inline]
    #[cfg(feature = "reqwest")]
    pub fn reqwest_sync() -> Self {
        Self::_new(Box::new(qiniu_reqwest::SyncReqwestHTTPCaller::default()))
    }

    #[inline]
    #[cfg(all(feature = "reqwest", feature = "async"))]
    pub fn reqwest_async() -> Self {
        Self::_new(Box::new(qiniu_reqwest::AsyncReqwestHTTPCaller::default()))
    }

    #[inline]
    pub fn new(http_caller: Box<dyn HTTPCaller>) -> Self {
        Self::_new(http_caller)
    }

    #[inline]
    fn _new(http_caller: Box<dyn HTTPCaller>) -> Self {
        return HTTPClientBuilder {
            http_caller,
            use_https: true,
            appended_user_agent: Default::default(),
            request_retrier: default_retrier(),
            retry_delay_policy: default_retry_delay_policy(),
            chooser: default_chooser(),
            resolver: default_resolver(),
            callbacks: Default::default(),
        };

        #[inline]
        fn default_resolver() -> Box<dyn Resolver> {
            let chained_resolver = {
                let base_resolver = Box::new(TimeoutResolver::<SimpleResolver>::default());

                #[allow(unused_mut)]
                let mut builder = ChainedResolver::builder(base_resolver);

                #[cfg(feature = "c_ares")]
                if let Ok(resolver) = super::CAresResolver::new() {
                    builder = builder.prepend_resolver(Box::new(resolver));
                }

                #[cfg(all(feature = "trust_dns", feature = "async"))]
                if let Ok(resolver) = async_std::task::block_on(async {
                    super::TrustDnsResolver::from_system_conf().await
                }) {
                    builder = builder.prepend_resolver(Box::new(resolver));
                }

                builder.build()
            };
            let cached_resolver = CachedResolver::default_load_or_create_from(chained_resolver);
            let shuffled_resolver = ShuffledResolver::new(cached_resolver);
            Box::new(shuffled_resolver)
        }

        #[inline]
        fn default_chooser() -> Box<dyn Chooser> {
            Box::new(NeverChooseNoneChooser::<ShuffledChooser<SubnetChooser>>::default())
        }

        #[inline]
        fn default_retrier() -> Box<dyn RequestRetrier> {
            Box::new(LimitedRetrier::<ErrorRetrier>::default())
        }

        #[inline]
        fn default_retry_delay_policy() -> Box<dyn RetryDelayPolicy> {
            Box::new(RandomizedRetryDelayPolicy::<ExponentialRetryDelayPolicy>::default())
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
    pub fn http_caller(mut self, http_caller: Box<dyn HTTPCaller>) -> Self {
        self.http_caller = http_caller;
        self
    }

    #[inline]
    pub fn request_retrier(mut self, request_retrier: Box<dyn RequestRetrier>) -> Self {
        self.request_retrier = request_retrier;
        self
    }

    #[inline]
    pub fn retry_delay_policy(mut self, retry_delay_policy: Box<dyn RetryDelayPolicy>) -> Self {
        self.retry_delay_policy = retry_delay_policy;
        self
    }

    #[inline]
    pub fn chooser(mut self, chooser: Box<dyn Chooser>) -> Self {
        self.chooser = chooser;
        self
    }

    #[inline]
    pub fn resolver(mut self, resolver: Box<dyn Resolver>) -> Self {
        self.resolver = resolver;
        self
    }

    #[inline]
    pub fn on_uploading_progress(mut self, callback: OnProgress) -> Self {
        self.callbacks = self.callbacks.on_uploading_progress(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_status(mut self, callback: OnStatusCode) -> Self {
        self.callbacks = self.callbacks.on_receive_response_status(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_header(mut self, callback: OnHeader) -> Self {
        self.callbacks = self.callbacks.on_receive_response_header(callback);
        self
    }

    #[inline]
    pub fn on_to_resolve_domain(mut self, callback: OnToResolveDomain) -> Self {
        self.callbacks = self.callbacks.on_to_resolve_domain(callback);
        self
    }

    #[inline]
    pub fn on_domain_resolved(mut self, callback: OnDomainResolved) -> Self {
        self.callbacks = self.callbacks.on_domain_resolved(callback);
        self
    }

    #[inline]
    pub fn on_to_choose_ips(mut self, callback: OnToChooseIPs) -> Self {
        self.callbacks = self.callbacks.on_to_choose_ips(callback);
        self
    }

    #[inline]
    pub fn on_ips_chosen(mut self, callback: OnIPsChosen) -> Self {
        self.callbacks = self.callbacks.on_ips_chosen(callback);
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
    pub fn build(self) -> HTTPClient {
        HTTPClient {
            inner: Arc::new(HTTPClientInner {
                use_https: self.use_https,
                appended_user_agent: self.appended_user_agent,
                http_caller: self.http_caller,
                request_retrier: self.request_retrier,
                retry_delay_policy: self.retry_delay_policy,
                chooser: self.chooser,
                resolver: self.resolver,
                callbacks: self.callbacks.build(),
            }),
        }
    }
}
