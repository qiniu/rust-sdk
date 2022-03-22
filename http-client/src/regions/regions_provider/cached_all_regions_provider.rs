use super::{
    super::{
        super::{ApiResult, CacheController, Endpoints, HttpClient},
        cache_key::CacheKey,
    },
    all_regions_provider::AllRegionsProvider,
    regions_cache::RegionsCache,
    GetOptions, GotRegion, GotRegions, RegionsProvider,
};
use qiniu_credential::CredentialProvider;
use std::{path::Path, time::Duration};

#[cfg(feature = "async")]
use {async_std::task::spawn, futures::future::BoxFuture};

const DEFAULT_SHRINK_INTERVAL: Duration = Duration::from_secs(86400);
const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(86400);

/// 七牛所有区域信息缓存器
#[derive(Clone, Debug)]
pub struct CachedAllRegionsProvider {
    cache_key: CacheKey,
    provider: AllRegionsProvider,
    cache: RegionsCache,
}

impl CachedAllRegionsProvider {
    /// 创建七牛所有区域信息缓存器
    #[inline]
    pub fn new(credential_provider: impl CredentialProvider + 'static) -> Self {
        Self::builder(credential_provider).build()
    }

    /// 构建七牛所有区域信息缓存器
    #[inline]
    pub fn builder(credential_provider: impl CredentialProvider + 'static) -> CachedAllRegionsProviderBuilder {
        CachedAllRegionsProviderBuilder {
            credential_provider: Box::new(credential_provider),
            cache_lifetime: DEFAULT_CACHE_LIFETIME,
            shrink_interval: DEFAULT_SHRINK_INTERVAL,
            uc_endpoints: None,
            http_client: None,
        }
    }
}

impl RegionsProvider for CachedAllRegionsProvider {
    fn get(&self, opts: &GetOptions) -> ApiResult<GotRegion> {
        self.get_all(opts)
            .map(|regions| regions.try_into().expect("Regions API returns empty regions"))
    }

    fn get_all(&self, opts: &GetOptions) -> ApiResult<GotRegions> {
        let provider = self.to_owned();
        let opts = opts.to_owned();
        self.cache
            .get(&self.cache_key, move || provider.provider.get_all(&opts))
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get<'a>(&'a self, opts: &'a GetOptions) -> BoxFuture<'a, ApiResult<GotRegion>> {
        let provider = self.to_owned();
        let opts = opts.to_owned();
        Box::pin(async move { spawn(async move { provider.get(&opts) }).await })
    }

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

/// 七牛所有区域信息缓存构建器
#[derive(Clone, Debug)]
pub struct CachedAllRegionsProviderBuilder {
    cache_lifetime: Duration,
    shrink_interval: Duration,
    http_client: Option<HttpClient>,
    uc_endpoints: Option<Endpoints>,
    credential_provider: Box<dyn CredentialProvider>,
}

impl CachedAllRegionsProviderBuilder {
    /// 缓存时长
    #[inline]
    pub fn cache_lifetime(mut self, cache_lifetime: Duration) -> Self {
        self.cache_lifetime = cache_lifetime;
        self
    }

    /// 清理间隔时长
    #[inline]
    pub fn shrink_interval(mut self, shrink_interval: Duration) -> Self {
        self.shrink_interval = shrink_interval;
        self
    }

    /// 设置 HTTP 客户端
    #[inline]
    pub fn http_client(mut self, http_client: HttpClient) -> Self {
        self.http_client = Some(http_client);
        self
    }

    /// 设置存储空间管理终端地址列表
    #[inline]
    pub fn uc_endpoints(mut self, uc_endpoints: impl Into<Endpoints>) -> Self {
        self.uc_endpoints = Some(uc_endpoints.into());
        self
    }

    /// 从文件系统加载或构建七牛所有区域查询器
    ///
    /// 可以选择是否启用自动持久化缓存功能
    pub fn load_or_create_from(self, path: impl AsRef<Path>, auto_persistent: bool) -> CachedAllRegionsProvider {
        CachedAllRegionsProvider {
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

    /// 从默认文件系统路径加载或构建七牛所有区域查询器，并启用自动持久化缓存功能
    #[inline]
    pub fn build(self) -> CachedAllRegionsProvider {
        self.default_load_or_create_from(true)
    }

    /// 从默认文件系统路径加载或构建七牛所有区域查询器
    ///
    /// 可以选择是否启用自动持久化缓存功能
    pub fn default_load_or_create_from(self, auto_persistent: bool) -> CachedAllRegionsProvider {
        CachedAllRegionsProvider {
            cache: RegionsCache::default_load_or_create_from(
                auto_persistent,
                self.cache_lifetime,
                self.shrink_interval,
            ),
            cache_key: self.new_cache_key(),
            provider: self.new_regions_provider(),
        }
    }

    /// 构建七牛所有区域查询器
    ///
    /// 不启用文件系统持久化缓存
    pub fn in_memory(self) -> CachedAllRegionsProvider {
        CachedAllRegionsProvider {
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

    fn new_regions_provider(self) -> AllRegionsProvider {
        let mut builder = AllRegionsProvider::builder(self.credential_provider);
        if let Some(http_client) = self.http_client {
            builder = builder.http_client(http_client);
        }
        if let Some(uc_endpoints) = self.uc_endpoints {
            builder = builder.uc_endpoints(uc_endpoints);
        }
        builder.build()
    }
}
