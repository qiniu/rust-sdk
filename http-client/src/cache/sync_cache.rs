use super::{
    super::{spawn::spawn, ApiResult},
    traits::{
        CacheController, CacheInnerLockedData, CacheValue, PersistentCacheCommand, PersistentCacheEntry,
        PersistentFile, PersistentResult,
    },
    IsCacheValid,
};
use crossbeam_queue::SegQueue;
use dashmap::DashMap;
use fs4::FileExt;
use log::{info, warn};
use once_cell::sync::OnceCell;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    borrow::Borrow,
    fmt::{self, Debug},
    fs::{create_dir_all, File, OpenOptions},
    hash::Hash,
    io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write},
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tap::prelude::*;

#[derive(Clone, Debug)]
pub(in super::super) struct Cache<
    K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
    V: IsCacheValid + Clone + Serialize + DeserializeOwned,
> {
    inner: Arc<CacheInner<K, V>>,
}

struct CacheInner<
    K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
    V: IsCacheValid + Clone + Serialize + DeserializeOwned,
> {
    cache_lifetime: Duration,
    shrink_interval: Duration,
    cache: OnceCell<DashMap<K, CacheValue<V>>>,
    locked_data: Mutex<CacheInnerLockedData>,
    persistent: Option<PersistentFile<K, V>>,
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
        V: IsCacheValid + Clone + Debug + Serialize + DeserializeOwned,
    > Cache<K, V>
{
    pub(in super::super) fn load_or_create_from(
        path: &Path,
        auto_persistent: bool,
        cache_lifetime: Duration,
        shrink_interval: Duration,
    ) -> Self {
        Self::new(
            cache_lifetime,
            shrink_interval,
            Some(PersistentFile::new(path.to_owned(), auto_persistent)),
        )
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
        V: IsCacheValid + Clone + Serialize + DeserializeOwned,
    > Cache<K, V>
{
    pub(in super::super) fn in_memory(cache_lifetime: Duration, shrink_interval: Duration) -> Self {
        Self::new(cache_lifetime, shrink_interval, None)
    }

    fn new(cache_lifetime: Duration, shrink_interval: Duration, persistent: Option<PersistentFile<K, V>>) -> Self {
        Self {
            inner: Arc::new(CacheInner {
                cache_lifetime,
                shrink_interval,
                persistent,
                cache: Default::default(),
                locked_data: Default::default(),
            }),
        }
    }

    pub(in super::super) fn persistent_path(&self) -> Option<&Path> {
        self.inner.persistent.as_ref().map(|p| p.path())
    }

    pub(in super::super) fn auto_persistent(&self) -> Option<bool> {
        self.inner.persistent.as_ref().map(|p| p.auto_persistent())
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned + Sync + Send + 'static,
        V: IsCacheValid + Clone + Sync + Send + Serialize + DeserializeOwned + 'static,
    > Cache<K, V>
{
    pub(in super::super) fn get<Q: Hash + Eq + ToOwned<Owned = K> + ?Sized, F: FnOnce() -> ApiResult<V>>(
        &self,
        key: &Q,
        f: F,
    ) -> ApiResult<V>
    where
        K: Borrow<Q>,
    {
        return get(self, key, f).tap(|_| {
            do_some_work_async(&self.inner);
        });

        fn get<
            K: Borrow<Q> + Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned + Sync + Send + 'static,
            V: IsCacheValid + Clone + Sync + Send + Serialize + DeserializeOwned + 'static,
            Q: Hash + Eq + ToOwned<Owned = K> + ?Sized,
            F: FnOnce() -> ApiResult<V>,
        >(
            cache: &Cache<K, V>,
            key: &Q,
            f: F,
        ) -> ApiResult<V> {
            let value_ref = cache.inner.cache().get(key);
            if let Some(old_value) = &value_ref {
                if old_value.is_cache_valid(cache.inner.cache_lifetime) {
                    return Ok(old_value.value().value().to_owned());
                }
            }
            match f() {
                Ok(new_value) => {
                    drop(value_ref);
                    let new_value = CacheValue::new(new_value);
                    cache.inner.cache().insert(key.to_owned(), new_value.to_owned());
                    cache.push_command_if_persistent_enabled(|| {
                        PersistentCacheCommand::Append(PersistentCacheEntry::new(
                            key.to_owned(),
                            Some(new_value.to_owned()),
                        ))
                    });
                    Ok(new_value.into_value())
                }
                Err(err) => value_ref
                    .map(|found_value| Ok(found_value.value().value().to_owned()))
                    .unwrap_or(Err(err)),
            }
        }
    }

    pub(in super::super) fn set(&self, key: K, value: V) {
        let value = CacheValue::new(value);
        self.push_command_if_persistent_enabled(|| {
            PersistentCacheCommand::Append(PersistentCacheEntry::new(key.to_owned(), Some(value.to_owned())))
        });
        self.inner.cache().insert(key, value);
        do_some_work_async(&self.inner);
    }

    #[allow(dead_code)]
    pub(in super::super) fn exists(&self, key: &K) -> bool {
        self.inner.cache().contains_key(key)
    }

    pub(in super::super) fn remove(&self, key: &K) {
        self.inner.cache().remove(key);
        self.push_command_if_persistent_enabled(|| {
            PersistentCacheCommand::Append(PersistentCacheEntry::new(key.to_owned(), None))
        });
        do_some_work_async(&self.inner);
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned + Sync + Send + 'static,
        V: IsCacheValid + Clone + Serialize + DeserializeOwned + Sync + Send + 'static,
    > CacheController for Cache<K, V>
{
    fn clear(&self) {
        self.inner.cache().clear();
        self.push_command_if_persistent_enabled(|| PersistentCacheCommand::ClearAll);
        do_some_work_async(&self.inner);
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
        V: IsCacheValid + Clone + Serialize + DeserializeOwned,
    > Cache<K, V>
{
    fn push_command_if_persistent_enabled(&self, get_cmd: impl FnOnce() -> PersistentCacheCommand<K, V>) {
        self.inner.push_command_if_persistent_enabled(get_cmd);
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
        V: IsCacheValid + Clone + Debug + Serialize + DeserializeOwned,
    > Debug for CacheInner<K, V>
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CacheInner")
            .field("cache", &self.cache)
            .field("cache_lifetime", &self.cache_lifetime)
            .field("shrink_interval", &self.shrink_interval)
            .finish()
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
        V: IsCacheValid + Clone + Serialize + DeserializeOwned,
    > Drop for CacheInner<K, V>
{
    #[inline]
    fn drop(&mut self) {
        if let Ok(mut locked_data) = self.locked_data.lock() {
            do_some_work_with_locked_data(self, &mut locked_data);
        }
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
        V: IsCacheValid + Clone + Serialize + DeserializeOwned,
    > CacheInner<K, V>
{
    fn cache(&self) -> &DashMap<K, CacheValue<V>> {
        return self.cache.get_or_init(|| {
            if let Some(persistent) = &self.persistent {
                load_cache_from_persistent_file(persistent.path(), self.cache_lifetime).unwrap_or_default()
            } else {
                Default::default()
            }
        });

        fn load_cache_from_persistent_file<
            K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
            V: IsCacheValid + Clone + Serialize + DeserializeOwned,
        >(
            path: &Path,
            cache_lifetime: Duration,
        ) -> PersistentResult<DashMap<K, CacheValue<V>>> {
            let mut file = File::open(path)?;
            file.lock_shared()?;
            let cache = DashMap::new();
            for line in BufReader::new(&mut file).lines() {
                let line = line?;
                let entry: PersistentCacheEntry<K, V> = serde_json::from_str(&line)?;
                let (key, value) = entry.into_parts();
                if let Some(value) = value {
                    if value.is_cache_valid(cache_lifetime) {
                        cache.insert(key, value);
                    }
                } else {
                    cache.remove(&key);
                }
            }
            file.unlock()?;
            drop(file);
            Ok(cache)
        }
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
        V: IsCacheValid + Clone + Serialize + DeserializeOwned,
    > CacheInner<K, V>
{
    fn push_command_if_persistent_enabled(&self, get_cmd: impl FnOnce() -> PersistentCacheCommand<K, V>) {
        if let Some(persistent) = self.persistent.as_ref() {
            if persistent.auto_persistent() {
                persistent.commands().push(get_cmd());
            }
        }
    }
}

fn do_some_work_async<
    K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned + Sync + Send + 'static,
    V: IsCacheValid + Clone + Serialize + DeserializeOwned + Sync + Send + 'static,
>(
    inner: &Arc<CacheInner<K, V>>,
) {
    let mut need_to_persistent = false;
    let inner = inner.to_owned();
    let need_to_shrink = if let Ok(mut locked_data) = inner.locked_data.try_lock() {
        is_time_to_shrink(&inner, &mut locked_data, false)
    } else {
        // Looks like some work is being done, we don't want to spawn another thread
        info!("Cache has hired someone to do the housework, so we don't hire another now");
        return;
    };

    if let Some(persistent) = inner.persistent.as_ref() {
        need_to_persistent = !persistent.commands().is_empty();
    }

    if need_to_shrink || need_to_persistent {
        if let Err(err) = spawn("qiniu.rust-sdk.http-client.cache.Cache".into(), move || {
            if let Ok(mut locked_data) = inner.locked_data.try_lock() {
                info!("Cache spawns thread to do some housework");
                do_some_work_with_locked_data(&inner, &mut locked_data);
            }
        }) {
            warn!(
                "Cache was failed to spawn thread to shrink, refresh and persist cache: {}",
                err
            );
        }
    }
}

fn do_some_work_with_locked_data<
    K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
    V: IsCacheValid + Clone + Serialize + DeserializeOwned,
>(
    inner: &CacheInner<K, V>,
    locked_data: &mut CacheInnerLockedData,
) {
    if is_time_to_shrink(inner, locked_data, true) {
        shrink_cache(inner);
    }

    persistent_to_file(inner);
    return;

    fn shrink_cache<
        K: Eq + Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
        V: IsCacheValid + Clone + Serialize + DeserializeOwned,
    >(
        inner: &CacheInner<K, V>,
    ) {
        let mut count = 0usize;
        inner.cache().retain(|_, cache| {
            if cache.is_cache_valid(inner.cache_lifetime) {
                true
            } else {
                count += 1;
                false
            }
        });
        if count > 0 {
            inner.cache().shrink_to_fit();
            info!("Cache is shrunken, {} entries are removed", count);
        }
    }

    fn persistent_to_file<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
        V: IsCacheValid + Clone + Serialize + DeserializeOwned,
    >(
        inner: &CacheInner<K, V>,
    ) {
        if let Some(persistent) = &inner.persistent {
            if let Err(err) = _persistent_to_file(persistent.commands(), persistent.path(), inner.cache_lifetime) {
                warn!(
                    "Cache was failed to persist to file {}: {}",
                    persistent.path().display(),
                    err
                );
            }
        }
    }

    fn _persistent_to_file<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
        V: IsCacheValid + Clone + Serialize + DeserializeOwned,
    >(
        commands: &SegQueue<PersistentCacheCommand<K, V>>,
        path: &Path,
        cache_lifetime: Duration,
    ) -> PersistentResult<()> {
        if !commands.is_empty() {
            if let Some(parent_dir) = path.parent() {
                create_dir_all(parent_dir)?;
            }
            let mut file = OpenOptions::new().create(true).write(true).open(path)?;
            file.lock_exclusive()?;
            file.seek(SeekFrom::End(0))?;
            let mut writer = BufWriter::new(file);
            let result = _execute_commands(commands, &mut writer, cache_lifetime);
            writer.flush()?;
            writer.get_ref().unlock()?;
            drop(writer);
            result?;
            info!("Cache was persisted to file {}", path.display())
        }
        Ok(())
    }

    fn _execute_commands<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
        V: IsCacheValid + Clone + Serialize + DeserializeOwned,
    >(
        commands: &SegQueue<PersistentCacheCommand<K, V>>,
        mut writer: &mut BufWriter<File>,
        cache_lifetime: Duration,
    ) -> PersistentResult<()> {
        while let Some(command) = commands.pop() {
            match command {
                PersistentCacheCommand::Append(entry) => {
                    let (key, value) = entry.into_parts();
                    _append_cache_entry_to_file(&mut writer, key, value, cache_lifetime)?;
                }
                PersistentCacheCommand::ClearAll => {
                    writer.flush()?;
                    writer.get_mut().set_len(0)?;
                }
            }
        }
        Ok(())
    }

    fn _append_cache_entry_to_file<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
        V: IsCacheValid + Clone + Serialize + DeserializeOwned,
    >(
        mut writer: impl Write,
        key: K,
        value: Option<CacheValue<V>>,
        cache_lifetime: Duration,
    ) -> PersistentResult<()> {
        if let Some(value) = value {
            if value.is_cache_valid(cache_lifetime) {
                let line = serde_json::to_string(&PersistentCacheEntry::new(key, Some(value)))?;
                writeln!(writer, "{}", line)?;
            }
        } else {
            let line = serde_json::to_string(&PersistentCacheEntry::<_, V>::new(key, None))?;
            writeln!(writer, "{}", line)?;
        }
        Ok(())
    }
}

fn is_time_to_shrink<
    K: Eq + PartialEq + Hash + Clone + Debug + Serialize + DeserializeOwned,
    V: IsCacheValid + Clone + Serialize + DeserializeOwned,
>(
    inner: &CacheInner<K, V>,
    locked_data: &mut CacheInnerLockedData,
    update_last_shrink_time: bool,
) -> bool {
    if locked_data.last_shrink_time().elapsed() > inner.shrink_interval {
        if update_last_shrink_time {
            *locked_data.last_shrink_time_mut() = Instant::now();
        }
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{
        super::super::{ResponseError, ResponseErrorKind},
        *,
    };
    use serde::Deserialize;
    use std::{
        sync::atomic::{AtomicUsize, Ordering::Relaxed},
        thread::sleep,
    };

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct SimpleCacheValue(String);

    impl IsCacheValid for SimpleCacheValue {}

    #[test]
    fn test_cache_in_memory_refresh() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let cache = Cache::in_memory(Duration::from_secs(2), Duration::from_secs(2));
        let called = Arc::new(AtomicUsize::new(0));
        let cache_key = "key_1".to_owned();
        let cache_value_1 = SimpleCacheValue("val_1".to_owned());
        assert_eq!(
            cache.get(&cache_key, {
                || {
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

        let cache_value_2 = SimpleCacheValue("val_2".to_owned());
        assert_eq!(
            cache.get(&cache_key, {
                || {
                    called.fetch_add(1, Relaxed);
                    Ok(cache_value_2.to_owned())
                }
            })?,
            cache_value_2,
        );
        assert_eq!(called.load(Relaxed), 2);
        assert!(cache.exists(&cache_key));

        sleep(Duration::from_secs(1));

        assert_eq!(cache.get(&cache_key, || unreachable!())?, cache_value_2);

        Ok(())
    }

    #[test]
    fn test_cache_in_memory_shrink() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let cache = Cache::in_memory(Duration::from_secs(2), Duration::from_secs(2));
        let called = Arc::new(AtomicUsize::new(0));
        let cache_key = "key_1".to_owned();
        let cache_value_1 = SimpleCacheValue("val_1".to_owned());
        assert_eq!(
            cache.get(&cache_key, {
                || {
                    called.fetch_add(1, Relaxed);
                    Ok(cache_value_1.to_owned())
                }
            })?,
            cache_value_1,
        );
        assert_eq!(called.load(Relaxed), 1);
        assert!(cache.exists(&cache_key));
        assert_eq!(
            cache.get(&cache_key, || Err(ResponseError::new_with_msg(
                ResponseErrorKind::NoTry,
                "test error"
            )))?,
            cache_value_1
        );

        sleep(Duration::from_secs(2));
        assert!(cache.exists(&cache_key));
        assert_eq!(
            cache.get(&cache_key, || Err(ResponseError::new_with_msg(
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
