mod service_name;
pub use service_name::{InvalidServiceName, ServiceName};

mod endpoints;
pub use endpoints::{Endpoints, EndpointsBuilder};

mod regions_provider_endpoints;
pub use regions_provider_endpoints::RegionsProviderEndpoints;

mod bucket_domains_provider;
pub use bucket_domains_provider::{BucketDomainsProvider, BucketDomainsQueryer, BucketDomainsQueryerBuilder};

mod endpoints_cache;

use super::{super::ApiResult, Endpoint};
use auto_impl::auto_impl;
use dyn_clonable::clonable;
use std::{borrow::Cow, fmt, mem::take};

#[cfg(feature = "async")]
type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + 'a + Send>>;

/// 终端地址列表获取接口
///
/// 同时提供阻塞获取接口和异步获取接口，异步获取接口则需要启用 `async` 功能
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait EndpointsProvider: Clone + fmt::Debug + Send + Sync {
    /// 获取终端地址列表
    ///
    /// 该方法的异步版本为 [`Self::async_get_endpoints`]。
    fn get_endpoints<'e>(&'e self, options: GetOptions<'_>) -> ApiResult<Cow<'e, Endpoints>>;

    /// 异步获取终端地址列表
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get_endpoints<'a>(&'a self, options: GetOptions<'a>) -> BoxFuture<'a, ApiResult<Cow<'a, Endpoints>>> {
        Box::pin(async move { self.get_endpoints(options) })
    }
}

/// 获取终端地址列表的选项
#[derive(Copy, Clone, Debug, Default)]
pub struct GetOptions<'a> {
    service_names: &'a [ServiceName],
}

impl<'a> GetOptions<'a> {
    /// 创建获取终端地址列表的选项构建器
    #[inline]
    pub fn builder() -> GetOptionsBuilder<'a> {
        Default::default()
    }

    /// 获取服务列表
    #[inline]
    pub fn service_names(&'a self) -> &'a [ServiceName] {
        self.service_names
    }
}

/// 获取终端地址列表的选项构建器
#[derive(Clone, Debug, Default)]
pub struct GetOptionsBuilder<'a>(GetOptions<'a>);

impl<'a> GetOptionsBuilder<'a> {
    /// 设置服务列表
    #[inline]
    pub fn service_names(&mut self, service_names: &'a [ServiceName]) -> &mut Self {
        self.0.service_names = service_names;
        self
    }

    /// 构建获取终端地址列表的选项
    #[inline]
    pub fn build(&mut self) -> GetOptions<'a> {
        take(&mut self.0)
    }
}

impl EndpointsProvider for Endpoint {
    #[inline]
    fn get_endpoints<'e>(&'e self, _services: GetOptions<'_>) -> ApiResult<Cow<'e, Endpoints>> {
        Ok(Cow::Owned(Endpoints::builder(self.to_owned()).build()))
    }
}

impl EndpointsProvider for Endpoints {
    #[inline]
    fn get_endpoints<'e>(&'e self, _services: GetOptions<'_>) -> ApiResult<Cow<'e, Endpoints>> {
        Ok(Cow::Borrowed(self))
    }
}
