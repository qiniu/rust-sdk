use super::{
    super::{IntoEndpoints, IpAddrWithPort, ServiceName},
    Backoff, CachedResolver, CallbackContext, Callbacks, CallbacksBuilder, ChainedResolver,
    Chooser, ErrorRetrier, ExponentialBackoff, ExtendedCallbackContext, LimitedRetrier,
    NeverEmptyHandedChooser, RandomizedBackoff, RequestRetrier, ResolveAnswers, Resolver,
    ResponseError, ShuffledChooser, ShuffledResolver, SimpleResolver, SimplifiedCallbackContext,
    SubnetChooser, SyncRequestBuilder, TimeoutResolver,
};
use cfg_if::cfg_if;
use qiniu_http::{
    HeaderName, HeaderValue, HttpCaller, Method, StatusCode, TransferProgressInfo, UserAgent,
};
use std::{sync::Arc, time::Duration};

#[cfg(feature = "isahc")]
use qiniu_isahc::isahc::error::Error as IsahcError;

#[cfg(feature = "async")]
use super::AsyncRequestBuilder;

#[derive(Debug, Clone)]
pub struct HttpClient {
    inner: Arc<HttpClientInner>,
}

#[derive(Debug)]
struct HttpClientInner {
    use_https: bool,
    appended_user_agent: UserAgent,
    http_caller: Box<dyn HttpCaller>,
    request_retrier: Box<dyn RequestRetrier>,
    backoff: Box<dyn Backoff>,
    chooser: Box<dyn Chooser>,
    resolver: Box<dyn Resolver>,
    callbacks: Callbacks<'static>,
}

impl HttpClient {
    #[inline]
    #[cfg(feature = "ureq")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "ureq")))]
    pub fn ureq() -> Self {
        Self::build_ureq().build()
    }

    #[inline]
    #[cfg(feature = "isahc")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "isahc")))]
    pub fn isahc() -> Result<Self, IsahcError> {
        Ok(Self::build_isahc()?.build())
    }

    #[inline]
    #[cfg(feature = "reqwest")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "reqwest")))]
    pub fn reqwest_sync() -> Self {
        Self::build_reqwest_sync().build()
    }

