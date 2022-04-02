use super::{super::super::cache::Cache, ResolveAnswers, ResolveOptions, ResolveResult, Resolver};
use std::{
    env::temp_dir,
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

#[cfg(feature = "async")]
use {super::super::super::cache::AsyncCache, futures::future::BoxFuture};

const DEFAULT_SHRINK_INTERVAL: Duration = Duration::from_secs(120);
const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(120);

/// 域名解析缓存器
///
/// 为一个域名解析器实例提供内存和文件系统缓存功能
///
/// 默认缓存 120 秒，清理间隔为 120 秒
#[derive(Debug)]
pub struct CachedResolver<R: ?Sized> {
    resolver: Arc<R>,
    cache: Cache<String, ResolveAnswers>,

    #[cfg(feature = "async")]
    async_cache: AsyncCache<String, ResolveAnswers>,
}

impl<R> CachedResolver<R> {
    /// 创建域名解析缓存构建器
    #[inline]
    pub fn builder(backend: R) -> CachedResolverBuilder<R> {
        CachedResolverBuilder::new(backend)
    }

    /// 获得持久化路径
    ///
    /// 仅在文件系统持久化开启的情况下返回有效值
    #[inline]
    pub fn persistent_path(&self) -> Option<&Path> {
        self.cache.persistent_path()
    }

    /// 是否开启自动持久化
    #[inline]
    pub fn auto_persistent(&self) -> Option<bool> {
        self.cache.auto_persistent()
    }

    fn default_persistent_path() -> PathBuf {
        let mut path = dirs::cache_dir().unwrap_or_else(temp_dir);
        path.push(".qiniu-rust-sdk");
        path.push("resolver-cache.json");
        path
    }
}

impl<R: Default> Default for CachedResolver<R> {
    #[inline]
    fn default() -> Self {
        Self::builder(R::default()).default_load_or_create_from(true)
    }
}

impl<R> Clone for CachedResolver<R> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            resolver: self.resolver.clone(),
            cache: self.cache.clone(),

            #[cfg(feature = "async")]
            async_cache: self.async_cache.clone(),
        }
    }
}

impl<R: Resolver + 'static> Resolver for CachedResolver<R> {
    fn resolve(&self, domain: &str, opts: ResolveOptions) -> ResolveResult {
        self.cache.get(domain, || self.resolver.resolve(domain, opts))
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_resolve<'a>(&'a self, domain: &'a str, opts: ResolveOptions<'a>) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            self.async_cache
                .get(domain, self.resolver.async_resolve(domain, opts))
                .await
        })
    }
}

/// 域名解析缓存构建器
#[derive(Debug)]
pub struct CachedResolverBuilder<R: ?Sized> {
    cache_lifetime: Duration,
    shrink_interval: Duration,
    resolver: R,
}

impl<R> CachedResolverBuilder<R> {
    /// 创建域名解析缓存构建器
    #[inline]
    pub fn new(resolver: R) -> Self {
        Self {
            resolver,
            cache_lifetime: DEFAULT_CACHE_LIFETIME,
            shrink_interval: DEFAULT_SHRINK_INTERVAL,
        }
    }

    /// 设置缓存时长
    #[inline]
    pub fn cache_lifetime(mut self, cache_lifetime: Duration) -> Self {
        self.cache_lifetime = cache_lifetime;
        self
    }

    /// 设置缓存清理间隔
    #[inline]
    pub fn shrink_interval(mut self, shrink_interval: Duration) -> Self {
        self.shrink_interval = shrink_interval;
        self
    }

    /// 从文件系统加载或构建域名解析缓存器
    ///
    /// 可以选择是否启用自动持久化缓存功能
    #[inline]
    pub fn load_or_create_from(self, path: impl AsRef<Path>, auto_persistent: bool) -> CachedResolver<R> {
        CachedResolver {
            resolver: Arc::new(self.resolver),
            cache: Cache::load_or_create_from(
                path.as_ref(),
                auto_persistent,
                self.cache_lifetime,
                self.shrink_interval,
            ),

            #[cfg(feature = "async")]
            async_cache: AsyncCache::load_or_create_from(
                path.as_ref(),
                auto_persistent,
                self.cache_lifetime,
                self.shrink_interval,
            ),
        }
    }

    /// 从默认文件系统路径加载或构建域名解析缓存器
    ///
    /// 可以选择是否启用自动持久化缓存功能
    #[inline]
    pub fn default_load_or_create_from(self, auto_persistent: bool) -> CachedResolver<R> {
        CachedResolver {
            resolver: Arc::new(self.resolver),
            cache: Cache::load_or_create_from(
                &CachedResolver::<R>::default_persistent_path(),
                auto_persistent,
                self.cache_lifetime,
                self.shrink_interval,
            ),

            #[cfg(feature = "async")]
            async_cache: AsyncCache::load_or_create_from(
                &CachedResolver::<R>::default_persistent_path(),
                auto_persistent,
                self.cache_lifetime,
                self.shrink_interval,
            ),
        }
    }

