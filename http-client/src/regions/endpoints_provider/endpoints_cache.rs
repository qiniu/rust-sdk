use super::{
    super::{
        super::{
            cache::{Cache, CacheController},
            ApiResult,
        },
        cache_key::CacheKey,
    },
    Endpoints,
};
use std::{
    env::temp_dir,
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(feature = "async")]
use {
    super::super::super::cache::{AsyncCache, AsyncCacheController},
    futures::future::BoxFuture,
    std::future::Future,
};

#[derive(Debug, Clone)]
pub(super) struct EndpointsCache {
    cache: Cache<CacheKey, Endpoints>,

    #[cfg(feature = "async")]
    async_cache: AsyncCache<CacheKey, Endpoints>,
}

impl EndpointsCache {
    pub(super) fn load_or_create_from(
        path: &Path,
        auto_persistent: bool,
        cache_lifetime: Duration,
        shrink_interval: Duration,
    ) -> Self {
        Self {
            cache: Cache::load_or_create_from(path, auto_persistent, cache_lifetime, shrink_interval),

            #[cfg(feature = "async")]
            async_cache: AsyncCache::load_or_create_from(path, auto_persistent, cache_lifetime, shrink_interval),
        }
    }

    pub(super) fn default_load_or_create_from(
        auto_persistent: bool,
        cache_lifetime: Duration,
        shrink_interval: Duration,
    ) -> Self {
        Self::load_or_create_from(
            &Self::default_persistent_path(),
            auto_persistent,
            cache_lifetime,
            shrink_interval,
        )
    }

    fn default_persistent_path() -> PathBuf {
        let mut path = dirs::cache_dir().unwrap_or_else(temp_dir);
        path.push(".qiniu-rust-sdk");
        path.push("endpoints-cache.json");
        path
    }

    pub(super) fn in_memory(cache_lifetime: Duration, shrink_interval: Duration) -> Self {
        Self {
            cache: Cache::in_memory(cache_lifetime, shrink_interval),

            #[cfg(feature = "async")]
            async_cache: AsyncCache::in_memory(cache_lifetime, shrink_interval),
        }
    }

    pub(super) fn get(&self, key: &CacheKey, f: impl FnOnce() -> ApiResult<Endpoints>) -> ApiResult<Endpoints> {
        self.cache.get(key, f)
    }

    #[cfg(feature = "async")]
    pub(super) async fn async_get<Fut: Future<Output = ApiResult<Endpoints>>>(
        &self,
        key: &CacheKey,
        fut: Fut,
    ) -> ApiResult<Endpoints> {
        self.async_cache.get(key, fut).await
    }
}

impl CacheController for EndpointsCache {
    #[inline]
    fn clear(&self) {
        self.cache.clear();
    }
}

#[cfg(feature = "async")]
impl AsyncCacheController for EndpointsCache {
    #[inline]
    fn async_clear(&self) -> BoxFuture<()> {
        Box::pin(async move {
            self.async_cache.async_clear().await;
        })
    }
}
