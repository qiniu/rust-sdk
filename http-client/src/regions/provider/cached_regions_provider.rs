use super::{
    super::super::{ApiResult, CacheController, Endpoints, HttpClient},
    cache_key::CacheKey,
    regions_cache::RegionsCache,
    regions_provider::RegionsProvider,
    GetOptions, GotRegion, GotRegions, RegionProvider,
};
use qiniu_credential::CredentialProvider;
use std::{fmt, path::Path, time::Duration};

#[cfg(feature = "async")]
use {async_std::task::spawn, futures::future::BoxFuture};

const DEFAULT_SHRINK_INTERVAL: Duration = Duration::from_secs(86400);
const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(86400);

#[derive(Clone)]
pub struct CachedRegionsProvider {
    cache_key: CacheKey,
    provider: RegionsProvider,
    cache: RegionsCache,
}

impl CachedRegionsProvider {
    #[inline]
    pub fn new(credential_provider: impl CredentialProvider + 'static) -> Self {
        Self::builder(credential_provider).build()
    }

    #[inline]
    pub fn builder(
        credential_provider: impl CredentialProvider + 'static,
    ) -> CachedRegionsProviderBuilder {
        CachedRegionsProviderBuilder {
            credential_provider: Box::new(credential_provider),
            cache_lifetime: DEFAULT_CACHE_LIFETIME,
            shrink_interval: DEFAULT_SHRINK_INTERVAL,
            uc_endpoints: None,
            http_client: None,
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

    fn get_all(&self, opts: &GetOptions) -> ApiResult<GotRegions> {
        let provider = self.to_owned();
        let opts = opts.to_owned();
        self.cache
            .get(&self.cache_key, move || {
                provider
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
        Some(&self.cache)
    }
}

impl fmt::Debug for CachedRegionsProvider {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedRegionsProvider")
            .field("provider", &self.provider)
            .finish()
    }
}

#[derive(Clone)]
pub struct CachedRegionsProviderBuilder {
    cache_lifetime: Duration,
    shrink_interval: Duration,
    http_client: Option<HttpClient>,
    uc_endpoints: Option<Endpoints>,
    credential_provider: Box<dyn CredentialProvider>,
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
    pub fn http_client(mut self, http_client: HttpClient) -> Self {
        self.http_client = Some(http_client);
        self
    }

    #[inline]
    pub fn uc_endpoints(mut self, uc_endpoints: impl Into<Endpoints>) -> Self {
        self.uc_endpoints = Some(uc_endpoints.into());
        self
    }

    pub fn load_or_create_from(
        self,
        path: impl AsRef<Path>,
        auto_persistent: bool,
    ) -> CachedRegionsProvider {
        CachedRegionsProvider {
            cache: RegionsCache::load_or_create_from(
                path.as_ref(),
                auto_persistent,
                self.cache_lifetime,
                self.shrink_interval,
            ),
            cache_key: self.new_cache_key(),
            provider: self.new_regions_provider(),
        }
    }

    #[inline]
    pub fn build(self) -> CachedRegionsProvider {
        self.default_load_or_create_from(true)
    }

    pub fn default_load_or_create_from(self, auto_persistent: bool) -> CachedRegionsProvider {
        CachedRegionsProvider {
            cache: RegionsCache::default_load_or_create_from(
                auto_persistent,
                self.cache_lifetime,
                self.shrink_interval,
            ),
            cache_key: self.new_cache_key(),
            provider: self.new_regions_provider(),
        }
    }

    pub fn in_memory(self) -> CachedRegionsProvider {
        CachedRegionsProvider {
            cache: RegionsCache::in_memory(self.cache_lifetime, self.shrink_interval),
            cache_key: self.new_cache_key(),
            provider: self.new_regions_provider(),
        }
    }

    fn new_cache_key(&self) -> CacheKey {
        CacheKey::new_from_endpoint(
            if let Some(uc_endpoints) = self.uc_endpoints.as_ref() {
                uc_endpoints
            } else {
                Endpoints::public_uc_endpoints()
            },
            None,
        )
    }

    fn new_regions_provider(self) -> RegionsProvider {
        let mut builder = RegionsProvider::builder(self.credential_provider);
        if let Some(http_client) = self.http_client {
            builder = builder.http_client(http_client);
        }
        if let Some(uc_endpoints) = self.uc_endpoints {
            builder = builder.uc_endpoints(uc_endpoints);
        }
        builder.build()
    }
}
