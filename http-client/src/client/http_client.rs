use super::{
    super::{EndpointsProvider, IpAddrWithPort, ServiceName},
    callbacks::{Callbacks, CallbacksBuilder},
    Backoff, CachedResolver, CallbackContext, ChainedResolver, Chooser, ErrorRetrier, ExponentialBackoff,
    ExtendedCallbackContext, LimitedBackoff, LimitedRetrier, NeverEmptyHandedChooser, RandomizedBackoff,
    RequestRetrier, ResolveAnswers, Resolver, ResponseError, ShuffledChooser, ShuffledResolver, SimpleResolver,
    SimplifiedCallbackContext, SubnetChooser, SyncRequestBuilder, TimeoutResolver,
};
use anyhow::Result as AnyResult;
use assert_impl::assert_impl;
use cfg_if::cfg_if;
use qiniu_http::{
    HeaderName, HeaderValue, HttpCaller, Method, ResponseParts, StatusCode, TransferProgressInfo, UserAgent,
};
use std::{
    mem::{replace, take},
    sync::Arc,
    time::Duration,
};

#[cfg(feature = "isahc")]
use qiniu_isahc::isahc::error::Error as IsahcError;

#[cfg(feature = "async")]
use super::AsyncRequestBuilder;

/// HTTP 客户端
///
/// 用于发送 HTTP 请求的入口。
///
/// 其中 HTTP 请求将由 [`HttpCaller`] 实现的实例来发送，如果不指定，默认通过当前启用的功能来判定。
///
/// ### 私有云获取当前账户的 Buckets 列表
///
/// ##### 阻塞代码示例
///
/// ```
/// use qiniu_credential::Credential;
/// use qiniu_http_client::{Authorization, HttpClient, Region, RegionsProviderEndpoints, ServiceName};
///
/// # fn example() -> anyhow::Result<()> {
/// let region = Region::builder("z0")
///     .add_uc_preferred_endpoint("uc-qos.pocdemo.qiniu.io".parse()?)
///     .build();
/// let credential = Credential::new("abcdefghklmnopq", "1234567890");
/// let bucket_names: Vec<String> = HttpClient::default()
///     .get(&[ServiceName::Uc], RegionsProviderEndpoints::new(region))
///     .use_https(false)
///     .authorization(Authorization::v2(credential))
///     .accept_json()
///     .path("/buckets")
///     .call()?
///     .parse_json()?
///     .into_body();
/// # Ok(())
/// # }
/// ```
///
/// ##### 异步代码示例
///
/// ```
/// use qiniu_credential::Credential;
/// use qiniu_http_client::{Authorization, HttpClient, Region, RegionsProviderEndpoints, ServiceName};
///
/// # async fn example() -> anyhow::Result<()> {
/// let region = Region::builder("z0")
///     .add_uc_preferred_endpoint("uc-qos.pocdemo.qiniu.io".parse()?)
///     .build();
/// let credential = Credential::new("abcdefghklmnopq", "1234567890");
/// let bucket_names: Vec<String> = HttpClient::default()
///     .async_get(&[ServiceName::Uc], RegionsProviderEndpoints::new(region))
///     .use_https(false)
///     .authorization(Authorization::v2(credential))
///     .accept_json()
///     .path("/buckets")
///     .call()
///     .await?
///     .parse_json()
///     .await?
///     .into_body();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct HttpClient {
    inner: Arc<HttpClientInner>,
}

impl HttpClient {
    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
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
    /// 创建一个新的 HTTP 客户端，使用 [`crate::ureq::Client`] 作为 [`HttpCaller`] 实现
    #[inline]
    #[cfg(feature = "ureq")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "ureq")))]
    pub fn ureq() -> Self {
        Self::build_ureq().build()
    }

    /// 创建一个新的 HTTP 客户端，使用 [`crate::isahc::Client`] 作为 [`HttpCaller`] 实现
    #[inline]
    #[cfg(feature = "isahc")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "isahc")))]
    pub fn isahc() -> Result<Self, IsahcError> {
        Ok(Self::build_isahc()?.build())
    }

