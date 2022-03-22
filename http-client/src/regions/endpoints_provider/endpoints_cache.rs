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

#[derive(Debug, Clone)]
pub(super) struct EndpointsCache {
    inner: Cache<CacheKey, Endpoints>,
}

impl EndpointsCache {
    pub(super) fn load_or_create_from(
        path: &Path,
        auto_persistent: bool,
        cache_lifetime: Duration,
        shrink_interval: Duration,
    ) -> Self {
        Self {
            inner: Cache::load_or_create_from(path, auto_persistent, cache_lifetime, shrink_interval),
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
            inner: Cache::in_memory(cache_lifetime, shrink_interval),
        }
    }

    pub(super) fn get(
        &self,
        key: &CacheKey,
        f: impl FnMut() -> ApiResult<Endpoints> + Send + Sync + 'static,
    ) -> ApiResult<Endpoints> {
        self.inner.get(key, f)
    }

    #[allow(dead_code)]
    pub(super) fn set(&self, key: CacheKey, endpoints: Endpoints) {
        self.inner.set(key, endpoints)
    }

    #[allow(dead_code)]
    pub(super) fn remove(&self, key: &CacheKey) {
        self.inner.remove(key)
    }
}

impl CacheController for EndpointsCache {
    #[inline]
    fn clear(&self) {
        self.inner.clear();
    }
}
