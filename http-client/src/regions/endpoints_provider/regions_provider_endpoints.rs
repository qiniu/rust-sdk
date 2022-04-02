use super::{
    super::{super::ApiResult, RegionsProvider},
    Endpoints, EndpointsProvider, GetOptions as EndpointsGetOptions,
};
use std::borrow::Cow;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// 区域终端地址列表获取
///
/// 为一个 [`RegionsProvider`] 实现提供获取终端地址列表的兼容接口
#[derive(Debug, Clone)]
pub struct RegionsProviderEndpoints<R: ?Sized>(R);

impl<R> RegionsProviderEndpoints<R> {
    /// 封装一个 [`RegionsProvider`] 实现以获取终端地址列表的兼容接口
    #[inline]
    pub fn new(region_provider: R) -> Self {
        Self(region_provider)
    }
}

impl<R: RegionsProvider + Clone> EndpointsProvider for RegionsProviderEndpoints<R> {
    #[inline]
    fn get_endpoints<'e>(&'e self, options: EndpointsGetOptions<'_>) -> ApiResult<Cow<'e, Endpoints>> {
        Ok(Cow::Owned(Endpoints::from_region_provider(
            &self.0,
            options.service_names(),
        )?))
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get_endpoints<'a>(
        &'a self,
        options: EndpointsGetOptions<'a>,
    ) -> BoxFuture<'a, ApiResult<Cow<'a, Endpoints>>> {
        Box::pin(async move {
            Ok(Cow::Owned(
                Endpoints::async_from_region_provider(&self.0, options.service_names()).await?,
            ))
        })
    }
}

impl<R> From<R> for RegionsProviderEndpoints<R> {
    #[inline]
    fn from(region_provider: R) -> Self {
        Self::new(region_provider)
    }
}
