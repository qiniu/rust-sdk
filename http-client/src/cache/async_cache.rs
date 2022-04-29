use super::{
    super::ApiResult,
    traits::{
        CacheInnerLockedData, CacheValue, PersistentCacheCommand, PersistentCacheEntry, PersistentFile,
        PersistentResult,
    },
    AsyncCacheController, IsCacheValid,
};
use async_once_cell::Lazy;
use async_std::{
    fs::{create_dir_all, File, OpenOptions},
    io::{BufReader, BufWriter},
    sync::{Mutex, RwLock},
    task::{spawn, spawn_blocking},
};
use crossbeam_queue::SegQueue;
use fs4::async_std::AsyncFileExt;
use futures::{future::BoxFuture, AsyncBufReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt, TryStreamExt};
use log::{info, warn};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::{self, Debug},
    future::Future,
    hash::Hash,
    io::SeekFrom,
    mem::{swap, take},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub(in super::super) struct AsyncCache<
    K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + 'static,
    V: IsCacheValid + Clone + Serialize + Send + Sync + 'static,
> {
    inner: Arc<CacheInner<K, V>>,
}

type RwLockedMap<K, V> = RwLock<HashMap<K, V>>;

struct CacheInner<
    K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + 'static,
    V: IsCacheValid + Clone + Serialize + Send + Sync + 'static,