    /// 创建一个新的 HTTP 客户端，使用 [`crate::reqwest::SyncClient`] 作为 [`HttpCaller`] 实现
    #[inline]
    #[cfg(feature = "reqwest")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "reqwest")))]
    pub fn reqwest_sync() -> Self {
        Self::build_reqwest_sync().build()
    }

    /// 创建一个新的 HTTP 客户端，使用 [`crate::reqwest::AsyncClient`] 作为 [`HttpCaller`] 实现
    #[inline]
    #[cfg(all(feature = "reqwest", feature = "async"))]
    #[cfg_attr(feature = "docs", doc(cfg(all(feature = "reqwest", feature = "async"))))]
    pub fn reqwest_async() -> Self {
        Self::build_reqwest_async().build()
    }

    /// 创建一个新的 HTTP 客户端，根据当前环境变量选择 [`HttpCaller`] 实现
    #[inline]
    pub fn build_default() -> HttpClientBuilder {
        HttpClientBuilder::default()
    }

    /// 创建一个新的 HTTP 客户端构建器，使用 [`crate::ureq::Client`] 作为 [`HttpCaller`] 实现
    #[inline]
    #[cfg(feature = "ureq")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "ureq")))]
    pub fn build_ureq() -> HttpClientBuilder {
        HttpClientBuilder::ureq()
    }

    /// 创建一个新的 HTTP 客户端构建器，使用 [`crate::isahc::Client`] 作为 [`HttpCaller`] 实现
    #[inline]
    #[cfg(feature = "isahc")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "isahc")))]
    pub fn build_isahc() -> Result<HttpClientBuilder, IsahcError> {
        HttpClientBuilder::isahc()
    }

    /// 创建一个新的 HTTP 客户端构建器，使用 [`crate::reqwest::SyncClient`] 作为 [`HttpCaller`] 实现
    #[inline]
    #[cfg(feature = "reqwest")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "reqwest")))]
    pub fn build_reqwest_sync() -> HttpClientBuilder {
        HttpClientBuilder::reqwest_sync()
    }

    /// 创建一个新的 HTTP 客户端构建器，使用 [`crate::reqwest::AsyncClient`] 作为 [`HttpCaller`] 实现
    #[inline]
    #[cfg(all(feature = "reqwest", feature = "async"))]
    #[cfg_attr(feature = "docs", doc(cfg(all(feature = "reqwest", feature = "async"))))]
    pub fn build_reqwest_async() -> HttpClientBuilder {
        HttpClientBuilder::reqwest_async()
    }

    /// 创建一个新的 HTTP 客户端，需要指定 [`HttpCaller`] 实现
    #[inline]
    pub fn new(http_caller: impl HttpCaller + 'static) -> Self {
        HttpClientBuilder::new(http_caller).build()
    }

    /// 创建一个新的 HTTP 客户端构建器，需要指定 [`HttpCaller`] 实现
    #[inline]
    pub fn builder(http_caller: impl HttpCaller + 'static) -> HttpClientBuilder {
        HttpClientBuilder::new(http_caller)
    }

