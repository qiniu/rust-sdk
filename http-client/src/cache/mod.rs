mod sync_cache;
mod traits;

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
mod async_cache;

pub(crate) use traits::CacheController;

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
pub(crate) use traits::AsyncCacheController;

pub(super) use sync_cache::Cache;
pub(super) use traits::IsCacheValid;

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
pub(super) use async_cache::AsyncCache;
