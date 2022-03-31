use auto_impl::auto_impl;
use crossbeam_queue::SegQueue;
use serde::{Deserialize, Serialize};
use serde_json::Error as JsonError;
use std::{
    io::Error as IoError,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering::Relaxed},
    time::{Duration, Instant, SystemTime},
};
use thiserror::Error;

pub(in super::super) trait IsCacheValid {
    fn is_valid(&self) -> bool {
        true
    }
}

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub(crate) trait CacheController {
    fn clear(&self);
}

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[auto_impl(&, &mut, Box, Rc, Arc)]
#[cfg(feature = "async")]
pub(crate) trait AsyncCacheController {
    fn async_clear(&self) -> BoxFuture<()>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct CacheValue<V> {
    value: V,
    cached_at: SystemTime,
}

impl<V: IsCacheValid> CacheValue<V> {
    pub(super) fn new(value: V) -> Self {
        Self {
            value,
            cached_at: SystemTime::now(),
        }
    }

    pub(super) fn value(&self) -> &V {
        &self.value
    }

    pub(super) fn into_value(self) -> V {
        self.value
    }

    pub(super) fn is_cache_valid(&self, cache_lifetime: Duration) -> bool {
        self.cached_at + cache_lifetime >= SystemTime::now() && self.value.is_valid()
    }
}

impl<V: IsCacheValid> IsCacheValid for CacheValue<V> {
    fn is_valid(&self) -> bool {
        self.value.is_valid()
    }
}

#[derive(Debug)]
pub(super) struct CacheInnerLockedData {
    last_shrink_time: Instant,
}

pub(super) struct PersistentFile<K, V> {
    path: PathBuf,
    auto_persistent: AtomicBool,
    commands: SegQueue<PersistentCacheCommand<K, V>>,
}

impl<K, V> PersistentFile<K, V> {
    pub(super) fn new(path: PathBuf, auto_persistent: bool) -> Self {
        Self {
            path,
            auto_persistent: auto_persistent.into(),
            commands: Default::default(),
        }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn auto_persistent(&self) -> bool {
        self.auto_persistent.load(Relaxed)
    }

    pub(super) fn commands(&self) -> &SegQueue<PersistentCacheCommand<K, V>> {
        &self.commands
    }
}

impl CacheInnerLockedData {
    pub(super) fn last_shrink_time(&self) -> Instant {
        self.last_shrink_time
    }

    pub(super) fn last_shrink_time_mut(&mut self) -> &mut Instant {
        &mut self.last_shrink_time
    }
}

impl Default for CacheInnerLockedData {
    #[inline]
    fn default() -> Self {
        Self {
            last_shrink_time: Instant::now(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct PersistentCacheEntry<K, V> {
    key: K,
    value: Option<CacheValue<V>>,
}

impl<K, V> PersistentCacheEntry<K, V> {
    pub(super) fn new(key: K, value: Option<CacheValue<V>>) -> Self {
        Self { key, value }
    }

    pub(super) fn into_parts(self) -> (K, Option<CacheValue<V>>) {
        (self.key, self.value)
    }
}

pub(super) enum PersistentCacheCommand<K, V> {
    Append(PersistentCacheEntry<K, V>),
    ClearAll,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub(super) enum PersistentError {
    #[error("I/O error: {0}")]
    IoError(#[from] IoError),

    #[error("JSON serialize/deserialize error: {0}")]
    JsonError(#[from] JsonError),
}
pub(super) type PersistentResult<T> = Result<T, PersistentError>;