    /// 构建域名解析缓存器
    ///
    /// 不启用文件系统持久化缓存
    #[inline]
    pub fn in_memory(self) -> CachedResolver<R> {
        CachedResolver {
            resolver: Arc::new(self.resolver),
            cache: Cache::in_memory(self.cache_lifetime, self.shrink_interval),

            #[cfg(feature = "async")]
            async_cache: AsyncCache::in_memory(self.cache_lifetime, self.shrink_interval),
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
            self.table.insert(domain.into(), ip_addrs.into_boxed_slice());
        }

        fn resolved(&self, domain: impl AsRef<str>) -> Option<usize> {
            self.resolved.get(domain.as_ref()).map(|v| *v)
        }
    }

    impl Resolver for ResolverFromTable {
        fn resolve(&self, domain: &str, _opts: ResolveOptions) -> ResolveResult {
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
                .unwrap_or_default()
                .into())
        }

        #[cfg(feature = "async")]
        #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
        fn async_resolve<'a>(&'a self, _domain: &'a str, _opts: ResolveOptions) -> BoxFuture<'a, ResolveResult> {
            unreachable!()
        }
    }

    #[test]
    fn test_thread_safe_cached_resolver() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let mut backend = ResolverFromTable::default();
        backend.add("test_domain_1.com", vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]);
        backend.add("test_domain_2.com", vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))]);
        backend.add("test_domain_3.com", vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))]);
        let resolver = Arc::new(
            CachedResolver::builder(backend)
                .cache_lifetime(Duration::from_secs(5))
                .in_memory(),
        );
        let threads_1 = (0..3).map(|_| {
            let resolver = resolver.to_owned();
            spawn(move || {
                let result = resolver.resolve("test_domain_1.com", Default::default()).unwrap();
                assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]);
            })
        });
        let threads_2 = (0..5).map(|_| {
            let resolver = resolver.to_owned();
            spawn(move || {
                let result = resolver.resolve("test_domain_2.com", Default::default()).unwrap();
                assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))]);
            })
        });
        let threads_3 = (0..7).map(|_| {
            let resolver = resolver.to_owned();
            spawn(move || {
                let result = resolver.resolve("test_domain_3.com", Default::default()).unwrap();
                assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))]);
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
        backend.add("test_domain_1.com", vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]);
        let resolver = CachedResolver::builder(backend)
            .cache_lifetime(Duration::from_secs(1))
            .in_memory();

        for _ in 0..5 {
            let result = resolver.resolve("test_domain_1.com", Default::default()).unwrap();
            assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]);
        }

        assert_eq!(resolver.resolver.resolved("test_domain_1.com"), Some(1));

        sleep(Duration::from_secs(2));

        for _ in 0..5 {
            let result = resolver.resolve("test_domain_1.com", Default::default()).unwrap();
            assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]);
            sleep(Duration::from_millis(50));
        }

        assert_eq!(resolver.resolver.resolved("test_domain_1.com"), Some(2));
        Ok(())
    }

    #[test]
    fn test_persistent_resolver() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let mut backend = ResolverFromTable::default();
        backend.add("test_domain_1.com", vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]);
        backend.add("test_domain_2.com", vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))]);
        backend.add("test_domain_3.com", vec![IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))]);

        let tempdir = tempdir()?;
        let tempfile_path = {
            let mut path = tempdir.path().to_owned();
            path.push("resolve_result");
            path
        };

        {
            let resolver = CachedResolver::builder(backend.to_owned()).load_or_create_from(&tempfile_path, true);
            {
                let result = resolver.resolve("test_domain_1.com", Default::default()).unwrap();
                assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]);
            }
            {
                let result = resolver.resolve("test_domain_2.com", Default::default()).unwrap();
                assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))]);
            }
            sleep(Duration::from_secs(1));
            File::open(resolver.persistent_path().unwrap())?;
        }

        {
            let resolver = CachedResolver::builder(backend.to_owned()).load_or_create_from(&tempfile_path, true);
            {
                let result = resolver.resolve("test_domain_1.com", Default::default()).unwrap();
                assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]);
            }
            {
                let result = resolver.resolve("test_domain_2.com", Default::default()).unwrap();
                assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))]);
            }
            {
                let result = resolver.resolve("test_domain_3.com", Default::default()).unwrap();
                assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))]);
            }
            assert_eq!(resolver.resolver.resolved("test_domain_1.com"), None);
            assert_eq!(resolver.resolver.resolved("test_domain_2.com"), None);
            assert_eq!(resolver.resolver.resolved("test_domain_3.com"), Some(1));
        }

        sleep(Duration::from_secs(1));

        {
            let resolver = CachedResolver::builder(backend.to_owned()).load_or_create_from(&tempfile_path, true);
            {
                let result = resolver.resolve("test_domain_1.com", Default::default()).unwrap();
                assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))]);
            }
            {
                let result = resolver.resolve("test_domain_2.com", Default::default()).unwrap();
                assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 2))]);
            }
            {
                let result = resolver.resolve("test_domain_3.com", Default::default()).unwrap();
                assert_eq!(result.ip_addrs(), &[IpAddr::V4(Ipv4Addr::new(192, 168, 0, 3))]);
            }
            assert_eq!(resolver.resolver.resolved("test_domain_1.com"), None);
            assert_eq!(resolver.resolver.resolved("test_domain_2.com"), None);
            assert_eq!(resolver.resolver.resolved("test_domain_3.com"), None);
        }

        Ok(())
    }
}
