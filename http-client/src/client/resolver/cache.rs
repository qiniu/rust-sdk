use super::{
    super::super::cache::{Cache, PersistentResult},
    ResolveAnswers, ResolveResult, Resolver,
};
use std::{
    any::Any,
    env::temp_dir,
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

#[cfg(feature = "async")]
use {async_std::task::spawn, futures::future::BoxFuture};

const DEFAULT_SHRINK_INTERVAL: Duration = Duration::from_secs(120);
const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub struct CachedResolver<R> {
    resolver: Arc<R>,
    cache: Cache<String, ResolveAnswers>,
}

impl<R> CachedResolver<R> {
    #[inline]
    pub fn builder(backend: R) -> CachedResolverBuilder<R> {
        CachedResolverBuilder::new(backend)
    }

    #[inline]
    pub fn persistent_path(&self) -> Option<&Path> {
        self.cache.persistent_path()
    }

    #[inline]
    pub fn auto_persistent(&self) -> Option<bool> {
        self.cache.auto_persistent()
    }

    #[inline]
    pub fn default_persistent_path() -> PathBuf {
        let mut path = dirs::cache_dir().unwrap_or_else(temp_dir);
        path.push(".qiniu-rust-sdk");
        path.push("resolver-cache.json");
        path
    }
}

impl<R: Default> Default for CachedResolver<R> {
    #[inline]
    fn default() -> Self {
        Self::builder(R::default())
            .default_load_or_create_from(true)
            .unwrap_or_else(|_| Self::builder(R::default()).in_memory())
    }
}

impl<R> Clone for CachedResolver<R> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            resolver: self.resolver.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl<R: Resolver> Resolver for CachedResolver<R> {
    #[inline]
    fn resolve(&self, domain: &str) -> ResolveResult {
        let resolver = self.resolver.to_owned();
        self.cache.get(domain, {
            let domain = domain.to_owned();
            move || resolver.resolve(&domain)
        })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_resolve<'a>(&'a self, domain: &'a str) -> BoxFuture<'a, ResolveResult> {
        let resolver = self.to_owned();
        let domain = domain.to_owned();
        Box::pin(async move { spawn(async move { resolver.resolve(&domain) }).await })
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

#[derive(Debug)]
pub struct CachedResolverBuilder<R> {
    resolver: R,
    cache_lifetime: Duration,
    shrink_interval: Duration,
}

impl<R> CachedResolverBuilder<R> {
    #[inline]
    pub fn new(resolver: R) -> Self {
        Self {
            resolver,
            cache_lifetime: DEFAULT_CACHE_LIFETIME,
            shrink_interval: DEFAULT_SHRINK_INTERVAL,
        }
    }

    #[inline]
    pub fn cache_lifetime(mut self, cache_lifetime: Duration) -> Self {
        self.cache_lifetime = cache_lifetime;
        self
    }

    #[inline]
    pub fn shrink_interval(mut self, shrink_interval: Duration) -> Self {
        self.shrink_interval = shrink_interval;
        self
    }

    #[inline]
    pub fn load_or_create_from(
        self,
        path: impl AsRef<Path>,
        auto_persistent: bool,
    ) -> PersistentResult<CachedResolver<R>> {
        Ok(CachedResolver {
            resolver: Arc::new(self.resolver),
            cache: Cache::load_or_create_from(
                path.as_ref(),
                auto_persistent,
                self.cache_lifetime,
                self.shrink_interval,
            )?,
        })
    }

    #[inline]
    pub fn default_load_or_create_from(
        self,
        auto_persistent: bool,
    ) -> PersistentResult<CachedResolver<R>> {
        Ok(CachedResolver {
            resolver: Arc::new(self.resolver),
            cache: Cache::load_or_create_from(
                &CachedResolver::<R>::default_persistent_path(),
                auto_persistent,
                self.cache_lifetime,
                self.shrink_interval,
            )?,
        })
    }

    #[inline]
    pub fn in_memory(self) -> CachedResolver<R> {
        CachedResolver {
            resolver: Arc::new(self.resolver),
            cache: Cache::in_memory(self.cache_lifetime, self.shrink_interval),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashmap::DashMap;
    use std::{
        collections::HashMap,
        error::Error,
        fs::File,
        net::{IpAddr, Ipv4Addr},
        sync::Arc,
        thread::{sleep, spawn},
    };
    use tap::tap::TapOptional;
    use tempfile::tempdir;

    #[derive(Debug, Clone, Default)]
    struct ResolverFromTable {
        table: HashMap<String, Box<[IpAddr]>>,
        resolved: DashMap<String, usize>,
    }

    impl ResolverFromTable {
        fn add(&mut self, domain: impl Into<String>, ip_addrs: Vec<IpAddr>) {
            self.table
                .insert(domain.into(), ip_addrs.into_boxed_slice());
        }

        fn resolved(&self, domain: impl AsRef<str>) -> Option<usize> {
            self.resolved.get(domain.as_ref()).map(|v| *v)
        }
    }

    impl Resolver for ResolverFromTable {
        #[inline]
        fn resolve(&self, domain: &str) -> ResolveResult {
            let key = domain.to_owned();
            Ok(self
                .table
                .get(&key)
                .tap_some(|_| {
                    self.resolved
                        .entry(key)
                        .and_modify(|resolved| *resolved += 1)
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
                .cache_lifetime(Duration::from_secs(5))
                .in_memory(),
        );
        let threads_1 = (0..3).map(|_| {
            let resolver = resolver.to_owned();
            spawn(move || {
                let result = resolver.resolve("test_domain_1.com").unwrap();
                assert_eq!(
                    result.ip_addrs(),
                    &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]
                );
            })
        });
        let threads_2 = (0..5).map(|_| {
            let resolver = resolver.to_owned();
            spawn(move || {
                let result = resolver.resolve("test_domain_2.com").unwrap();
                assert_eq!(
                    result.ip_addrs(),
                    &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))]
                );
            })
        });
        let threads_3 = (0..7).map(|_| {
            let resolver = resolver.to_owned();
            spawn(move || {
                let result = resolver.resolve("test_domain_3.com").unwrap();
                assert_eq!(
                    result.ip_addrs(),
                    &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))]
                );
            })
        });
        threads_1
            .into_iter()
            .chain(threads_2.into_iter())
            .chain(threads_3.into_iter())
            .try_for_each(|thread| thread.join())
            .unwrap();
        let resolver = Arc::try_unwrap(resolver).unwrap();
        assert_eq!(resolver.resolver.resolved("test_domain_1.com"), Some(1));
        assert_eq!(resolver.resolver.resolved("test_domain_2.com"), Some(1));
        assert_eq!(resolver.resolver.resolved("test_domain_3.com"), Some(1));
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
            .cache_lifetime(Duration::from_secs(1))
            .in_memory();

        for _ in 0..5 {
            let result = resolver.resolve("test_domain_1.com").unwrap();
            assert_eq!(
                result.ip_addrs(),
                &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]
            );
        }

        assert_eq!(resolver.resolver.resolved("test_domain_1.com"), Some(1));

        sleep(Duration::from_secs(2));

        for _ in 0..5 {
            let result = resolver.resolve("test_domain_1.com").unwrap();
            assert_eq!(
                result.ip_addrs(),
                &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]
            );
            sleep(Duration::from_millis(50));
        }

        assert_eq!(resolver.resolver.resolved("test_domain_1.com"), Some(2));
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
            let resolver = CachedResolver::builder(backend.to_owned())
                .load_or_create_from(&tempfile_path, true)?;
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
            sleep(Duration::from_secs(1));
            File::open(resolver.persistent_path().unwrap())?;
        }

        {
            let resolver = CachedResolver::builder(backend.to_owned())
                .load_or_create_from(&tempfile_path, true)?;
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
            assert_eq!(resolver.resolver.resolved("test_domain_1.com"), None);
            assert_eq!(resolver.resolver.resolved("test_domain_2.com"), None);
            assert_eq!(resolver.resolver.resolved("test_domain_3.com"), Some(1));
        }

        sleep(Duration::from_secs(1));

        {
            let resolver = CachedResolver::builder(backend.to_owned())
                .load_or_create_from(&tempfile_path, true)?;
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
            assert_eq!(resolver.resolver.resolved("test_domain_1.com"), None);
            assert_eq!(resolver.resolver.resolved("test_domain_2.com"), None);
            assert_eq!(resolver.resolver.resolved("test_domain_3.com"), None);
        }

        Ok(())
    }
}