> {
    cache_lifetime: Duration,
    shrink_interval: Duration,
    cache: Lazy<RwLockedMap<K, CacheValue<V>>>,
    persistent: Option<PersistentFile<K, V>>,
    locked_data: Mutex<CacheInnerLockedData>,
    have_dropped: bool,
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + DeserializeOwned + 'static,
        V: IsCacheValid + Clone + Serialize + Send + Sync + DeserializeOwned + 'static,
    > AsyncCache<K, V>
{
    pub(in super::super) fn load_or_create_from(
        path: &Path,
        auto_persistent: bool,
        cache_lifetime: Duration,
        shrink_interval: Duration,
    ) -> Self {
        let path = path.to_owned();
        return Self::new(
            cache_lifetime,
            shrink_interval,
            Some(PersistentFile::new(path.to_owned(), auto_persistent)),
            async move { load_or_create_from(&path, cache_lifetime).await },
        );

        async fn load_or_create_from<
            K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + DeserializeOwned + 'static,
            V: IsCacheValid + Clone + Serialize + Send + Sync + DeserializeOwned + 'static,
        >(
            path: &Path,
            cache_lifetime: Duration,
        ) -> RwLockedMap<K, CacheValue<V>> {
            load_cache_from_persistent_file(path, cache_lifetime)
                .await
                .unwrap_or_default()
        }

        async fn load_cache_from_persistent_file<
            K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + DeserializeOwned + 'static,
            V: IsCacheValid + Clone + Serialize + Send + Sync + DeserializeOwned + 'static,
        >(
            path: &Path,
            cache_lifetime: Duration,
        ) -> PersistentResult<RwLockedMap<K, CacheValue<V>>> {
            let mut cache = HashMap::new();
            let mut file = File::open(path).await?;
            file = spawn_blocking(move || file.lock_shared().map(|_| file)).await?;
            let mut lines = BufReader::new(&mut file).lines();
            while let Some(line) = lines.try_next().await? {
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
            Ok(RwLock::new(cache))
        }
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + 'static,
        V: IsCacheValid + Clone + Serialize + Send + Sync + 'static,
    > AsyncCache<K, V>
{
    pub(in super::super) fn in_memory(cache_lifetime: Duration, shrink_interval: Duration) -> Self {
        Self::new(cache_lifetime, shrink_interval, None, async move { Default::default() })
    }

    fn new<Fut: Future<Output = RwLockedMap<K, CacheValue<V>>> + Send + 'static>(
        cache_lifetime: Duration,
        shrink_interval: Duration,
        persistent: Option<PersistentFile<K, V>>,
        cache_fut: Fut,
    ) -> Self {
        Self {
            inner: Arc::new(CacheInner {
                cache_lifetime,
                shrink_interval,
                persistent,
                cache: Lazy::new(Box::pin(cache_fut)),
                locked_data: Default::default(),
                have_dropped: false,
            }),
        }
    }

    #[allow(dead_code)]
    pub(in super::super) fn persistent_path(&self) -> Option<&Path> {
        self.inner.persistent.as_ref().map(|p| p.path())
    }

    #[allow(dead_code)]
    pub(in super::super) fn auto_persistent(&self) -> Option<bool> {
        self.inner.persistent.as_ref().map(|p| p.auto_persistent())
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Sync + Send + 'static,
        V: IsCacheValid + Clone + Serialize + Sync + Send + 'static,
    > AsyncCache<K, V>
{
    pub(in super::super) async fn get<Q: Hash + Eq + ToOwned<Owned = K> + ?Sized, Fut: Future<Output = ApiResult<V>>>(
        &self,
        key: &Q,
        fut: Fut,
    ) -> ApiResult<V>
    where
        K: Borrow<Q>,
    {
        let result = get(self, key, fut).await;
        do_some_work_async(&self.inner).await;
        return result;

        async fn get<
            K: Borrow<Q> + Eq + PartialEq + Hash + Clone + Debug + Serialize + Sync + Send + 'static,
            V: IsCacheValid + Clone + Serialize + Sync + Send + 'static,
            Q: Hash + Eq + ToOwned<Owned = K> + ?Sized,
            Fut: Future<Output = ApiResult<V>>,
        >(
            cache: &AsyncCache<K, V>,
            key: &Q,
            fut: Fut,
        ) -> ApiResult<V> {
            let locked_map = cache.inner.cache().await.read().await;
            let value_ref = locked_map.get(key);
            if let Some(old_value) = &value_ref {
                if old_value.is_cache_valid(cache.inner.cache_lifetime) {
                    return Ok(old_value.value().to_owned());
                }
            }
            match fut.await {
                Ok(new_value) => {
                    drop(locked_map);
                    let new_value = CacheValue::new(new_value);
                    cache
                        .inner
                        .cache()
                        .await
                        .write()
                        .await
                        .insert(key.to_owned(), new_value.to_owned());
                    cache.push_command_if_persistent_enabled(|| {
                        PersistentCacheCommand::Append(PersistentCacheEntry::new(
                            key.to_owned(),
                            Some(new_value.to_owned()),
                        ))
                    });
                    Ok(new_value.into_value())
                }
                Err(err) => value_ref
                    .map(|found_value| Ok(found_value.value().to_owned()))
                    .unwrap_or(Err(err)),
            }
        }
    }

    pub(in super::super) async fn set(&self, key: K, value: V) {
        let value = CacheValue::new(value);
        self.push_command_if_persistent_enabled(|| {
            PersistentCacheCommand::Append(PersistentCacheEntry::new(key.to_owned(), Some(value.to_owned())))
        });
        self.inner.cache().await.write().await.insert(key, value);
        do_some_work_async(&self.inner).await;
    }

    #[allow(dead_code)]
    pub(in super::super) async fn exists(&self, key: &K) -> bool {
        self.inner.cache().await.read().await.contains_key(key)
    }

    pub(in super::super) async fn remove(&self, key: &K) {
        self.inner.cache().await.write().await.remove(key);
        self.push_command_if_persistent_enabled(|| {
            PersistentCacheCommand::Append(PersistentCacheEntry::new(key.to_owned(), None))
        });
        do_some_work_async(&self.inner).await;
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Sync + Send + 'static,
        V: IsCacheValid + Clone + Serialize + Sync + Send + 'static,
    > AsyncCacheController for AsyncCache<K, V>
{
    fn async_clear(&self) -> BoxFuture<()> {
        Box::pin(async move {
            self.inner.cache().await.write().await.clear();
            self.push_command_if_persistent_enabled(|| PersistentCacheCommand::ClearAll);
            do_some_work_async(&self.inner).await;
        })
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + 'static,
        V: IsCacheValid + Clone + Serialize + Send + Sync + 'static,
    > AsyncCache<K, V>
{
    fn push_command_if_persistent_enabled(&self, get_cmd: impl FnOnce() -> PersistentCacheCommand<K, V>) {
        self.inner.push_command_if_persistent_enabled(get_cmd);
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + 'static,
        V: IsCacheValid + Clone + Debug + Serialize + Send + Sync + 'static,
    > Debug for CacheInner<K, V>
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("CacheInner");
        if let Some(cache) = self.cache.try_get() {
            d.field("cache", cache);
        } else {
            d.field("cache", &"<uninitialized>");
        }
        d.field("cache_lifetime", &self.cache_lifetime)
            .field("shrink_interval", &self.shrink_interval)
            .finish()
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + 'static,
        V: IsCacheValid + Clone + Serialize + Send + Sync + 'static,
    > CacheInner<K, V>
{
    async fn cache(&self) -> &RwLockedMap<K, CacheValue<V>> {
        self.cache.get().await
    }

    fn push_command_if_persistent_enabled(&self, get_cmd: impl FnOnce() -> PersistentCacheCommand<K, V>) {
        if let Some(persistent) = self.persistent.as_ref() {
            if persistent.auto_persistent() {
                persistent.commands().push(get_cmd());
            }
        }
    }
}

impl<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + 'static,
        V: IsCacheValid + Clone + Serialize + Send + Sync + 'static,
    > Drop for CacheInner<K, V>
{
    #[inline]
    fn drop(&mut self) {
        if self.have_dropped {
            return;
        }
        let mut cache: Lazy<RwLockedMap<K, CacheValue<V>>> = Lazy::new(Box::pin(async move { Default::default() }));
        swap(&mut self.cache, &mut cache);
        let new_cache_inner = CacheInner {
            cache,
            cache_lifetime: self.cache_lifetime,
            shrink_interval: self.shrink_interval,
            locked_data: take(&mut self.locked_data),
            persistent: self.persistent.take(),
            have_dropped: true,
        };
        spawn(async move {
            let mut locked_data = new_cache_inner.locked_data.lock().await;
            do_some_work_with_locked_data(&new_cache_inner, &mut locked_data).await;
        });
    }
}
async fn do_some_work_async<
    K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Sync + Send + 'static,
    V: IsCacheValid + Clone + Serialize + Sync + Send + 'static,
>(
    inner: &Arc<CacheInner<K, V>>,
) {
    let mut need_to_persistent = false;
    let inner = inner.to_owned();
    let need_to_shrink = if let Some(mut locked_data) = inner.locked_data.try_lock() {
        is_time_to_shrink(&inner, &mut locked_data, false)
    } else {
        // Looks like some work is being done, we don't want to spawn another thread
        info!("AsyncCache has hired someone to do the housework, so we don't hire another now");
        return;
    };

    if let Some(persistent) = inner.persistent.as_ref() {
        need_to_persistent = !persistent.commands().is_empty();
    }

    if need_to_shrink || need_to_persistent {
        spawn(async move {
            if let Some(mut locked_data) = inner.locked_data.try_lock() {
                info!("AsyncCache spawns thread to do some housework");
                do_some_work_with_locked_data(&inner, &mut locked_data).await;
            }
        });
    }
}

async fn do_some_work_with_locked_data<
    K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + 'static,
    V: IsCacheValid + Clone + Serialize + Send + Sync + 'static,
>(
    inner: &CacheInner<K, V>,
    locked_data: &mut CacheInnerLockedData,
) {
    if is_time_to_shrink(inner, locked_data, true) {
        shrink_cache(inner).await;
    }

    persistent_to_file(inner).await;

    return;

    async fn shrink_cache<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + 'static,
        V: IsCacheValid + Clone + Serialize + Send + Sync + 'static,
    >(
        inner: &CacheInner<K, V>,
    ) {
        let mut count = 0usize;
        let mut cache = inner.cache().await.write().await;
        cache.retain(|_, cache| {
            if cache.is_cache_valid(inner.cache_lifetime) {
                true
            } else {
                count += 1;
                false
            }
        });
        if count > 0 {
            cache.shrink_to_fit();
            info!("AsyncCache is shrunken, {} entries are removed", count);
        }
    }

    async fn persistent_to_file<
        K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + 'static,
        V: IsCacheValid + Clone + Serialize + Send + Sync + 'static,
    >(
        inner: &CacheInner<K, V>,
    ) {
        if let Some(persistent) = &inner.persistent {
            if let Err(err) = _persistent_to_file(persistent.commands(), persistent.path(), inner.cache_lifetime).await
            {
                warn!(
                    "AsyncCache was failed to persist to file {}: {}",
                    persistent.path().display(),
                    err
                );
            }
        }
    }

    async fn _persistent_to_file<K: Serialize, V: IsCacheValid + Serialize>(
        commands: &SegQueue<PersistentCacheCommand<K, V>>,
        path: &Path,
        cache_lifetime: Duration,
    ) -> PersistentResult<()> {
        if !commands.is_empty() {
            if let Some(parent_dir) = path.parent() {
                create_dir_all(parent_dir).await?;
            }
            let mut file = OpenOptions::new().create(true).write(true).open(path).await?;
            file = spawn_blocking(move || file.lock_exclusive().map(|_| file)).await?;
            file.seek(SeekFrom::End(0)).await?;
            let mut writer = BufWriter::new(file);
            let result = _execute_commands(commands, &mut writer, cache_lifetime).await;
            writer.flush().await?;
            writer.get_mut().unlock()?;
            result?;
            info!("AsyncCache was persisted to file {}", path.display())
        }
        Ok(())
    }

    async fn _execute_commands<K: Serialize, V: IsCacheValid + Serialize>(
        commands: &SegQueue<PersistentCacheCommand<K, V>>,
        mut writer: &mut BufWriter<File>,
        cache_lifetime: Duration,
    ) -> PersistentResult<()> {
        while let Some(command) = commands.pop() {
            match command {
                PersistentCacheCommand::Append(entry) => {
                    let (key, value) = entry.into_parts();
                    _append_cache_entry_to_file(&mut writer, key, value, cache_lifetime).await?;
                }
                PersistentCacheCommand::ClearAll => {
                    writer.flush().await?;
                    writer.get_mut().set_len(0).await?;
                }
            }
        }
        Ok(())
    }

    async fn _append_cache_entry_to_file<K: Serialize, V: IsCacheValid + Serialize>(
        mut writer: impl AsyncWrite + Unpin,
        key: K,
        value: Option<CacheValue<V>>,
        cache_lifetime: Duration,
    ) -> PersistentResult<()> {
        if let Some(value) = value {
            if value.is_cache_valid(cache_lifetime) {
                let line = serde_json::to_string(&PersistentCacheEntry::new(key, Some(value)))?;
                writer.write_all(line.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }
        } else {
            let line = serde_json::to_string(&PersistentCacheEntry::<_, V>::new(key, None))?;
            writer.write_all(line.as_bytes()).await?;
            writer.write_all(b"\n").await?;
        }
        Ok(())
    }
}

fn is_time_to_shrink<
    K: Eq + PartialEq + Hash + Clone + Debug + Serialize + Send + Sync + 'static,
    V: IsCacheValid + Clone + Serialize + Send + Sync + 'static,
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
    use async_std::task::sleep;
    use serde::Deserialize;
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct SimpleCacheValue(String);

    impl IsCacheValid for SimpleCacheValue {}

    #[async_std::test]
    async fn test_async_cache_in_memory_refresh() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let cache = AsyncCache::in_memory(Duration::from_secs(2), Duration::from_secs(2));
        let called = Arc::new(AtomicUsize::new(0));
        let cache_key = "key_1".to_owned();
        let cache_value_1 = SimpleCacheValue("val_1".to_owned());
        assert_eq!(
            cache
                .get(&cache_key, async {
                    called.fetch_add(1, Relaxed);
                    Ok(cache_value_1.to_owned())
                })
                .await?,
            cache_value_1,
        );
        assert_eq!(called.load(Relaxed), 1);
        assert!(cache.exists(&cache_key).await);
        assert_eq!(cache.get(&cache_key, async { unreachable!() }).await?, cache_value_1);

        sleep(Duration::from_secs(2)).await;

        let cache_value_2 = SimpleCacheValue("val_2".to_owned());
        assert_eq!(
            cache
                .get(&cache_key, async {
                    called.fetch_add(1, Relaxed);
                    Ok(cache_value_2.to_owned())
                })
                .await?,
            cache_value_2,
        );
        assert_eq!(called.load(Relaxed), 2);

        sleep(Duration::from_secs(1)).await;

        assert!(cache.exists(&cache_key).await);
        assert_eq!(cache.get(&cache_key, async { unreachable!() }).await?, cache_value_2);

        Ok(())
    }

    #[async_std::test]
    async fn test_async_cache_in_memory_shrink() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let cache = AsyncCache::in_memory(Duration::from_secs(2), Duration::from_secs(2));
        let called = Arc::new(AtomicUsize::new(0));
        let cache_key = "key_1".to_owned();
        let cache_value_1 = SimpleCacheValue("val_1".to_owned());
        assert_eq!(
            cache
                .get(&cache_key, async {
                    called.fetch_add(1, Relaxed);
                    Ok(cache_value_1.to_owned())
                })
                .await?,
            cache_value_1,
        );
        assert_eq!(called.load(Relaxed), 1);
        assert!(cache.exists(&cache_key).await);
        assert_eq!(
            cache
                .get(&cache_key, async {
                    Err(ResponseError::new_with_msg(ResponseErrorKind::NoTry, "test error"))
                })
                .await?,
            cache_value_1
        );

        sleep(Duration::from_secs(2)).await;
        assert!(cache.exists(&cache_key).await);
        assert_eq!(
            cache
                .get(&cache_key, async {
                    Err(ResponseError::new_with_msg(ResponseErrorKind::NoTry, "test error"))
                })
                .await?,
            cache_value_1
        );

        sleep(Duration::from_secs(1)).await;
        assert!(!cache.exists(&cache_key).await);

        Ok(())
    }
}