    /// 创建 GET 请求的请求构建器
    ///
    /// 该方法的异步版本为 [`HttpClient::async_get`]。
    #[inline]
    pub fn get<'r, E: EndpointsProvider + 'r>(
        &'r self,
        service_names: &'r [ServiceName],
        endpoints_provider: E,
    ) -> SyncRequestBuilder<'r, E> {
        self.new_request(Method::GET, service_names, endpoints_provider)
    }

    /// 创建 POST 请求的请求构建器
    ///
    /// 该方法的异步版本为 [`HttpClient::async_post`]。
    #[inline]
    pub fn post<'r, E: EndpointsProvider + 'r>(
        &'r self,
        service_names: &'r [ServiceName],
        endpoints_provider: E,
    ) -> SyncRequestBuilder<'r, E> {
        self.new_request(Method::POST, service_names, endpoints_provider)
    }

    /// 创建 PUT 请求的请求构建器
    ///
    /// 该方法的异步版本为 [`HttpClient::async_put`]。
    #[inline]
    pub fn put<'r, E: EndpointsProvider + 'r>(
        &'r self,
        service_names: &'r [ServiceName],
        endpoints_provider: E,
    ) -> SyncRequestBuilder<'r, E> {
        self.new_request(Method::PUT, service_names, endpoints_provider)
    }

    /// 创建 DELETE 请求的请求构建器
    ///
    /// 该方法的异步版本为 [`HttpClient::async_delete`]。
    #[inline]
    pub fn delete<'r, E: EndpointsProvider + 'r>(
        &'r self,
        service_names: &'r [ServiceName],
        endpoints_provider: E,
    ) -> SyncRequestBuilder<'r, E> {
        self.new_request(Method::DELETE, service_names, endpoints_provider)
    }

    /// 创建请求的请求构建器
    ///
    /// 该方法的异步版本为 [`HttpClient::new_async_request`]。
    #[inline]
    pub fn new_request<'r, E: EndpointsProvider + 'r>(
        &'r self,
        method: Method,
        service_names: &'r [ServiceName],
        endpoints_provider: E,
    ) -> SyncRequestBuilder<'r, E> {
        SyncRequestBuilder::new(self, method, endpoints_provider, service_names)
    }

    /// 创建 GET 请求的异步请求构建器
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub fn async_get<'r, E: EndpointsProvider + 'r>(
        &'r self,
        service_names: &'r [ServiceName],
        endpoints_provider: E,
    ) -> AsyncRequestBuilder<'r, E> {
        self.new_async_request(Method::GET, service_names, endpoints_provider)
    }

    /// 创建 POST 请求的异步请求构建器
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub fn async_post<'r, E: EndpointsProvider + 'r>(
        &'r self,
        service_names: &'r [ServiceName],
        endpoints_provider: E,
    ) -> AsyncRequestBuilder<'r, E> {
        self.new_async_request(Method::POST, service_names, endpoints_provider)
    }

    /// 创建 PUT 请求的异步请求构建器
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub fn async_put<'r, E: EndpointsProvider + 'r>(
        &'r self,
        service_names: &'r [ServiceName],
        endpoints_provider: E,
    ) -> AsyncRequestBuilder<'r, E> {
        self.new_async_request(Method::PUT, service_names, endpoints_provider)
    }

    /// 创建 DELETE 请求的异步请求构建器
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub fn async_delete<'r, E: EndpointsProvider + 'r>(
        &'r self,
        service_names: &'r [ServiceName],
        endpoints_provider: E,
    ) -> AsyncRequestBuilder<'r, E> {
        self.new_async_request(Method::DELETE, service_names, endpoints_provider)
    }

    /// 创建异步请求的请求构建器
    #[cfg(feature = "async")]
    pub fn new_async_request<'r, E: EndpointsProvider + 'r>(
        &'r self,
        method: Method,
        service_names: &'r [ServiceName],
        endpoints_provider: E,
    ) -> AsyncRequestBuilder<'r, E> {
        AsyncRequestBuilder::new(self, method, endpoints_provider, service_names)
    }

    pub(super) fn use_https(&self) -> bool {
        self.inner.use_https
    }

    pub(super) fn appended_user_agent(&self) -> &UserAgent {
        &self.inner.appended_user_agent
    }

    pub(super) fn callbacks(&self) -> &Callbacks<'static> {
        &self.inner.callbacks
    }

    pub(super) fn http_caller(&self) -> &dyn HttpCaller {
        self.inner.http_caller.as_ref()
    }

    pub(super) fn request_retrier(&self) -> &dyn RequestRetrier {
        self.inner.request_retrier.as_ref()
    }

    pub(super) fn backoff(&self) -> &dyn Backoff {
        self.inner.backoff.as_ref()
    }

    pub(super) fn chooser(&self) -> &dyn Chooser {
        self.inner.chooser.as_ref()
    }

    pub(super) fn resolver(&self) -> &dyn Resolver {
        self.inner.resolver.as_ref()
    }
}

