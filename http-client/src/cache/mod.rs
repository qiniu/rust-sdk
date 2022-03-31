mod sync_cache;
mod traits;

#[cfg(feature = "async")]
mod async_cache;

pub(crate) use traits::CacheController;

#[cfg(feature = "async")]
pub(crate) use traits::AsyncCacheController;

pub(super) use sync_cache::Cache;
pub(super) use traits::IsCacheValid;

#[cfg(feature = "async")]
pub(super) use async_cache::AsyncCache;
