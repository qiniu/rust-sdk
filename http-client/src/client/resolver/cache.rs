use super::{ResolveResult, Resolver, SimpleResolver};
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
    sync::{Arc, Mutex},
    thread::Builder as ThreadBuilder,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const CACHE_SIZE_TO_SHRINK: usize = 100;
const MIN_SHRINK_INTERVAL: Duration = Duration::from_secs(120);
const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub struct CachedResolver<R: Resolver> {
    inner: Arc<CachedResolverInner<R>>,
    lifetime: Duration,
    persistent: Option<PersistentFile>,
}

type Cache = DashMap<Box<str>, CachedResolverValue>;

#[derive(Debug)]
struct CachedResolverInner<R: Resolver> {
    backend: R,
    cache: Cache,
    thread_lock: Mutex<CachedResolverInnerLockedData>,
}

#[derive(Debug)]
struct CachedResolverInnerLockedData {
    last_shrink_timestamp: SystemTime,
    last_refresh_timestamp: SystemTime,
}

#[derive(Debug, Clone)]
struct PersistentFile {
    path: PathBuf,
    auto_persistent: bool,
}

#[derive(Debug, Clone)]
struct CachedResolverValue {
    ip_addrs: Box<[IpAddr]>,
    deadline: SystemTime,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PersistentCache {
    lifetime: Duration,
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
    pub fn new(backend: R, lifetime: Duration) -> Self {
        Self {
            inner: Arc::new(CachedResolverInner {
                backend,
                cache: Default::default(),
                thread_lock: Default::default(),
            }),
            lifetime,
            persistent: None,
        }
    }

    pub fn load_from(
        path: impl Into<PathBuf>,
        auto_persistent: bool,
        backend: R,
    ) -> PersistentResult<Self> {
        let path = path.into();
        let (cache, lifetime) = Self::load_cache_from_persistent_file(&path)
            .map(|cache| cache.into_cache_and_lifetime())?;
        Ok(Self {
            inner: Arc::new(CachedResolverInner {
                backend,
                cache,
                thread_lock: Default::default(),
            }),
            lifetime,
            persistent: Some(PersistentFile {
                path,
                auto_persistent,
            }),
        })
    }

    pub fn load_or_create_from(
        path: impl Into<PathBuf>,
        auto_persistent: bool,
        backend: R,
    ) -> Self {
        let path = path.into();
        let (cache, lifetime) = Self::load_cache_from_persistent_file(&path)
            .map(|cache| cache.into_cache_and_lifetime())
            .unwrap_or_else(|_| (Default::default(), DEFAULT_CACHE_LIFETIME));
        Self {
            inner: Arc::new(CachedResolverInner {
                backend,
                cache,
                thread_lock: Default::default(),
            }),
            lifetime,
            persistent: Some(PersistentFile {
                path,
                auto_persistent,
            }),
        }
    }

    pub fn persistent(&self) -> PersistentResult<()> {
        self.save_cache_into_persistent_file()
    }

    pub fn set_auto_persistent(&mut self, auto_persistent: bool) {
        if let Some(persistent) = &mut self.persistent {
            persistent.auto_persistent = auto_persistent;
        }
    }

    pub fn as_backend(&self) -> &R {
        &self.inner.backend
    }

    fn load_cache_from_persistent_file(path: &Path) -> PersistentResult<PersistentCache> {
        let mut file = OpenOptions::new().read(true).open(path)?;
        let cache = serde_json::from_reader(&mut file)?;
        Ok(cache)
    }

    fn save_cache_into_persistent_file(&self) -> PersistentResult<()> {
        if let Some(persistent) = &self.persistent {
            _save_cache_into_persistent_file(
                self.inner.cache.to_owned(),
                &persistent.path,
                self.lifetime,
            )?;
        }
        Ok(())
    }

    #[inline]
    pub fn persistent_path(&self) -> Option<&Path> {
        self.persistent
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
        &PersistentCache::from_cache_and_lifetime(cache, lifetime),
    )?;
    Ok(())
}

impl Default for CachedResolver<SimpleResolver> {
    #[inline]
    fn default() -> Self {
        Self::load_or_create_from(Self::default_persistent_path(), true, SimpleResolver)
    }
}

impl<R: Resolver> Resolver for CachedResolver<R> {
    #[inline]
    fn resolve(&self, domain: &str) -> ResolveResult {
        let cache_key = domain.to_owned().into_boxed_str();
        if let Some(cache_entry) = self.inner.cache.get(&cache_key) {
            if cache_entry.deadline > SystemTime::now() {
                return Ok(cache_entry.ip_addrs.to_owned());
            }
        }
        let mut resolve_result: Option<ResolveResult> = None;
        let (mut need_to_persistent, mut need_to_refresh) = (false, false);
        macro_rules! resolve_domain {
            () => {
                match self.inner.backend.resolve(domain) {
                    Ok(ip_addrs) => {
                        resolve_result = Some(Ok(ip_addrs.to_owned()));
                        if let Some(persistent) = &self.persistent {
                            if persistent.auto_persistent {
                                need_to_persistent = true;
                            }
                        }
                        Ok(CachedResolverValue::new(ip_addrs, self.lifetime))
                    }
                    Err(err) => {
                        resolve_result = Some(Err(err));
                        Err(())
                    }
                }
            };
        }

        self.inner
            .cache
            .entry(cache_key)
            .and_modify(|cache| {
                resolve_result = Some(Ok(cache.ip_addrs.to_owned()));
                if cache.deadline < SystemTime::now() {
                    need_to_refresh = true;
                }
            })
            .or_try_insert_with(|| resolve_domain!())
            .ok();

        self.do_some_work_async(
            need_to_persistent,
            need_to_refresh,
            self.persistent.as_ref().map(|p| p.path.as_ref()),
        );
        resolve_result.unwrap()
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
        &self,
        need_to_persistent: bool,
        need_to_refresh: bool,
        persistent_path: Option<&Path>,
    ) {
        if need_to_persistent || need_to_refresh {
            let inner = self.inner.to_owned();
            if inner.thread_lock.try_lock().is_err() {
                // Looks like some work is being done, we don't want to spawn another thread
                info!("Resolver cache has hired someone to do the housework, so we don't hire another now");
                return;
            }
            let persistent_path = persistent_path.map(|path| path.to_path_buf());
            let lifetime = self.lifetime;
            ThreadBuilder::new()
                .name("qiniu.rust-sdk.http-client.CachedResolver".into())
                .spawn(move || {
                    if let Ok(mut locked_data) = inner.thread_lock.try_lock() {
                        info!("Resolver cache spawns thread to do some housework");

                        do_some_work_in_thread(
                            &inner,
                            need_to_persistent,
                            need_to_refresh,
                            persistent_path,
                            lifetime,
                            &mut *locked_data,
                        );
                    }
                })
                .ok();
        }

        fn do_some_work_in_thread(
            inner: &CachedResolverInner<impl Resolver>,
            need_to_persistent: bool,
            need_to_refresh: bool,
            persistent_path: Option<PathBuf>,
            lifetime: Duration,
            locked_data: &mut CachedResolverInnerLockedData,
        ) {
            if need_to_refresh {
                refresh_domains(inner, lifetime);
            }
            if need_to_persistent {
                if is_time_to_shrink(inner, locked_data) {
                    shrink_cache(inner);
                }
                if let Some(path) = persistent_path.as_ref() {
                    save_cache_into_persistent_file(inner, path, lifetime);
                }
            }
        }

        fn is_time_to_shrink(
            inner: &CachedResolverInner<impl Resolver>,
            locked_data: &mut CachedResolverInnerLockedData,
        ) -> bool {
            if locked_data.last_shrink_timestamp + MIN_SHRINK_INTERVAL < SystemTime::now()
                && inner.cache.len() >= CACHE_SIZE_TO_SHRINK
            {
                locked_data.last_shrink_timestamp = SystemTime::now();
                return true;
            }
            false
        }

        fn shrink_cache(inner: &CachedResolverInner<impl Resolver>) {
            inner
                .cache
                .retain(|_, cache| cache.deadline >= SystemTime::now());
            info!("Resolver cache is shrink");
        }

        fn refresh_domains(inner: &CachedResolverInner<impl Resolver>, lifetime: Duration) {
            inner.cache.alter_all(|domain, cache| {
                if cache.deadline < SystemTime::now() {
                    if let Ok(ip_addrs) = inner.backend.resolve(domain) {
                        return CachedResolverValue::new(ip_addrs, lifetime);
                    }
                }
                cache
            });
            info!("Expired resolver cache entries are refreshed");
        }

        fn save_cache_into_persistent_file(
            inner: &CachedResolverInner<impl Resolver>,
            persistent_path: &Path,
            lifetime: Duration,
        ) {
            let cache = inner.cache.to_owned();
            match _save_cache_into_persistent_file(cache, persistent_path, lifetime) {
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

#[derive(Error, Debug)]
pub enum PersistentError {
    #[error("I/O error: {0}")]
    IOError(#[from] IOError),

    #[error("JSON serialize/deserialize error: {0}")]
    JSONError(#[from] JSONError),
}
pub type PersistentResult<T> = Result<T, PersistentError>;

impl PersistentCache {
    fn into_cache_and_lifetime(self) -> (Cache, Duration) {
        let cache = DashMap::from_iter(self.cache_entries.into_iter().map(|entry| {
            (
                entry.key,
                CachedResolverValue {
                    ip_addrs: entry.ip_addrs,
                    deadline: entry.deadline,
                },
            )
        }));
        (cache, self.lifetime)
    }

    fn from_cache_and_lifetime(cache: Cache, lifetime: Duration) -> Self {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::{
        collections::HashMap,
        error::Error,
        fs::File,
        io::{BufRead, ErrorKind as IOErrorKind},
        net::Ipv4Addr,
        sync::Arc,
        thread::{sleep, spawn},
    };
    use tap::tap::TapOptional;
    use tempfile::tempdir;

    static LOG_READER: Lazy<Mutex<pipe::PipeReader>> = Lazy::new(|| {
        let (r, w) = pipe::pipe();
        simplelog::WriteLogger::init(
            simplelog::LevelFilter::Info,
            simplelog::Config::default(),
            w,
        )
        .unwrap();
        Mutex::new(r)
    });

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
                .unwrap_or(vec![].into_boxed_slice()))
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
        let resolver = Arc::new(CachedResolver::new(backend, Duration::from_secs(5)));
        let threads_1: Vec<_> = (0..3)
            .map(|_| {
                let resolver = resolver.to_owned();
                spawn(move || {
                    let result = resolver.resolve("test_domain_1.com").unwrap();
                    assert_eq!(
                        result,
                        vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))].into_boxed_slice()
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
                        result,
                        vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))].into_boxed_slice()
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
                        result,
                        vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))].into_boxed_slice()
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
        assert_eq!(resolver.as_backend().resolved("test_domain_1.com"), Some(1));
        assert_eq!(resolver.as_backend().resolved("test_domain_2.com"), Some(1));
        assert_eq!(resolver.as_backend().resolved("test_domain_3.com"), Some(1));
        Ok(())
    }

    #[test]
    fn test_resolver_cache() -> Result<(), Box<dyn Error>> {
        Lazy::force(&LOG_READER);

        let mut backend = ResolverFromTable::default();
        backend.add(
            "test_domain_1.com",
            vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))],
        );
        let resolver = CachedResolver::new(backend, Duration::from_secs(1));