/// HTTP 客户端构建器
#[derive(Debug)]
pub struct HttpClientBuilder {
    use_https: bool,
    appended_user_agent: UserAgent,
    http_caller: Option<Box<dyn HttpCaller>>,
    request_retrier: Option<Box<dyn RequestRetrier>>,
    backoff: Option<Box<dyn Backoff>>,
    chooser: Option<Box<dyn Chooser>>,
    resolver: Option<Box<dyn Resolver>>,
    callbacks: CallbacksBuilder<'static>,
}

impl HttpClientBuilder {
    /// 创建一个新的 HTTP 客户端构建器，使用 [`crate::ureq::Client`] 作为 [`HttpCaller`] 实现
    #[inline]
    #[cfg(feature = "ureq")]
    pub fn ureq() -> Self {
        Self::_new(Some(Box::new(qiniu_ureq::Client::default())))
    }

    /// 创建一个新的 HTTP 客户端构建器，使用 [`crate::isahc::Client`] 作为 [`HttpCaller`] 实现
    #[inline]
    #[cfg(feature = "isahc")]
    pub fn isahc() -> Result<Self, IsahcError> {
        Ok(Self::_new(Some(Box::new(qiniu_isahc::Client::default_client()?))))
    }

    /// 创建一个新的 HTTP 客户端构建器，使用 [`crate::reqwest::SyncClient`] 作为 [`HttpCaller`] 实现
    #[inline]
    #[cfg(feature = "reqwest")]
    pub fn reqwest_sync() -> Self {
        Self::_new(Some(Box::new(qiniu_reqwest::SyncClient::default())))
    }

    /// 创建一个新的 HTTP 客户端构建器，使用 [`crate::reqwest::SyncClient`] 作为 [`HttpCaller`] 实现
    #[inline]
    #[cfg(all(feature = "reqwest", feature = "async"))]
    pub fn reqwest_async() -> Self {
        Self::_new(Some(Box::new(qiniu_reqwest::AsyncClient::default())))
    }

    /// 创建一个新的 HTTP 客户端构建器，需要指定 [`HttpCaller`] 实现
    #[inline]
    pub fn new(http_caller: impl HttpCaller + 'static) -> Self {
        Self::_new(Some(Box::new(http_caller)))
    }

    fn _new(http_caller: Option<Box<dyn HttpCaller>>) -> Self {
        HttpClientBuilder {
            http_caller,
            use_https: true,
            appended_user_agent: Default::default(),
            request_retrier: Default::default(),
            backoff: Default::default(),
            chooser: Default::default(),
            resolver: Default::default(),
            callbacks: Default::default(),
        }
    }

    /// 设置是否使用 HTTPS
    ///
    /// 默认为使用 HTTPS
    #[inline]
    pub fn use_https(&mut self, use_https: bool) -> &mut Self {
        self.use_https = use_https;
        self
    }

    /// 设置追加的 UserAgent
    #[inline]
    pub fn appended_user_agent(&mut self, appended_user_agent: impl Into<UserAgent>) -> &mut Self {
        self.appended_user_agent = appended_user_agent.into();
        self
    }

    /// 设置 HTTP 客户端实现
    ///
    /// 默认根据启用的功能自动选择。
    #[inline]
    pub fn http_caller(&mut self, http_caller: impl HttpCaller + 'static) -> &mut Self {
        self.http_caller = Some(Box::new(http_caller));
        self
    }

    /// 设置重试器
    ///
    /// 默认使用 [`ErrorRetrier`]，并使用 [`LimitedRetrier`] 对其进行包装。
    #[inline]
    pub fn request_retrier(&mut self, request_retrier: impl RequestRetrier + 'static) -> &mut Self {
        self.request_retrier = Some(Box::new(request_retrier));
        self
    }