    #[inline]
    #[cfg(all(feature = "reqwest", feature = "async"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(all(feature = "reqwest", feature = "async")))
    )]
    pub fn reqwest_async() -> Self {
        Self::build_reqwest_async().build()
    }

    #[inline]
    pub fn build_default() -> HttpClientBuilder {
        HttpClientBuilder::default()
    }

    #[inline]
    #[cfg(feature = "ureq")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "ureq")))]
    pub fn build_ureq() -> HttpClientBuilder {
        HttpClientBuilder::ureq()
    }

    #[inline]
    #[cfg(feature = "isahc")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "isahc")))]
    pub fn build_isahc() -> Result<HttpClientBuilder, IsahcError> {
        HttpClientBuilder::isahc()
    }

    #[inline]
    #[cfg(feature = "reqwest")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "reqwest")))]
    pub fn build_reqwest_sync() -> HttpClientBuilder {
        HttpClientBuilder::reqwest_sync()
    }

    #[inline]
    #[cfg(all(feature = "reqwest", feature = "async"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(all(feature = "reqwest", feature = "async")))
    )]
    pub fn build_reqwest_async() -> HttpClientBuilder {
        HttpClientBuilder::reqwest_async()
    }

    #[inline]
    pub fn new(http_caller: Box<dyn HttpCaller>) -> Self {
        HttpClientBuilder::new(http_caller).build()
    }

    #[inline]
    pub fn builder(http_caller: Box<dyn HttpCaller>) -> HttpClientBuilder {
        HttpClientBuilder::new(http_caller)
    }

    #[inline]
    pub fn get<'r>(
        &'r self,
        service_names: &'r [ServiceName],
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> SyncRequestBuilder<'r> {
        self.new_sync_request(Method::GET, service_names, into_endpoints.into())
    }

    #[inline]
    pub fn head<'r>(
        &'r self,
        service_names: &'r [ServiceName],
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> SyncRequestBuilder<'r> {
        self.new_sync_request(Method::HEAD, service_names, into_endpoints.into())
    }

    #[inline]
    pub fn post<'r>(
        &'r self,
        service_names: &'r [ServiceName],
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> SyncRequestBuilder<'r> {
        self.new_sync_request(Method::POST, service_names, into_endpoints.into())
    }

    #[inline]
    pub fn put<'r>(
        &'r self,
        service_names: &'r [ServiceName],
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> SyncRequestBuilder<'r> {
        self.new_sync_request(Method::PUT, service_names, into_endpoints.into())
    }

    #[inline]
    pub fn delete<'r>(
        &'r self,
        service_names: &'r [ServiceName],
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> SyncRequestBuilder<'r> {
        self.new_sync_request(Method::DELETE, service_names, into_endpoints.into())
    }

    #[inline]
    fn new_sync_request<'r>(
        &'r self,
        method: Method,
        service_names: &'r [ServiceName],
        into_endpoints: IntoEndpoints<'r>,
    ) -> SyncRequestBuilder<'r> {
        SyncRequestBuilder::new(self, method, into_endpoints, service_names)
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub fn async_get<'r>(
        &'r self,
        service_names: &'r [ServiceName],
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> AsyncRequestBuilder<'r> {
        self.new_async_request(Method::GET, service_names, into_endpoints.into())
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub fn async_head<'r>(
        &'r self,
        service_names: &'r [ServiceName],
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> AsyncRequestBuilder<'r> {
        self.new_async_request(Method::HEAD, service_names, into_endpoints.into())
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub fn async_post<'r>(
        &'r self,
        service_names: &'r [ServiceName],
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> AsyncRequestBuilder<'r> {
        self.new_async_request(Method::POST, service_names, into_endpoints.into())
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub fn async_put<'r>(
        &'r self,
        service_names: &'r [ServiceName],
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> AsyncRequestBuilder<'r> {
        self.new_async_request(Method::PUT, service_names, into_endpoints.into())
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub fn async_delete<'r>(
        &'r self,
        service_names: &'r [ServiceName],
        into_endpoints: impl Into<IntoEndpoints<'r>>,
    ) -> AsyncRequestBuilder<'r> {
        self.new_async_request(Method::DELETE, service_names, into_endpoints.into())
    }

    #[inline]
    #[cfg(feature = "async")]
    fn new_async_request<'r>(
        &'r self,
        method: Method,
        service_names: &'r [ServiceName],
        into_endpoints: IntoEndpoints<'r>,
    ) -> AsyncRequestBuilder<'r> {
        AsyncRequestBuilder::new(self, method, into_endpoints, service_names)
    }

    #[inline]
    pub(super) fn use_https(&self) -> bool {
        self.inner.use_https
    }

    #[inline]
    pub(super) fn appended_user_agent(&self) -> &UserAgent {
        &self.inner.appended_user_agent
    }

    #[inline]
    pub(super) fn callbacks(&self) -> &Callbacks<'static> {
        &self.inner.callbacks
    }

    #[inline]
    pub(super) fn http_caller(&self) -> &dyn HttpCaller {
        self.inner.http_caller.as_ref()
    }

    #[inline]
    pub(super) fn request_retrier(&self) -> &dyn RequestRetrier {
        self.inner.request_retrier.as_ref()
    }

    #[inline]
    pub(super) fn backoff(&self) -> &dyn Backoff {
        self.inner.backoff.as_ref()
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
pub struct HttpClientBuilder {
    use_https: bool,
    appended_user_agent: UserAgent,
    http_caller: Box<dyn HttpCaller>,
    request_retrier: Box<dyn RequestRetrier>,
    backoff: Box<dyn Backoff>,
    chooser: Box<dyn Chooser>,
    resolver: Box<dyn Resolver>,
    callbacks: CallbacksBuilder<'static>,
}

impl HttpClientBuilder {
    #[inline]
    #[cfg(feature = "ureq")]
    pub fn ureq() -> Self {
        Self::_new(Box::new(qiniu_ureq::Client::default()))
    }

    #[inline]
    #[cfg(feature = "isahc")]
    pub fn isahc() -> Result<Self, IsahcError> {
        Ok(Self::_new(Box::new(qiniu_isahc::Client::default_client()?)))
    }

    #[inline]
    #[cfg(feature = "reqwest")]
    pub fn reqwest_sync() -> Self {
        Self::_new(Box::new(qiniu_reqwest::SyncReqwestHttpCaller::default()))
    }

    #[inline]
    #[cfg(all(feature = "reqwest", feature = "async"))]
    pub fn reqwest_async() -> Self {
        Self::_new(Box::new(qiniu_reqwest::AsyncReqwestHttpCaller::default()))
    }

    #[inline]
    pub fn new(http_caller: Box<dyn HttpCaller>) -> Self {
        Self::_new(http_caller)
    }

    #[inline]
    fn _new(http_caller: Box<dyn HttpCaller>) -> Self {
        return HttpClientBuilder {
            http_caller,
            use_https: true,
            appended_user_agent: Default::default(),
            request_retrier: HttpClient::default_retrier(),
            backoff: HttpClient::default_backoff(),
            chooser: HttpClient::default_chooser(),
            resolver: HttpClient::default_resolver(),
            callbacks: Default::default(),
        };
    }

    #[inline]
    pub fn use_https(mut self, use_https: bool) -> Self {
        self.use_https = use_https;
        self
    }

    #[inline]
    pub fn appended_user_agent(mut self, appended_user_agent: impl Into<UserAgent>) -> Self {
        self.appended_user_agent = appended_user_agent.into();
        self
    }

    #[inline]
    pub fn http_caller(mut self, http_caller: Box<dyn HttpCaller>) -> Self {
        self.http_caller = http_caller;
        self
    }

    #[inline]
    pub fn request_retrier(mut self, request_retrier: Box<dyn RequestRetrier>) -> Self {
        self.request_retrier = request_retrier;
        self
    }

    #[inline]
    pub fn backoff(mut self, backoff: Box<dyn Backoff>) -> Self {
        self.backoff = backoff;
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
    pub fn on_uploading_progress(
        mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, &TransferProgressInfo) -> bool
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_uploading_progress(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_status(
        mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, StatusCode) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_receive_response_status(callback);
        self
    }

    #[inline]
    pub fn on_receive_response_header(
        mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, &HeaderName, &HeaderValue) -> bool
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_receive_response_header(callback);
        self
    }

    #[inline]
    pub fn on_to_resolve_domain(
        mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_to_resolve_domain(callback);
        self
    }

    #[inline]
    pub fn on_domain_resolved(
        mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str, &ResolveAnswers) -> bool
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_domain_resolved(callback);
        self
    }

    #[inline]
    pub fn on_to_choose_ips(
        mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort]) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_to_choose_ips(callback);
        self
    }

    #[inline]
    pub fn on_ips_chosen(
        mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort], &[IpAddrWithPort]) -> bool
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_ips_chosen(callback);
        self
    }

    #[inline]
    pub fn on_before_request_signed(
        mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_before_request_signed(callback);
        self
    }

    #[inline]
    pub fn on_after_request_signed(
        mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_after_request_signed(callback);
        self
    }

    #[inline]
    pub fn on_error(
        mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseError) -> bool
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_error(callback);
        self
    }

    #[inline]
    pub fn on_before_backoff(
        mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_before_backoff(callback);
        self
    }

    #[inline]
    pub fn on_after_backoff(
        mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.callbacks = self.callbacks.on_after_backoff(callback);
        self
    }

    #[inline]
    pub fn build(self) -> HttpClient {
        HttpClient {
            inner: Arc::new(HttpClientInner {
                use_https: self.use_https,
                appended_user_agent: self.appended_user_agent,
                http_caller: self.http_caller,
                request_retrier: self.request_retrier,
                backoff: self.backoff,
                chooser: self.chooser,
                resolver: self.resolver,
                callbacks: self.callbacks.build(),
            }),
        }
    }
}

