use super::{
    super::super::{APIResult, CacheController, Endpoints, HTTPClient, PersistentResult},
    regions_cache::{CacheKey, RegionsCache},
    regions_provider::RegionsProvider,
    GetOptions, GotRegion, GotRegions, RegionProvider,
};
use qiniu_credential::CredentialProvider;
use std::{any::Any, fmt, path::Path, sync::Arc, time::Duration};

#[cfg(feature = "async")]
use {async_std::task::spawn, futures::future::BoxFuture};

const DEFAULT_SHRINK_INTERVAL: Duration = Duration::from_secs(86400);
const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(86400);

#[derive(Clone)]
pub struct CachedRegionsProvider {
    inner: Arc<CachedRegionsProviderInner>,
}

struct CachedRegionsProviderInner {
    cache_key: CacheKey,
    provider: RegionsProvider,
    cache: RegionsCache,
}

impl CachedRegionsProvider {
    #[inline]
    pub fn builder(
        http_client: HTTPClient,
        uc_endpoints: impl Into<Endpoints>,
        credential_provider: Arc<dyn CredentialProvider>,
    ) -> CachedRegionsProviderBuilder {
        let uc_endpoints = uc_endpoints.into();
        CachedRegionsProviderBuilder {
            cache_key: CacheKey::new_from_endpoint(&uc_endpoints, None),
            provider: RegionsProvider::new(http_client, uc_endpoints, credential_provider),
            cache_lifetime: DEFAULT_CACHE_LIFETIME,
            shrink_interval: DEFAULT_SHRINK_INTERVAL,
        }
    }
}

impl RegionProvider for CachedRegionsProvider {
    fn get(&self, opts: &GetOptions) -> APIResult<GotRegion> {
        self.get_all(opts).map(|regions| {
            regions
                .into_regions()
                .into_iter()
                .next()
                .expect("Regions API returns empty regions")
                .into()
        })
    }

    #[inline]
    fn get_all(&self, opts: &GetOptions) -> APIResult<GotRegions> {
        let provider = self.to_owned();
        let opts = opts.to_owned();
        self.inner
            .cache
            .get(&self.inner.cache_key, move || {
                provider
                    .inner
                    .provider
                    .get_all(&opts)
                    .map(|results| results.into_regions())
            })
            .map(GotRegions::from)
    }

    /// 异步返回七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get<'a>(&'a self, opts: &'a GetOptions) -> BoxFuture<'a, APIResult<GotRegion>> {
        let provider = self.to_owned();
        let opts = opts.to_owned();
        Box::pin(async move { spawn(async move { provider.get(&opts) }).await })
    }

    /// 异步返回多个七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get_all<'a>(&'a self, opts: &'a GetOptions) -> BoxFuture<'a, APIResult<GotRegions>> {
        let provider = self.to_owned();
        let opts = opts.to_owned();
        Box::pin(async move { spawn(async move { provider.get_all(&opts) }).await })
    }

    #[inline]
    fn cache_controller(&self) -> Option<&dyn CacheController> {
        Some(&self.inner.cache)
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_region_provider(&self) -> &dyn RegionProvider {
        self
    }
}

impl fmt::Debug for CachedRegionsProvider {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedRegionsProvider")
            .field("provider", &self.inner.provider)
            .finish()
    }
}

pub struct CachedRegionsProviderBuilder {
    provider: RegionsProvider,
    cache_key: CacheKey,
    cache_lifetime: Duration,
    shrink_interval: Duration,
}

impl fmt::Debug for CachedRegionsProviderBuilder {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedRegionsProviderBuilder")
            .field("provider", &self.provider)
            .field("cache_lifetime", &self.cache_lifetime)
            .field("shrink_interval", &self.shrink_interval)
            .finish()
    }
}

impl CachedRegionsProviderBuilder {
    #[inline]
    pub fn cache_lifetime(mut self, cache_lifetime: Duration) -> Self {
        self.cache_lifetime = cache_lifetime;
        self
    }

    #[inline]
    pub fn shrink_interval(mut self, shrink_interval: Duration) -> Self {
        self.shrink_interval = shrink_interval;
        self
    }

    #[inline]
    pub fn load_or_create_from(
        self,
        path: impl AsRef<Path>,
        auto_persistent: bool,
    ) -> PersistentResult<CachedRegionsProvider> {
        Ok(CachedRegionsProvider {
            inner: Arc::new(CachedRegionsProviderInner {
                provider: self.provider,
                cache_key: self.cache_key,
                cache: RegionsCache::load_or_create_from(
                    path.as_ref(),
                    auto_persistent,
                    self.cache_lifetime,
                    self.shrink_interval,
                )?,
            }),
        })
    }

    #[inline]
    pub fn default_load_or_create_from(
        self,
        auto_persistent: bool,
    ) -> PersistentResult<CachedRegionsProvider> {
        Ok(CachedRegionsProvider {
            inner: Arc::new(CachedRegionsProviderInner {
                provider: self.provider,
                cache_key: self.cache_key,
                cache: RegionsCache::default_load_or_create_from(
                    auto_persistent,
                    self.cache_lifetime,
                    self.shrink_interval,
                )?,
            }),
        })
    }

    #[inline]
    pub fn in_memory(self) -> CachedRegionsProvider {
        CachedRegionsProvider {
            inner: Arc::new(CachedRegionsProviderInner {
                provider: self.provider,
                cache_key: self.cache_key,
                cache: RegionsCache::in_memory(self.cache_lifetime, self.shrink_interval),
            }),
        }
    }
}
