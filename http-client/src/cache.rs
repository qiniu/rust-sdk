use super::{spawn::spawn, APIResult};
use crossbeam_queue::SegQueue;
use dashmap::DashMap;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Error as JSONError;
use std::{
    borrow::Borrow,
    fmt,
    fs::{create_dir_all, File, OpenOptions},
    hash::Hash,
    io::{BufRead, BufReader, Error as IOError, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime},
};
use thiserror::Error;

pub trait CacheController {
    fn clear(&self);
}

#[derive(Clone, Serialize, Deserialize)]
struct CacheValue<V> {
    value: V,
    cached_at: SystemTime,
}

#[derive(Clone)]
pub(super) struct Cache<K: Eq + PartialEq + Hash + Clone + Serialize, V: Clone + Serialize> {
    inner: Arc<CacheInner<K, V>>,
}

struct CacheInner<K, V> {
    cache_lifetime: Duration,
    shrink_interval: Duration,
    cache: DashMap<K, CacheValue<V>>,
    thread_lock: Mutex<CacheInnerLockedData>,
    persistent: Option<PersistentFile<K, V>>,
    refreshes: DashMap<K, Box<dyn FnMut() -> APIResult<V> + Send + Sync + 'static>>,
}

struct CacheInnerLockedData {
    last_shrink_time: Instant,
}

pub(super) struct PersistentFile<K, V> {
    path: PathBuf,
    auto_persistent: AtomicBool,
    commands: SegQueue<PersistentCacheCommand<K, V>>,
}