impl Default for HttpClientBuilder {
    #[inline]
    fn default() -> Self {
        HttpClientBuilder::_new(HttpClient::default_http_caller())
    }
}

impl Default for HttpClient {
    #[inline]
    fn default() -> Self {
        HttpClientBuilder::default().build()
    }
}

impl HttpClient {
    #[inline]
    pub fn default_http_caller() -> Box<dyn HttpCaller> {
        cfg_if! {
            if #[cfg(all(feature = "ureq", not(feature = "async")))] {
                Box::new(qiniu_ureq::Client::default())
            } else if #[cfg(feature = "isahc")] {
                Box::new(qiniu_isahc::Client::default_client().expect("Failed to initialize isahc"))
            } else if #[cfg(all(feature = "reqwest", not(feature = "async")))] {
                Box::new(qiniu_reqwest::SyncReqwestHttpCaller::default())
            } else if #[cfg(all(feature = "reqwest", feature = "async"))] {
                Box::new(qiniu_reqwest::AsyncReqwestHttpCaller::default())
            } else {
                panic!("No http caller available, can you enable feature `isahc` to resolve this problem?")
            }
        }
    }

    #[inline]
    pub fn default_resolver() -> Box<dyn Resolver> {
        let chained_resolver = {
            let base_resolver = Box::new(TimeoutResolver::<SimpleResolver>::default());

            #[allow(unused_mut)]
            let mut builder = ChainedResolver::builder(base_resolver);

            #[cfg(feature = "c_ares")]
            if let Ok(resolver) = super::CAresResolver::new() {
                builder.prepend_resolver(Box::new(resolver));
            }

            #[cfg(all(feature = "trust_dns", feature = "async"))]
            if let Ok(resolver) = async_std::task::block_on(async {
                super::TrustDnsResolver::from_system_conf().await
            }) {
                builder.prepend_resolver(Box::new(resolver));
            }

            builder.build()
        };
        let cached_resolver = CachedResolver::builder(chained_resolver).in_memory();
        let shuffled_resolver = ShuffledResolver::new(cached_resolver);
        Box::new(shuffled_resolver)
    }

    #[inline]
    pub fn default_chooser() -> Box<dyn Chooser> {
        Box::new(NeverEmptyHandedChooser::<ShuffledChooser<SubnetChooser>>::default())
    }

    #[inline]
    pub fn default_retrier() -> Box<dyn RequestRetrier> {
        Box::new(LimitedRetrier::<ErrorRetrier>::default())
    }

    #[inline]
    pub fn default_backoff() -> Box<dyn Backoff> {
        Box::new(RandomizedBackoff::<ExponentialBackoff>::default())
    }
}
