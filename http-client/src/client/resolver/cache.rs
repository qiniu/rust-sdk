use super::{super::spawn::spawn, ResolveAnswers, ResolveResult, Resolver};
use dashmap::DashMap;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Error as JSONError;
use std::{
    any::Any,
    env::temp_dir,
    fmt::Debug,
    fs::{create_dir_all, OpenOptions},
    io::Error as IOError,
    iter::FromIterator,
    net::IpAddr,
    path::{Path, PathBuf},
    result::Result,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

#[cfg(feature = "async")]
use {async_std::task::block_on, futures::future::BoxFuture};

const DEFAULT_SHRINK_INTERVAL: Duration = Duration::from_secs(120);
const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(120);

#[derive(Debug, Clone)]
pub struct CachedResolver<R: Resolver> {
    inner: Arc<CachedResolverInner<R>>,
}

type Cache = DashMap<Box<str>, CachedResolverValue>;

#[derive(Debug)]
struct CachedResolverInner<R: Resolver> {
    backend: R,
    cache: Cache,
    thread_lock: Mutex<CachedResolverInnerLockedData>,
    lifetime: Duration,
    shrink_interval: Duration,
    persistent: Option<PersistentFile>,
}

#[derive(Debug)]
struct CachedResolverInnerLockedData {
    last_shrink_timestamp: SystemTime,
    last_refresh_timestamp: SystemTime,
}

#[derive(Debug)]
struct PersistentFile {
    path: PathBuf,
    auto_persistent: AtomicBool,
}

#[derive(Debug, Clone)]
struct CachedResolverValue {
    ip_addrs: Box<[IpAddr]>,
    deadline: SystemTime,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PersistentCache {
    lifetime: Duration,
    shrink_interval: Duration,
    cache_entries: Vec<PersistentCacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistentCacheEntry {
    key: Box<str>,
    ip_addrs: Box<[IpAddr]>,
    deadline: SystemTime,
}

impl<R: Resolver> CachedResolver<R> {
    #[inline]
    pub fn builder(backend: R) -> CachedResolverBuilder<R> {
        CachedResolverBuilder::new(backend)
    }

    pub fn load_from(
        path: impl Into<PathBuf>,
        auto_persistent: bool,
        backend: R,
    ) -> PersistentResult<Self> {
        let path = path.into();
        let (cache, lifetime, shrink_interval) = Self::load_cache_from_persistent_file(&path)
            .map(|cache| cache.into_cache_and_lifetime_and_shrink_interval())?;
        Ok(Self {
            inner: Arc::new(CachedResolverInner {
                backend,
                cache,
                lifetime,
                shrink_interval,
                persistent: Some(PersistentFile {
                    path,
                    auto_persistent: auto_persistent.into(),
                }),
                thread_lock: Default::default(),
            }),
        })
    }

    pub fn load_or_create_from(
        path: impl Into<PathBuf>,
        auto_persistent: bool,
        backend: R,
    ) -> Self {
        let path = path.into();
        let (cache, lifetime, shrink_interval) = Self::load_cache_from_persistent_file(&path)
            .map(|cache| cache.into_cache_and_lifetime_and_shrink_interval())
            .unwrap_or_else(|_| {
                (
                    Default::default(),
                    DEFAULT_CACHE_LIFETIME,
                    DEFAULT_SHRINK_INTERVAL,
                )
            });
        Self {
            inner: Arc::new(CachedResolverInner {
                backend,
                cache,
                lifetime,
                shrink_interval,
                persistent: Some(PersistentFile {
                    path,
                    auto_persistent: auto_persistent.into(),
                }),
                thread_lock: Default::default(),
            }),
        }
    }

    #[inline]
    pub fn default_load_or_create_from(backend: R) -> Self {
        Self::load_or_create_from(Self::default_persistent_path(), true, backend)
    }

    #[inline]
    pub fn persistent(&self) -> PersistentResult<()> {
        self.save_cache_into_persistent_file()
    }

    #[inline]
    pub fn set_auto_persistent(&mut self, auto_persistent: bool) {
        if let Some(persistent) = &self.inner.persistent {
            persistent.auto_persistent.store(auto_persistent, Relaxed);
        }
    }

    fn load_cache_from_persistent_file(path: &Path) -> PersistentResult<PersistentCache> {
        let mut file = OpenOptions::new().read(true).open(path)?;
        let cache = serde_json::from_reader(&mut file)?;
        Ok(cache)
    }

    fn save_cache_into_persistent_file(&self) -> PersistentResult<()> {
        if let Some(persistent) = &self.inner.persistent {
            _save_cache_into_persistent_file(
                self.inner.cache.to_owned(),
                &persistent.path,
                self.inner.lifetime,
                self.inner.shrink_interval,
            )?;
        }
        Ok(())
    }

    #[inline]
    pub fn persistent_path(&self) -> Option<&Path> {
        self.inner
            .persistent
            .as_ref()
            .map(|persistent| persistent.path.as_path())
    }

    #[inline]
    pub fn default_persistent_path() -> PathBuf {
        let mut path = dirs::cache_dir().unwrap_or_else(temp_dir);
        path.push(".qiniu-rust-sdk");
        path.push("resolver-cache.json");
        path
    }
}

fn _save_cache_into_persistent_file(
    cache: Cache,
    persistent_path: &Path,
    lifetime: Duration,
    shrink_interval: Duration,
) -> PersistentResult<()> {
    if let Some(parent_dir) = persistent_path.parent() {
        create_dir_all(parent_dir)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(persistent_path)?;
    serde_json::to_writer(
        &mut file,
        &PersistentCache::from_cache_and_lifetime_and_shrink_interval(
            cache,
            lifetime,
            shrink_interval,
        ),
    )?;
    Ok(())
}

impl<R: Resolver + Default> Default for CachedResolver<R> {
    #[inline]
    fn default() -> Self {
        Self::default_load_or_create_from(R::default())
    }
}

macro_rules! resolve {
    ($domain:expr, $ctx:expr, $resolve_method:ident, $do_some_work_async:path, $blocking_block:ident) => {{
        let cache_key = $domain.to_owned().into_boxed_str();
        if let Some(cache_entry) = $ctx.cache.get(&cache_key) {
            if cache_entry.deadline > SystemTime::now() {
                return Ok(ResolveAnswers::new(cache_entry.ip_addrs.to_owned()));
            }
        }
        let mut resolve_result: Option<ResolveResult> = None;
        let (mut need_to_persistent, mut need_to_refresh) = (false, false);
        macro_rules! resolve_domain {
            () => {
                match $blocking_block!({ $ctx.backend.$resolve_method($domain) }) {
                    Ok(answers) => {
                        resolve_result = Some(Ok(answers.to_owned()));
                        if let Some(persistent) = &$ctx.persistent {
                            if persistent.auto_persistent.load(Relaxed) {
                                need_to_persistent = true;
                            }
                        }
                        Ok(CachedResolverValue::new(
                            answers.into_ip_addrs(),
                            $ctx.lifetime,
                        ))
                    }
                    Err(err) => {
                        resolve_result = Some(Err(err));
                        Err(())
                    }
                }
            };
        }

        $ctx.cache
            .entry(cache_key)
            .and_modify(|cache| {
                resolve_result = Some(Ok(ResolveAnswers::new(cache.ip_addrs.to_owned())));
                if cache.deadline < SystemTime::now() {
                    need_to_refresh = true;
                }
            })
            .or_try_insert_with(|| resolve_domain!())
            .ok();

        $do_some_work_async(
            $ctx,
            need_to_persistent,
            need_to_refresh,
            $ctx.persistent.as_ref().map(|p| p.path.as_ref()),
        );
        resolve_result.unwrap()
    }};
}

macro_rules! sync_block {
    ($block:block) => {{
        $block
    }};
}

#[cfg(feature = "async")]
macro_rules! blocking_async_block {
    ($block:block) => {
        block_on(async { $block.await })
    };
}

impl<R: Resolver> Resolver for CachedResolver<R> {
    #[inline]
    fn resolve(&self, domain: &str) -> ResolveResult {
        resolve!(
            domain,
            &self.inner,
            resolve,
            Self::do_some_work_async,
            sync_block
        )
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_resolve<'a>(&'a self, domain: &'a str) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            resolve!(
                domain,
                &self.inner,
                async_resolve,
                Self::do_some_work_async,
                blocking_async_block
            )
        })
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_resolver(&self) -> &dyn Resolver {
        self
    }
}

impl<R: Resolver> CachedResolver<R> {
    fn do_some_work_async(
        inner: &Arc<CachedResolverInner<R>>,
        need_to_persistent: bool,
        need_to_refresh: bool,
        persistent_path: Option<&Path>,
    ) {
        if need_to_persistent || need_to_refresh {
            let inner = inner.to_owned();
            if inner.thread_lock.try_lock().is_err() {
                // Looks like some work is being done, we don't want to spawn another thread
                info!("Resolver cache has hired someone to do the housework, so we don't hire another now");
                return;
            }
            let persistent_path = persistent_path.map(|path| path.to_path_buf());
            if let Err(err) = spawn(
                "qiniu.rust-sdk.http-client.resolver.CachedResolver".into(),
                move || {
                    if let Ok(mut locked_data) = inner.thread_lock.try_lock() {
                        info!("Resolver cache spawns thread to do some housework");

                        do_some_work_in_thread(
                            &inner,
                            need_to_persistent,
                            need_to_refresh,
                            persistent_path,
                            &mut *locked_data,
                        );
                    }
                },
            ) {
                warn!(
                    "Resolver cache was failed to spawn thread to resolve domain: {}",
                    err
                );
            }
        }

        fn do_some_work_in_thread(
            inner: &CachedResolverInner<impl Resolver>,
            need_to_persistent: bool,
            need_to_refresh: bool,
            persistent_path: Option<PathBuf>,
            locked_data: &mut CachedResolverInnerLockedData,
        ) {
            if need_to_refresh {
                refresh_domains(inner);
            }
            if need_to_persistent {
                if is_time_to_shrink(inner, locked_data) {
                    shrink_cache(inner);
                }
                if let Some(path) = persistent_path.as_ref() {
                    save_cache_into_persistent_file(inner, path);
                }
            }
        }

        #[inline]
        fn is_time_to_shrink(
            inner: &CachedResolverInner<impl Resolver>,
            locked_data: &mut CachedResolverInnerLockedData,
        ) -> bool {
            if locked_data.last_shrink_timestamp + inner.shrink_interval < SystemTime::now() {
                locked_data.last_shrink_timestamp = SystemTime::now();
                return true;
            }
            false
        }

        #[inline]
        fn shrink_cache(inner: &CachedResolverInner<impl Resolver>) {
            inner
                .cache
                .retain(|_, cache| cache.deadline >= SystemTime::now());
            info!("Resolver cache is shrunken");
        }

        #[inline]
        fn refresh_domains(inner: &CachedResolverInner<impl Resolver>) {
            inner.cache.alter_all(|domain, cache| {
                if cache.deadline < SystemTime::now() {
                    if let Ok(answers) = inner.backend.resolve(domain) {
                        return CachedResolverValue::new(answers.into_ip_addrs(), inner.lifetime);
                    }
                }
                cache
            });
            info!("Expired resolver cache entries are refreshed");
        }

        #[inline]
        fn save_cache_into_persistent_file(
            inner: &CachedResolverInner<impl Resolver>,
            persistent_path: &Path,
        ) {
            let cache = inner.cache.to_owned();
            match _save_cache_into_persistent_file(
                cache,
                persistent_path,
                inner.lifetime,
                inner.shrink_interval,
            ) {
                Ok(_) => info!("Resolver cache is persisted automatically"),
                Err(err) => warn!("Resolver cache persist error: {}", err),
            }
        }
    }
}

impl Default for CachedResolverInnerLockedData {
    #[inline]
    fn default() -> Self {
        Self {
            last_shrink_timestamp: UNIX_EPOCH,
            last_refresh_timestamp: UNIX_EPOCH,
        }
    }
}

impl CachedResolverValue {
    #[inline]
    fn new(ip_addrs: Box<[IpAddr]>, lifetime: Duration) -> Self {
        CachedResolverValue {
            ip_addrs,
            deadline: SystemTime::now() + lifetime,
        }
    }
}

#[derive(Debug)]
pub struct CachedResolverBuilder<R: Resolver> {
    inner: CachedResolverInner<R>,
}

impl<R: Resolver> CachedResolverBuilder<R> {
    #[inline]
    pub fn new(backend: R) -> Self {
        Self {
            inner: CachedResolverInner {
                backend,
                cache: Default::default(),
                thread_lock: Default::default(),
                lifetime: DEFAULT_CACHE_LIFETIME,
                shrink_interval: DEFAULT_SHRINK_INTERVAL,
                persistent: None,
            },
        }
    }

    #[inline]
    pub fn lifetime(mut self, lifetime: Duration) -> Self {
        self.inner.lifetime = lifetime;
        self
    }

    #[inline]
    pub fn shrink_interval(mut self, shrink_interval: Duration) -> Self {
        self.inner.shrink_interval = shrink_interval;
        self
    }

    #[inline]
    pub fn build(self) -> CachedResolver<R> {
        CachedResolver {
            inner: Arc::new(self.inner),
        }
    }
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

impl PersistentCache {
    #[inline]
    fn into_cache_and_lifetime_and_shrink_interval(self) -> (Cache, Duration, Duration) {
        let cache = DashMap::from_iter(self.cache_entries.into_iter().map(|entry| {
            (
                entry.key,
                CachedResolverValue {
                    ip_addrs: entry.ip_addrs,
                    deadline: entry.deadline,
                },
            )
        }));
        (cache, self.lifetime, self.shrink_interval)
    }

    #[inline]
    fn from_cache_and_lifetime_and_shrink_interval(
        cache: Cache,
        lifetime: Duration,
        shrink_interval: Duration,
    ) -> Self {
        PersistentCache {
            cache_entries: cache
                .into_iter()
                .map(|(key, value)| PersistentCacheEntry {
                    key,
                    ip_addrs: value.ip_addrs,
                    deadline: value.deadline,
                })
                .collect(),
            lifetime,
            shrink_interval,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        error::Error,
        fs::File,
        io::ErrorKind as IOErrorKind,
        net::Ipv4Addr,
        sync::Arc,
        thread::{sleep, spawn},
    };
    use tap::tap::TapOptional;
    use tempfile::tempdir;

    #[derive(Debug, Clone, Default)]
    struct ResolverFromTable {
        table: HashMap<Box<str>, Box<[IpAddr]>>,
        resolved: DashMap<Box<str>, usize>,
    }

    impl ResolverFromTable {
        fn add(&mut self, domain: impl Into<String>, ip_addrs: Vec<IpAddr>) {
            self.table
                .insert(domain.into().into_boxed_str(), ip_addrs.into_boxed_slice());
        }

        fn resolved(&self, domain: impl AsRef<str>) -> Option<usize> {
            self.resolved.get(domain.as_ref()).map(|v| *v)
        }
    }

    impl Resolver for ResolverFromTable {
        #[inline]
        fn resolve(&self, domain: &str) -> ResolveResult {
            let key = domain.to_owned().into_boxed_str();
            Ok(self
                .table
                .get(&key)
                .tap_some(|_| {
                    self.resolved
                        .entry(key)
                        .and_modify(|resolved| *resolved = *resolved + 1)
                        .or_insert(1);
                })
                .cloned()
                .map(ResolveAnswers::new)
                .unwrap_or_default())
        }

        #[inline]
        fn as_any(&self) -> &dyn Any {
            self
        }

        #[inline]
        fn as_resolver(&self) -> &dyn Resolver {
            self
        }
    }

    #[test]
    fn test_thread_safe_cached_resolver() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let mut backend = ResolverFromTable::default();
        backend.add(
            "test_domain_1.com",
            vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))],
        );
        backend.add(
            "test_domain_2.com",
            vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))],
        );
        backend.add(
            "test_domain_3.com",
            vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))],
        );
        let resolver = Arc::new(
            CachedResolver::builder(backend)
                .lifetime(Duration::from_secs(5))
                .build(),
        );
        let threads_1: Vec<_> = (0..3)
            .map(|_| {
                let resolver = resolver.to_owned();
                spawn(move || {
                    let result = resolver.resolve("test_domain_1.com").unwrap();
                    assert_eq!(
                        result.ip_addrs(),
                        &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]
                    );
                })
            })
            .collect();
        let threads_2: Vec<_> = (0..5)
            .map(|_| {
                let resolver = resolver.to_owned();
                spawn(move || {
                    let result = resolver.resolve("test_domain_2.com").unwrap();
                    assert_eq!(
                        result.ip_addrs(),
                        &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))]
                    );
                })
            })
            .collect();
        let threads_3: Vec<_> = (0..7)
            .map(|_| {
                let resolver = resolver.to_owned();
                spawn(move || {
                    let result = resolver.resolve("test_domain_3.com").unwrap();
                    assert_eq!(
                        result.ip_addrs(),
                        &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))]
                    );
                })
            })
            .collect();
        threads_1
            .into_iter()
            .chain(threads_2.into_iter())
            .chain(threads_3.into_iter())
            .try_for_each(|thread| thread.join())
            .unwrap();
        let resolver = Arc::try_unwrap(resolver).unwrap();
        assert_eq!(
            resolver.inner.backend.resolved("test_domain_1.com"),
            Some(1)
        );
        assert_eq!(
            resolver.inner.backend.resolved("test_domain_2.com"),
            Some(1)
        );
        assert_eq!(
            resolver.inner.backend.resolved("test_domain_3.com"),
            Some(1)
        );
        Ok(())
    }

    #[test]
    fn test_resolver_cache() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let mut backend = ResolverFromTable::default();
        backend.add(
            "test_domain_1.com",
            vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))],
        );
        let resolver = CachedResolver::builder(backend)
            .lifetime(Duration::from_secs(1))
            .build();

        for _ in 0..5 {
            let result = resolver.resolve("test_domain_1.com").unwrap();
            assert_eq!(
                result.ip_addrs(),
                &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]
            );
        }

        sleep(Duration::from_secs(1));

        let result = resolver.resolve("test_domain_1.com").unwrap();
        assert_eq!(
            result.ip_addrs(),
            &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]
        );

        for _ in 0..5 {
            let result = resolver.resolve("test_domain_1.com").unwrap();
            assert_eq!(
                result.ip_addrs(),
                &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]
            );
        }

        assert_eq!(
            resolver.inner.backend.resolved("test_domain_1.com"),
            Some(2)
        );
        Ok(())
    }

    #[test]
    fn test_persistent_resolver() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let mut backend = ResolverFromTable::default();
        backend.add(
            "test_domain_1.com",
            vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))],
        );
        backend.add(
            "test_domain_2.com",
            vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))],
        );
        backend.add(
            "test_domain_3.com",
            vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))],
        );

        let tempdir = tempdir()?;
        let tempfile_path = {
            let mut path = tempdir.path().to_owned();
            path.push("resolve_result");
            path
        };
        {
            let resolver =
                CachedResolver::load_or_create_from(&tempfile_path, false, backend.to_owned());
            {
                let err = File::open(resolver.persistent_path().unwrap()).unwrap_err();
                assert_eq!(err.kind(), IOErrorKind::NotFound);
            }
            {
                let result = resolver.resolve("test_domain_1.com").unwrap();
                assert_eq!(
                    result.ip_addrs(),
                    &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]
                );
            }
            {
                let result = resolver.resolve("test_domain_2.com").unwrap();
                assert_eq!(
                    result.ip_addrs(),
                    &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))]
                );
            }
            resolver.persistent()?;
            File::open(resolver.persistent_path().unwrap())?;
            {
                let result = resolver.resolve("test_domain_3.com").unwrap();
                assert_eq!(
                    result.ip_addrs(),
                    &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))]
                );
            }
        }

        sleep(Duration::from_secs(1));

        {
            let resolver =
                CachedResolver::load_or_create_from(&tempfile_path, true, backend.to_owned());
            {
                let result = resolver.resolve("test_domain_1.com").unwrap();
                assert_eq!(
                    result.ip_addrs(),
                    &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]
                );
            }
            {
                let result = resolver.resolve("test_domain_2.com").unwrap();
                assert_eq!(
                    result.ip_addrs(),
                    &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))]
                );
            }
            {
                let result = resolver.resolve("test_domain_3.com").unwrap();
                assert_eq!(
                    result.ip_addrs(),
                    &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))]
                );
            }
            assert_eq!(resolver.inner.backend.resolved("test_domain_1.com"), None);
            assert_eq!(resolver.inner.backend.resolved("test_domain_2.com"), None);
            assert_eq!(
                resolver.inner.backend.resolved("test_domain_3.com"),
                Some(1)
            );
        }

        sleep(Duration::from_secs(1));

        {
            let resolver = CachedResolver::load_or_create_from(&tempfile_path, true, backend);
            {
                let result = resolver.resolve("test_domain_1.com").unwrap();
                assert_eq!(
                    result.ip_addrs(),
                    &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]
                );
            }
            {
                let result = resolver.resolve("test_domain_2.com").unwrap();
                assert_eq!(
                    result.ip_addrs(),
                    &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))]
                );
            }
            {
                let result = resolver.resolve("test_domain_3.com").unwrap();
                assert_eq!(
                    result.ip_addrs(),
                    &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))]
                );
            }
            assert_eq!(resolver.inner.backend.resolved("test_domain_1.com"), None);
            assert_eq!(resolver.inner.backend.resolved("test_domain_2.com"), None);
            assert_eq!(resolver.inner.backend.resolved("test_domain_3.com"), None);
        }

        Ok(())
    }
}