impl<K, V> PersistentFile<K, V> {
    #[inline]
    pub(super) fn new(path: PathBuf, auto_persistent: bool) -> Self {
        Self {
            path,
            auto_persistent: auto_persistent.into(),
            commands: Default::default(),
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn auto_persistent(&self) -> bool {
        self.auto_persistent.load(Relaxed)
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Serialize + for<'de> Deserialize<'de>,
        V: Clone + Serialize + for<'de> Deserialize<'de>,
    > Cache<K, V>
{
    #[inline]
    pub(super) fn load_or_create_from(
        path: &Path,
        auto_persistent: bool,
        cache_lifetime: Duration,
        shrink_interval: Duration,
    ) -> PersistentResult<Self> {
        match Self::load_cache_from_persistent_file(
            path,
            auto_persistent,
            cache_lifetime,
            shrink_interval,
        ) {
            Ok(Some(cache)) => Ok(cache),
            _ => {
                let cache = Self::new(
                    cache_lifetime,
                    shrink_interval,
                    Some(PersistentFile::new(path.to_owned(), auto_persistent)),
                );
                cache.persistent_all_cache_entries_to_file()?;
                Ok(cache)
            }
        }
    }

    fn load_cache_from_persistent_file(
        path: &Path,
        auto_persistent: bool,
        cache_lifetime: Duration,
        shrink_interval: Duration,
    ) -> PersistentResult<Option<Self>> {
        let file = BufReader::new(File::open(path)?);
        let cache = DashMap::new();
        for line in file.lines() {
            let entry: PersistentCacheEntry<K, V> = serde_json::from_str(&line?)?;
            if let Some(value) = entry.value {
                if value.cached_at + cache_lifetime >= SystemTime::now() {
                    cache.insert(entry.key, value);
                }
            } else {
                cache.remove(&entry.key);
            }
        }
        Ok(Some(Self {
            inner: Arc::new(CacheInner {
                cache,
                cache_lifetime,
                shrink_interval,
                thread_lock: Default::default(),
                refreshes: Default::default(),
                persistent: Some(PersistentFile::new(path.to_owned(), auto_persistent)),
            }),
        }))
    }
}

impl<K: Eq + PartialEq + Hash + Clone + Serialize, V: Clone + Serialize> Cache<K, V> {
    #[inline]
    pub(super) fn in_memory(cache_lifetime: Duration, shrink_interval: Duration) -> Self {
        Self::new(cache_lifetime, shrink_interval, None)
    }

    #[inline]
    fn new(
        cache_lifetime: Duration,
        shrink_interval: Duration,
        persistent: Option<PersistentFile<K, V>>,
    ) -> Self {
        Self {
            inner: Arc::new(CacheInner {
                cache_lifetime,
                shrink_interval,
                persistent,
                cache: Default::default(),
                refreshes: Default::default(),
                thread_lock: Default::default(),
            }),
        }
    }

    #[inline]
    pub(super) fn persistent_path(&self) -> Option<&Path> {
        self.inner.persistent.as_ref().map(|p| p.path())
    }

    #[inline]
    pub(super) fn auto_persistent(&self) -> Option<bool> {
        self.inner.persistent.as_ref().map(|p| p.auto_persistent())
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Serialize + Sync + Send + 'static,
        V: Clone + Sync + Send + Serialize + 'static,
    > Cache<K, V>
{
    #[inline]
    pub(super) fn get<Q: ?Sized>(
        &self,
        key: &Q,
        mut f: impl FnMut() -> APIResult<V> + Send + Sync + 'static,
    ) -> APIResult<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ToOwned<Owned = K>,
    {
        let cache_result: APIResult<_> =
            self.inner
                .cache
                .entry(key.to_owned())
                .or_try_insert_with(|| {
                    let value = f()?;
                    let cached_at = SystemTime::now();
                    let cache_value = CacheValue { value, cached_at };
                    self.push_command_if_persistent_enabled(|| {
                        PersistentCacheCommand::Append(PersistentCacheEntry {
                            key: key.to_owned(),
                            value: Some(cache_value.to_owned()),
                        })
                    });
                    Ok(cache_value)
                });

        let cache = cache_result?;
        if cache.cached_at + self.inner.cache_lifetime < SystemTime::now() {
            self.inner.refreshes.insert(key.to_owned(), Box::new(f));
        }

        do_some_work_async(&self.inner);
        Ok(cache.value.to_owned())
    }

    #[inline]
    pub(super) fn set(&self, key: K, value: V) {
        let value = CacheValue {
            value,
            cached_at: SystemTime::now(),
        };
        self.push_command_if_persistent_enabled(|| {
            PersistentCacheCommand::Append(PersistentCacheEntry {
                key: key.to_owned(),
                value: Some(value.to_owned()),
            })
        });
        self.inner.cache.insert(key, value);
        do_some_work_async(&self.inner);
    }

    #[inline]
    #[allow(dead_code)]
    pub(super) fn exists(&self, key: &K) -> bool {
        self.inner.cache.contains_key(key)
    }

    #[inline]
    pub(super) fn remove(&self, key: &K) {
        self.inner.cache.remove(key);
        self.push_command_if_persistent_enabled(|| {
            PersistentCacheCommand::Append(PersistentCacheEntry {
                key: key.to_owned(),
                value: None,
            })
        });
        do_some_work_async(&self.inner);
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Serialize + Sync + Send + 'static,
        V: Clone + Sync + Send + Serialize + 'static,
    > CacheController for Cache<K, V>
{
    #[inline]
    fn clear(&self) {
        self.inner.cache.clear();
        if let Some(persistent) = self.inner.persistent.as_ref() {
            if persistent.auto_persistent.load(Relaxed) {
                persistent.commands.push(PersistentCacheCommand::ClearAll);
            }
        }
        self.push_command_if_persistent_enabled(|| PersistentCacheCommand::ClearAll);
        do_some_work_async(&self.inner);
    }
}

impl<K: Eq + PartialEq + Hash + Clone + Serialize, V: Clone + Serialize> Cache<K, V> {
    #[inline]
    fn push_command_if_persistent_enabled(
        &self,
        get_cmd: impl FnOnce() -> PersistentCacheCommand<K, V>,
    ) {
        self.inner.push_command_if_persistent_enabled(get_cmd);
    }

    #[inline]
    fn persistent_all_cache_entries_to_file(&self) -> PersistentResult<()> {
        if let Some(persistent) = &self.inner.persistent {
            _persistent_all_cache_entries_to_file(
                &self.inner.cache,
                &persistent.path,
                self.inner.cache_lifetime,
            )?;
        }
        return Ok(());

        fn _persistent_all_cache_entries_to_file<
            K: Eq + PartialEq + Hash + Clone + Serialize,
            V: Clone + Serialize,
        >(
            cache: &DashMap<K, CacheValue<V>>,
            persistent_path: &Path,
            cache_lifetime: Duration,
        ) -> PersistentResult<()> {
            if let Some(parent_dir) = persistent_path.parent() {
                create_dir_all(parent_dir)?;
            }
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(persistent_path)?;
            for pair in cache.iter() {
                let (key, value) = pair.pair();
                if value.cached_at + cache_lifetime >= SystemTime::now() {
                    let line = serde_json::to_string(&PersistentCacheEntry {
                        key: key.to_owned(),
                        value: Some(value.to_owned()),
                    })?;
                    writeln!(file, "{}", line)?;
                }
            }
            Ok(())
        }
    }
}

impl<K: Eq + PartialEq + Hash + Clone + Serialize, V: Clone + Serialize> fmt::Debug
    for Cache<K, V>
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cache").finish()
    }
}

impl<K: Eq + PartialEq + Hash + Clone + Serialize, V: Clone + Serialize> Drop for Cache<K, V> {
    #[inline]
    fn drop(&mut self) {
        if let Ok(mut locked_data) = self.inner.thread_lock.lock() {
            do_some_work_with_locked_data(&self.inner, &mut *locked_data);
        }
    }
}

impl<K: Eq + PartialEq + Hash + Clone + Serialize, V: Clone + Serialize> CacheInner<K, V> {
    #[inline]
    fn push_command_if_persistent_enabled(
        &self,
        get_cmd: impl FnOnce() -> PersistentCacheCommand<K, V>,
    ) {
        if let Some(persistent) = self.persistent.as_ref() {
            if persistent.auto_persistent.load(Relaxed) {
                persistent.commands.push(get_cmd());
            }
        }
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

#[derive(Clone, Serialize, Deserialize)]
struct PersistentCacheEntry<K, V> {
    key: K,
    value: Option<CacheValue<V>>,
}

enum PersistentCacheCommand<K, V> {
    Append(PersistentCacheEntry<K, V>),
    ClearAll,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum PersistentError {
    #[error("I/O error: {0}")]
    IOError(#[from] IOError),

    #[error("JSON serialize/deserialize error: {0}")]
    JSONError(#[from] JSONError),
}
pub type PersistentResult<T> = Result<T, PersistentError>;

#[inline]
fn do_some_work_async<
    K: Eq + PartialEq + Hash + Clone + Serialize + Sync + Send + 'static,
    V: Clone + Serialize + Sync + Send + 'static,
>(
    inner: &Arc<CacheInner<K, V>>,
) {
    let need_to_shrink;
    let mut need_to_refresh = false;
    let mut need_to_persistent = false;
    let inner = inner.to_owned();
    if let Ok(mut locked_data) = inner.thread_lock.try_lock() {
        need_to_shrink = is_time_to_shrink(&inner, &mut *locked_data, false);
    } else {
        // Looks like some work is being done, we don't want to spawn another thread
        info!("Cache has hired someone to do the housework, so we don't hire another now");
        return;
    }

    if !inner.refreshes.is_empty() {
        need_to_refresh = true;
    }

    if let Some(persistent) = inner.persistent.as_ref() {
        need_to_persistent = !persistent.commands.is_empty();
    }

    if need_to_shrink || need_to_refresh || need_to_persistent {
        if let Err(err) = spawn("qiniu.rust-sdk.http-client.cache.Cache".into(), move || {
            if let Ok(mut locked_data) = inner.thread_lock.try_lock() {
                info!("Cache spawns thread to do some housework");
                do_some_work_with_locked_data(&inner, &mut *locked_data);
            }
        }) {
            warn!(
                "Cache was failed to spawn thread to shrink, refresh and persist cache: {}",
                err
            );
        }
    }
}

#[inline]
fn do_some_work_with_locked_data<
    K: Eq + PartialEq + Hash + Clone + Serialize,
    V: Clone + Serialize,
>(
    inner: &CacheInner<K, V>,
    locked_data: &mut CacheInnerLockedData,
) {
    refresh_cache(inner);

    if is_time_to_shrink(inner, locked_data, true) {
        shrink_cache(inner);
    }

    persistent_to_file(inner);
    return;

    #[inline]
    fn refresh_cache<K: Eq + PartialEq + Hash + Clone + Serialize, V: Clone + Serialize>(
        inner: &CacheInner<K, V>,
    ) {
        inner.refreshes.retain(|key, f| {
            match f() {
                Ok(value) => {
                    let cached_at = SystemTime::now();
                    let cache_value = CacheValue { value, cached_at };
                    inner.push_command_if_persistent_enabled(|| {
                        PersistentCacheCommand::Append(PersistentCacheEntry {
                            key: key.to_owned(),
                            value: Some(cache_value.to_owned()),
                        })
                    });
                    inner.cache.insert(key.to_owned(), cache_value);
                }
                Err(err) => {
                    warn!("Failed to refresh cache: {}", err);
                }
            }
            false
        });
    }

    #[inline]
    fn shrink_cache<K: Eq + PartialEq + Hash, V>(inner: &CacheInner<K, V>) {
        let mut count = 0usize;
        inner.cache.retain(|_, cache| {
            if cache.cached_at + inner.cache_lifetime >= SystemTime::now() {
                true
            } else {
                count += 1;
                false
            }
        });
        if count > 0 {
            inner.cache.shrink_to_fit();
            info!("Cache is shrunken, {} entries are removed", count);
        }
    }

    #[inline]
    fn persistent_to_file<K: Serialize, V: Serialize>(inner: &CacheInner<K, V>) {
        if let Some(persistent) = &inner.persistent {
            if let Err(err) =
                _persistent_to_file(&persistent.commands, &persistent.path, inner.cache_lifetime)
            {
                warn!(
                    "Cache was failed to persist to file {:?}: {}",
                    &persistent.path, err
                );
            }
        }
    }

    #[inline]
    fn _persistent_to_file<K: Serialize, V: Serialize>(
        commands: &SegQueue<PersistentCacheCommand<K, V>>,
        path: &Path,
        cache_lifetime: Duration,
    ) -> PersistentResult<()> {
        if !commands.is_empty() {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(path)?;
            while let Some(command) = commands.pop() {
                match command {
                    PersistentCacheCommand::Append(entry) => {
                        _append_cache_entry_to_file(
                            &mut file,
                            entry.key,
                            entry.value,
                            cache_lifetime,
                        )?;
                    }
                    PersistentCacheCommand::ClearAll => {
                        file.set_len(0)?;
                    }
                }
            }
        }
        Ok(())
    }

    #[inline]
    fn _append_cache_entry_to_file<K: Serialize, V: Serialize>(
        file: &mut File,
        key: K,
        value: Option<CacheValue<V>>,
        cache_lifetime: Duration,
    ) -> PersistentResult<()> {
        if let Some(value) = value {
            if value.cached_at + cache_lifetime >= SystemTime::now() {
                let line = serde_json::to_string(&PersistentCacheEntry::<K, V> {
                    key,
                    value: Some(value),
                })?;
                writeln!(file, "{}", line)?;
            }
        } else {
            let line = serde_json::to_string(&PersistentCacheEntry::<K, V> { key, value: None })?;
            writeln!(file, "{}", line)?;
        }
        Ok(())
    }
}

#[inline]
fn is_time_to_shrink<K, V>(
    inner: &CacheInner<K, V>,
    locked_data: &mut CacheInnerLockedData,
    update_last_shrink_time: bool,
) -> bool {
    if locked_data.last_shrink_time.elapsed() > inner.shrink_interval {
        if update_last_shrink_time {
            locked_data.last_shrink_time = Instant::now();
        }
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{
        super::{ResponseError, ResponseErrorKind},
        *,
    };
    use std::{
        sync::atomic::{AtomicUsize, Ordering::Relaxed},
        thread::sleep,
    };

    #[test]
    fn test_cache_in_memory_refresh() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let cache = Cache::in_memory(Duration::from_secs(2), Duration::from_secs(2));
        let called = Arc::new(AtomicUsize::new(0));
        let cache_key = "key_1".to_owned();
        let cache_value_1 = "val_1".to_owned();
        assert_eq!(
            cache.get(&cache_key, {
                let called = called.to_owned();
                let cache_value_1 = cache_value_1.to_owned();
                move || {
                    called.fetch_add(1, Relaxed);
                    Ok(cache_value_1.to_owned())
                }
            })?,
            cache_value_1,
        );
        assert_eq!(called.load(Relaxed), 1);
        assert!(cache.exists(&cache_key));
        assert_eq!(cache.get(&cache_key, || unreachable!())?, cache_value_1);

        sleep(Duration::from_secs(2));

        let cache_value_2 = "val_2".to_owned();
        assert_eq!(
            cache.get(&cache_key, {
                let called = called.to_owned();
                let cache_value_2 = cache_value_2.to_owned();
                move || {
                    called.fetch_add(1, Relaxed);
                    Ok(cache_value_2.to_owned())
                }
            })?,
            cache_value_1,
        );
        assert_eq!(called.load(Relaxed), 1);
        assert!(cache.exists(&cache_key));

        sleep(Duration::from_secs(1));

        assert_eq!(called.load(Relaxed), 2);
        assert!(cache.exists(&cache_key));
        assert_eq!(cache.get(&cache_key, || unreachable!())?, cache_value_2);

        Ok(())
    }

    #[test]
    fn test_cache_in_memory_shrink() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let cache = Cache::in_memory(Duration::from_secs(2), Duration::from_secs(2));
        let called = Arc::new(AtomicUsize::new(0));
        let cache_key = "key_1".to_owned();
        let cache_value_1 = "val_1".to_owned();
        assert_eq!(
            cache.get(&cache_key, {
                let called = called.to_owned();
                let cache_value_1 = cache_value_1.to_owned();
                move || {
                    called.fetch_add(1, Relaxed);
                    Ok(cache_value_1.to_owned())
                }
            })?,
            cache_value_1,
        );
        assert_eq!(called.load(Relaxed), 1);
        assert!(cache.exists(&cache_key));
        assert_eq!(
            cache.get(&cache_key, || Err(ResponseError::new(
                ResponseErrorKind::NoTry,
                "test error"
            )))?,
            cache_value_1
        );

        sleep(Duration::from_secs(2));
        assert!(cache.exists(&cache_key));
        assert_eq!(
            cache.get(&cache_key, || Err(ResponseError::new(
                ResponseErrorKind::NoTry,
                "test error"
            )))?,
            cache_value_1
        );

        sleep(Duration::from_secs(1));
        assert!(!cache.exists(&cache_key));

        Ok(())
    }
}