    /// 设置退避器
    ///
    /// 默认使用 [`ExponentialBackoff`]，并使用 [`LimitedBackoff`] 和 [`RandomizedBackoff`] 对其进行包装。
    #[inline]
    pub fn backoff(&mut self, backoff: impl Backoff + 'static) -> &mut Self {
        self.backoff = Some(Box::new(backoff));
        self
    }

    /// 设置选择器
    ///
    /// 默认使用 [`SubnetChooser`]，并使用 [`ShuffledChooser`] 和 [`NeverEmptyHandedChooser`] 对其进行包装。
    #[inline]
    pub fn chooser(&mut self, chooser: impl Chooser + 'static) -> &mut Self {
        self.chooser = Some(Box::new(chooser));
        self
    }

    /// 设置域名解析器
    ///
    /// 默认通过当前启用的功能来判定，并使用 [`CachedResolver`] 和 [`ShuffledResolver`] 对其进行包装。
    #[inline]
    pub fn resolver(&mut self, resolver: impl Resolver + 'static) -> &mut Self {
        self.resolver = Some(Box::new(resolver));
        self
    }

    /// 设置上传进度回调函数
    #[inline]
    pub fn on_uploading_progress(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, TransferProgressInfo<'_>) -> AnyResult<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.callbacks.on_uploading_progress(callback);
        self
    }

