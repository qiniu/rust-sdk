use super::{
    super::super::{ApiResult, CacheController, Endpoints, HttpClient, PersistentResult},
    regions_cache::{CacheKey, RegionsCache},
    regions_provider::RegionsProvider,
    GetOptions, GotRegion, GotRegions, RegionProvider,
};
use qiniu_credential::CredentialProvider;
use std::{fmt, path::Path, sync::Arc, time::Duration};

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
        http_client: HttpClient,
        credential_provider: Arc<dyn CredentialProvider>,
    ) -> CachedRegionsProviderBuilder {
        CachedRegionsProviderBuilder {
            http_client,
            credential_provider,
            cache_lifetime: DEFAULT_CACHE_LIFETIME,
            shrink_interval: DEFAULT_SHRINK_INTERVAL,
            uc_endpoints: Endpoints::public_uc_endpoints().to_owned(),
        }
    }
}

impl RegionProvider for CachedRegionsProvider {
    fn get(&self, opts: &GetOptions) -> ApiResult<GotRegion> {
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
    fn get_all(&self, opts: &GetOptions) -> ApiResult<GotRegions> {
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
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get<'a>(&'a self, opts: &'a GetOptions) -> BoxFuture<'a, ApiResult<GotRegion>> {
        let provider = self.to_owned();
        let opts = opts.to_owned();
        Box::pin(async move { spawn(async move { provider.get(&opts) }).await })
    }

    /// 异步返回多个七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get_all<'a>(&'a self, opts: &'a GetOptions) -> BoxFuture<'a, ApiResult<GotRegions>> {
        let provider = self.to_owned();
        let opts = opts.to_owned();
        Box::pin(async move { spawn(async move { provider.get_all(&opts) }).await })
    }

    #[inline]
    fn cache_controller(&self) -> Option<&dyn CacheController> {
        Some(&self.inner.cache)
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
    // provider: RegionsProvider,
    // cache_key: CacheKey,
    cache_lifetime: Duration,
    shrink_interval: Duration,
    http_client: HttpClient,
    uc_endpoints: Endpoints,
    credential_provider: Arc<dyn CredentialProvider>,
}

impl fmt::Debug for CachedRegionsProviderBuilder {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedRegionsProviderBuilder")
            .field("http_client", &self.http_client)
            .field("uc_endpoints", &self.uc_endpoints)
            .field("credential_provider", &self.credential_provider)
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
                cache: RegionsCache::load_or_create_from(
                    path.as_ref(),
                    auto_persistent,
                    self.cache_lifetime,
                    self.shrink_interval,
                )?,
                cache_key: self.new_cache_key(),
                provider: self.new_regions_provider(),
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
                cache: RegionsCache::default_load_or_create_from(
                    auto_persistent,
                    self.cache_lifetime,
                    self.shrink_interval,
                )?,
                cache_key: self.new_cache_key(),
                provider: self.new_regions_provider(),
            }),
        })
    }

    #[inline]
    pub fn in_memory(self) -> CachedRegionsProvider {
        CachedRegionsProvider {
            inner: Arc::new(CachedRegionsProviderInner {
                cache: RegionsCache::in_memory(self.cache_lifetime, self.shrink_interval),
                cache_key: self.new_cache_key(),
                provider: self.new_regions_provider(),
            }),
        }
    }

    #[inline]
    fn new_cache_key(&self) -> CacheKey {
        CacheKey::new_from_endpoint(&self.uc_endpoints, None)
    }

    #[inline]
    fn new_regions_provider(self) -> RegionsProvider {
        RegionsProvider::new(
            self.http_client,
            self.uc_endpoints,
            self.credential_provider,
        )
    }
}