        for _ in 0..5 {
            let result = resolver.resolve("test_domain_1.com").unwrap();
            assert_eq!(
                result,
                vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))].into_boxed_slice()
            );
        }

        sleep(Duration::from_secs(1));

        let result = resolver.resolve("test_domain_1.com").unwrap();
        assert_eq!(
            result,
            vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))].into_boxed_slice()
        );

        loop {
            let mut line = String::new();
            LOG_READER.lock().unwrap().read_line(&mut line)?;
            if line.contains("Expired resolver cache entries are refreshed") {
                break;
            }
        }

        for _ in 0..5 {
            let result = resolver.resolve("test_domain_1.com").unwrap();
            assert_eq!(
                result,
                vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))].into_boxed_slice()
            );
        }

        assert_eq!(resolver.as_backend().resolved("test_domain_1.com"), Some(2));
        Ok(())
    }

    #[test]
    fn test_persistent_resolver() -> Result<(), Box<dyn Error>> {
        Lazy::force(&LOG_READER);

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
                    result,
                    vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))].into_boxed_slice()
                );
            }
            {
                let result = resolver.resolve("test_domain_2.com").unwrap();
                assert_eq!(
                    result,
                    vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))].into_boxed_slice()
                );
            }
            resolver.persistent()?;
            File::open(resolver.persistent_path().unwrap())?;
            {
                let result = resolver.resolve("test_domain_3.com").unwrap();
                assert_eq!(
                    result,
                    vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))].into_boxed_slice()
                );
            }
        }

        {
            let resolver =
                CachedResolver::load_or_create_from(&tempfile_path, true, backend.to_owned());
            {
                let result = resolver.resolve("test_domain_1.com").unwrap();
                assert_eq!(
                    result,
                    vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))].into_boxed_slice()
                );
            }
            {
                let result = resolver.resolve("test_domain_2.com").unwrap();
                assert_eq!(
                    result,
                    vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))].into_boxed_slice()
                );
            }
            {
                let result = resolver.resolve("test_domain_3.com").unwrap();
                assert_eq!(
                    result,
                    vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))].into_boxed_slice()
                );
            }
            assert_eq!(resolver.as_backend().resolved("test_domain_1.com"), None);
            assert_eq!(resolver.as_backend().resolved("test_domain_2.com"), None);
            assert_eq!(resolver.as_backend().resolved("test_domain_3.com"), Some(1));
            loop {
                let mut line = String::new();
                LOG_READER.lock().unwrap().read_line(&mut line)?;
                if line.contains("Resolver cache is persisted automatically") {
                    break;
                }
            }
        }
        {
            let resolver = CachedResolver::load_or_create_from(&tempfile_path, true, backend);
            {
                let result = resolver.resolve("test_domain_1.com").unwrap();
                assert_eq!(
                    result,
                    vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))].into_boxed_slice()
                );
            }
            {
                let result = resolver.resolve("test_domain_2.com").unwrap();
                assert_eq!(
                    result,
                    vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))].into_boxed_slice()
                );
            }
            {
                let result = resolver.resolve("test_domain_3.com").unwrap();
                assert_eq!(
                    result,
                    vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))].into_boxed_slice()
                );
            }
            assert_eq!(resolver.as_backend().resolved("test_domain_1.com"), None);
            assert_eq!(resolver.as_backend().resolved("test_domain_2.com"), None);
            assert_eq!(resolver.as_backend().resolved("test_domain_3.com"), None);
        }

        Ok(())
    }
}