    /// 设置响应状态码回调函数
    #[inline]
    pub fn on_receive_response_status(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, StatusCode) -> AnyResult<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.callbacks.on_receive_response_status(callback);
        self
    }

    /// 设置响应 HTTP 头回调函数
    #[inline]
    pub fn on_receive_response_header(
        &mut self,
        callback: impl Fn(&dyn SimplifiedCallbackContext, &HeaderName, &HeaderValue) -> AnyResult<()>
            + Send
            + Sync
            + 'static,
    ) -> &mut Self {
        self.callbacks.on_receive_response_header(callback);
        self
    }

    /// 设置域名解析前回调函数
    #[inline]
    pub fn on_to_resolve_domain(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str) -> AnyResult<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.callbacks.on_to_resolve_domain(callback);
        self
    }

    /// 设置域名解析成功回调函数
    #[inline]
    pub fn on_domain_resolved(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &str, &ResolveAnswers) -> AnyResult<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.callbacks.on_domain_resolved(callback);
        self
    }

    /// 设置 IP 地址选择前回调函数
    #[inline]
    pub fn on_to_choose_ips(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort]) -> AnyResult<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.callbacks.on_to_choose_ips(callback);
        self
    }

    /// 设置 IP 地址选择成功回调函数
    #[inline]
    pub fn on_ips_chosen(
        &mut self,
        callback: impl Fn(&mut dyn CallbackContext, &[IpAddrWithPort], &[IpAddrWithPort]) -> AnyResult<()>
            + Send
            + Sync
            + 'static,
    ) -> &mut Self {
        self.callbacks.on_ips_chosen(callback);
        self
    }

    /// 设置 HTTP 请求签名前回调函数
    #[inline]
    pub fn on_before_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> AnyResult<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.callbacks.on_before_request_signed(callback);
        self
    }

    /// 设置 HTTP 请求前回调函数
    #[inline]
    pub fn on_after_request_signed(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext) -> AnyResult<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.callbacks.on_after_request_signed(callback);
        self
    }

    /// 设置 HTTP 响应成功回调函数
    #[inline]
    pub fn on_response(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseParts) -> AnyResult<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.callbacks.on_response(callback);
        self
    }

    /// 设置 HTTP 响应出错回调函数
    #[inline]
    pub fn on_error(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, &ResponseError) -> AnyResult<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.callbacks.on_error(callback);
        self
    }

    /// 设置退避前回调函数
    #[inline]
    pub fn on_before_backoff(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> AnyResult<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.callbacks.on_before_backoff(callback);
        self
    }

    /// 设置退避后回调函数
    #[inline]
    pub fn on_after_backoff(
        &mut self,
        callback: impl Fn(&mut dyn ExtendedCallbackContext, Duration) -> AnyResult<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.callbacks.on_after_backoff(callback);
        self
    }

    /// 构建 HTTP 客户端
    pub fn build(&mut self) -> HttpClient {
        HttpClient {
            inner: Arc::new(HttpClientInner {
                use_https: replace(&mut self.use_https, true),
                appended_user_agent: take(&mut self.appended_user_agent),
                http_caller: take(&mut self.http_caller).unwrap_or_else(HttpClient::default_http_caller),
                request_retrier: take(&mut self.request_retrier).unwrap_or_else(HttpClient::default_retrier),
                backoff: take(&mut self.backoff).unwrap_or_else(HttpClient::default_backoff),
                chooser: take(&mut self.chooser).unwrap_or_else(HttpClient::default_chooser),
                resolver: take(&mut self.resolver).unwrap_or_else(HttpClient::default_resolver),
                callbacks: self.callbacks.build(),
            }),
        }
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl Default for HttpClientBuilder {
    #[inline]
    fn default() -> Self {
        HttpClientBuilder::_new(None)
    }
}

impl Default for HttpClient {
    #[inline]
    fn default() -> Self {
        HttpClientBuilder::default().build()
    }
}

impl HttpClient {
    /// 获得默认的 [`HttpCaller`] 实例
    ///
    /// 默认通过当前启用的功能来判定
    #[inline]
    pub fn default_http_caller() -> Box<dyn HttpCaller> {
        cfg_if! {
            if #[cfg(all(feature = "ureq", not(feature = "async")))] {
                Box::new(qiniu_ureq::Client::default())
            } else if #[cfg(feature = "isahc")] {
                Box::new(qiniu_isahc::Client::default_client().expect("Failed to initialize isahc"))
            } else if #[cfg(all(feature = "reqwest", not(feature = "async")))] {
                Box::new(qiniu_reqwest::SyncClient::default())
            } else if #[cfg(all(feature = "reqwest", feature = "async"))] {
                Box::new(qiniu_reqwest::AsyncClient::default())
            } else {
                panic!("No http caller available, can you enable feature `isahc` to resolve this problem?")
            }
        }
    }

    /// 获得默认的 [`Resolver`] 实例
    ///
    /// 默认通过当前启用的功能来判定，并使用 [`CachedResolver`] 和 [`ShuffledResolver`] 对其进行包装。
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
            if let Ok(resolver) = async_std::task::block_on(async { super::TrustDnsResolver::from_system_conf().await })
            {
                builder.prepend_resolver(Box::new(resolver));
            }

            builder.build()
        };
        let cached_resolver = CachedResolver::builder(chained_resolver).in_memory();
        let shuffled_resolver = ShuffledResolver::new(cached_resolver);
        Box::new(shuffled_resolver)
    }

    /// 获得默认的 [`Chooser`] 实例
    ///
    /// 默认使用 [`SubnetChooser`]，并使用 [`ShuffledChooser`] 和 [`NeverEmptyHandedChooser`] 对其进行包装。
    #[inline]
    pub fn default_chooser() -> Box<dyn Chooser> {
        Box::new(NeverEmptyHandedChooser::<ShuffledChooser<SubnetChooser>>::default())
    }

    /// 获得默认的 [`RequestRetrier`] 实例
    ///
    /// 默认使用 [`ErrorRetrier`]，并使用 [`LimitedRetrier`] 对其进行包装。
    #[inline]
    pub fn default_retrier() -> Box<dyn RequestRetrier> {
        Box::new(LimitedRetrier::<ErrorRetrier>::default())
    }

    /// 获得默认的 [`Backoff`] 实例
    ///
    /// 默认使用 [`ExponentialBackoff`]，并使用 [`LimitedBackoff`] 和 [`RandomizedBackoff`] 对其进行包装。
    #[inline]
    pub fn default_backoff() -> Box<dyn Backoff> {
        Box::new(LimitedBackoff::<RandomizedBackoff<ExponentialBackoff>>::default())
    }
}
