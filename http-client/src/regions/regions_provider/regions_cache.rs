use super::{
    super::{
        super::{
            cache::{Cache, CacheController},
            ApiResult,
        },
        cache_key::CacheKey,
    },
    GotRegions,
};
use std::{
    env::temp_dir,
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
use {
    super::super::super::cache::{AsyncCache, AsyncCacheController},
    futures::future::BoxFuture,
    std::future::Future,
};

#[derive(Debug, Clone)]
pub(super) struct RegionsCache {
    cache: Cache<CacheKey, GotRegions>,

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    async_cache: AsyncCache<CacheKey, GotRegions>,
}

impl RegionsCache {
    pub(super) fn load_or_create_from(path: &Path, auto_persistent: bool, shrink_interval: Duration) -> Self {
        Self {
            cache: Cache::load_or_create_from(path, auto_persistent, shrink_interval),

            #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
            async_cache: AsyncCache::load_or_create_from(path, auto_persistent, shrink_interval),
        }
    }

    pub(super) fn default_load_or_create_from(auto_persistent: bool, shrink_interval: Duration) -> Self {
        Self::load_or_create_from(&Self::default_persistent_path(), auto_persistent, shrink_interval)
    }

    fn default_persistent_path() -> PathBuf {
        let mut path = dirs::cache_dir().unwrap_or_else(temp_dir);
        path.push(".qiniu-rust-sdk");
        path.push("regions-cache.json");
        path
    }

    pub(super) fn in_memory(shrink_interval: Duration) -> Self {
        Self {
            cache: Cache::in_memory(shrink_interval),

            #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
            async_cache: AsyncCache::in_memory(shrink_interval),
        }
    }

    pub(super) fn get(&self, key: &CacheKey, f: impl FnOnce() -> ApiResult<GotRegions>) -> ApiResult<GotRegions> {
        self.cache.get(key, f)
    }

    #[allow(dead_code)]
    pub(super) fn set(&self, key: CacheKey, regions: GotRegions) {
        self.cache.set(key, regions)
    }

    #[allow(dead_code)]
    pub(super) fn remove(&self, key: &CacheKey) {
        self.cache.remove(key)
    }
}

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
impl RegionsCache {
    pub(super) async fn async_get<Fut: Future<Output = ApiResult<GotRegions>>>(
        &self,
        key: &CacheKey,
        fut: Fut,
    ) -> ApiResult<GotRegions> {
        self.async_cache.get(key, fut).await
    }

    #[allow(dead_code)]
    pub(super) async fn async_set(&self, key: CacheKey, regions: GotRegions) {
        self.async_cache.set(key, regions).await
    }

    #[allow(dead_code)]
    pub(super) async fn async_remove(&self, key: &CacheKey) {
        self.async_cache.remove(key).await
    }
}

impl CacheController for RegionsCache {
    #[inline]
    fn clear(&self) {
        self.cache.clear();
    }
}

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
impl AsyncCacheController for RegionsCache {
    #[inline]
    fn async_clear(&self) -> BoxFuture<()> {
        Box::pin(async move {
            self.async_cache.async_clear().await;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::super::{Endpoints, Region},
        *,
    };
    use crate::test_utils::chaotic_up_domains_region;
    use std::thread::sleep;
    use tempfile::NamedTempFile;

    #[test]
    fn test_regions_cache_in_memory() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let cache = RegionsCache::in_memory(Duration::from_secs(1));
        let cache_key = CacheKey::new_from_endpoint_and_ak_and_bucket(
            &Endpoints::builder("fake.uc.qiniu.com".parse()?).build(),
            "fakebucket".into(),
            "fakeaccesskey".into(),
        );
        let mut generate_new_cache = false;
        assert_eq!(
            cache
                .get(&cache_key, || {
                    generate_new_cache = true;
                    Ok(GotRegions::new(
                        vec![chaotic_up_domains_region()],
                        Duration::from_secs(1),
                    ))
                })?
                .len(),
            1
        );
        assert!(generate_new_cache);
        assert!(cache.cache.exists(&cache_key));
        cache.get(&cache_key, || unreachable!())?;

        sleep(Duration::from_secs(3));

        let cache_key2 = CacheKey::new_from_endpoint_and_ak_and_bucket(
            &Endpoints::builder("fake.uc2.qiniu.com".parse()?).build(),
            "fakebucket".into(),
            "fakeaccesskey".into(),
        );

        generate_new_cache = false;
        assert_eq!(
            cache
                .get(&cache_key2, || {
                    generate_new_cache = true;
                    Ok(GotRegions::new(
                        vec![chaotic_up_domains_region()],
                        Duration::from_secs(1),
                    ))
                })?
                .len(),
            1
        );
        assert!(generate_new_cache);
        assert!(cache.cache.exists(&cache_key2));

        sleep(Duration::from_secs(3));
        assert!(!cache.cache.exists(&cache_key));
        assert!(cache.cache.exists(&cache_key2));

        Ok(())
    }

    #[test]
    fn test_regions_cache_in_memory_with_invalidation() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let cache = RegionsCache::in_memory(Duration::from_secs(60));
        let cache_key = CacheKey::new_from_endpoint_and_ak_and_bucket(
            &Endpoints::builder("fake.uc.qiniu.com".parse()?).build(),
            "fakebucket".into(),
            "fakeaccesskey".into(),
        );
        let mut generate_new_cache = false;
        assert_eq!(
            cache
                .get(&cache_key, || {
                    generate_new_cache = true;
                    Ok(GotRegions::new(
                        vec![chaotic_up_domains_region()],
                        Duration::from_secs(1),
                    ))
                })?
                .len(),
            1
        );

        assert!(generate_new_cache);
        assert!(cache.cache.exists(&cache_key));
        cache.get(&cache_key, || unreachable!())?;

        sleep(Duration::from_secs(3));
        generate_new_cache = false;
        cache.get(&cache_key, || {
            generate_new_cache = true;
            Ok(GotRegions::new(
                vec![chaotic_up_domains_region()],
                Duration::from_secs(1),
            ))
        })?;
        assert!(generate_new_cache);

        Ok(())
    }

    #[test]
    fn test_regions_cache_auto_persistent() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let temp_file = NamedTempFile::new()?;
        let temp_file_path = temp_file.into_temp_path();
        let temp_file_path = temp_file_path.keep()?;
        let cache = RegionsCache::load_or_create_from(&temp_file_path, true, Duration::from_secs(120));
        let cache_key_1 = CacheKey::new_from_endpoint_and_ak_and_bucket(
            &Endpoints::builder("fake.uc.qiniu.com".parse()?).build(),
            "fakebucket".into(),
            "fakeaccesskey".into(),
        );
        let cache_key_2 = CacheKey::new_from_endpoint_and_ak_and_bucket(
            &Endpoints::builder("fake.uc2.qiniu.com".parse()?).build(),
            "fakebucket".into(),
            "fakeaccesskey".into(),
        );

        let regions_1 = GotRegions::new(
            vec![Region::builder("test")
                .add_up_preferred_endpoint("fakedomain_1.withport.com".parse()?)
                .build()],
            Duration::from_secs(2),
        );
        assert_eq!(cache.get(&cache_key_1, || Ok(regions_1.to_owned()))?.len(), 1);
        assert!(cache.cache.exists(&cache_key_1));

        let regions_2 = GotRegions::new(
            vec![Region::builder("test")
                .add_up_preferred_endpoint("fakedomain_2.withport.com".parse()?)
                .build()],
            Duration::from_secs(2),
        );
        cache.set(cache_key_1.to_owned(), regions_2.to_owned());
        assert!(cache.cache.exists(&cache_key_1));
        drop(cache);
        sleep(Duration::from_secs(1));

        let cache = RegionsCache::load_or_create_from(&temp_file_path, true, Duration::from_secs(120));
        assert_eq!(cache.get(&cache_key_1, || unreachable!())?, regions_2.to_owned());
        cache.remove(&cache_key_1);
        assert!(!cache.cache.exists(&cache_key_1));
        drop(cache);
        sleep(Duration::from_secs(1));

        let cache = RegionsCache::load_or_create_from(&temp_file_path, true, Duration::from_secs(120));
        assert!(!cache.cache.exists(&cache_key_1));

        assert_eq!(cache.get(&cache_key_1, || Ok(regions_1.to_owned()))?.len(), 1);
        assert_eq!(cache.get(&cache_key_2, || Ok(regions_2.to_owned()))?.len(), 1);
        assert!(cache.cache.exists(&cache_key_1));
        assert!(cache.cache.exists(&cache_key_2));

        sleep(Duration::from_secs(1));

        cache.clear();
        assert!(!cache.cache.exists(&cache_key_1));
        assert!(!cache.cache.exists(&cache_key_2));

        sleep(Duration::from_secs(1));
        drop(cache);

        let cache = RegionsCache::load_or_create_from(&temp_file_path, true, Duration::from_secs(120));
        assert!(!cache.cache.exists(&cache_key_1));
        assert!(!cache.cache.exists(&cache_key_2));

        assert_eq!(cache.get(&cache_key_1, || Ok(regions_1.to_owned()))?.len(), 1);
        assert_eq!(cache.get(&cache_key_2, || Ok(regions_2.to_owned()))?.len(), 1);
        sleep(Duration::from_secs(1));
        assert!(cache.cache.exists(&cache_key_1));
        assert!(cache.cache.exists(&cache_key_2));
        drop(cache);

        let cache = RegionsCache::load_or_create_from(&temp_file_path, true, Duration::from_secs(120));
        assert!(!cache.cache.exists(&cache_key_1));
        assert!(!cache.cache.exists(&cache_key_2));

        Ok(())
    }

    #[test]
    fn test_regions_cache_without_auto_persistent() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let temp_file = NamedTempFile::new()?;
        let temp_file_path = temp_file.into_temp_path();
        let cache = RegionsCache::load_or_create_from(&temp_file_path, false, Duration::from_secs(120));
        let cache_key = CacheKey::new_from_endpoint_and_ak_and_bucket(
            &Endpoints::builder("fake.uc.qiniu.com".parse()?).build(),
            "fakebucket".into(),
            "fakeaccesskey".into(),
        );
        let regions = vec![Region::builder("test")
            .add_up_preferred_endpoint("fakedomain_1.withport.com".parse()?)
            .build()];
        assert_eq!(cache.get(&cache_key, || Ok(regions.to_owned().into()))?.len(), 1);
        assert!(cache.cache.exists(&cache_key));
        drop(cache);

        let cache = RegionsCache::load_or_create_from(&temp_file_path, false, Duration::from_secs(120));
        assert!(!cache.cache.exists(&cache_key));

        Ok(())
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    mod async_test {
        use super::*;
        use qiniu_utils::async_task::sleep;

        #[qiniu_utils::async_runtime::test]
        async fn test_regions_cache_in_memory() -> anyhow::Result<()> {
            env_logger::builder().is_test(true).try_init().ok();

            let cache = RegionsCache::in_memory(Duration::from_secs(1));
            let cache_key = CacheKey::new_from_endpoint_and_ak_and_bucket(
                &Endpoints::builder("fake.uc.qiniu.com".parse()?).build(),
                "fakebucket".into(),
                "fakeaccesskey".into(),
            );
            let mut generate_new_cache = false;
            assert_eq!(
                cache
                    .async_get(&cache_key, async {
                        generate_new_cache = true;
                        Ok(GotRegions::new(
                            vec![chaotic_up_domains_region()],
                            Duration::from_secs(1),
                        ))
                    })
                    .await?
                    .len(),
                1
            );
            assert!(generate_new_cache);
            assert!(cache.async_cache.exists(&cache_key).await);
            cache.async_get(&cache_key, async { unreachable!() }).await?;

            sleep(Duration::from_secs(3)).await;

            let cache_key2 = CacheKey::new_from_endpoint_and_ak_and_bucket(
                &Endpoints::builder("fake.uc2.qiniu.com".parse()?).build(),
                "fakebucket".into(),
                "fakeaccesskey".into(),
            );

            generate_new_cache = false;
            assert_eq!(
                cache
                    .async_get(&cache_key2, async {
                        generate_new_cache = true;
                        Ok(GotRegions::new(
                            vec![chaotic_up_domains_region()],
                            Duration::from_secs(1),
                        ))
                    })
                    .await?
                    .len(),
                1
            );
            assert!(generate_new_cache);
            assert!(cache.async_cache.exists(&cache_key2).await);

            sleep(Duration::from_secs(3)).await;
            assert!(!cache.async_cache.exists(&cache_key).await);
            assert!(cache.async_cache.exists(&cache_key2).await);

            Ok(())
        }

        #[qiniu_utils::async_runtime::test]
        async fn test_regions_cache_in_memory_with_invalidation() -> anyhow::Result<()> {
            env_logger::builder().is_test(true).try_init().ok();

            let cache = RegionsCache::in_memory(Duration::from_secs(60));
            let cache_key = CacheKey::new_from_endpoint_and_ak_and_bucket(
                &Endpoints::builder("fake.uc.qiniu.com".parse()?).build(),
                "fakebucket".into(),
                "fakeaccesskey".into(),
            );
            let mut generate_new_cache = false;
            assert_eq!(
                cache
                    .async_get(&cache_key, async {
                        generate_new_cache = true;
                        Ok(GotRegions::new(
                            vec![chaotic_up_domains_region()],
                            Duration::from_secs(1),
                        ))
                    })
                    .await?
                    .len(),
                1
            );

            assert!(generate_new_cache);
            assert!(cache.async_cache.exists(&cache_key).await);
            cache.async_get(&cache_key, async { unreachable!() }).await?;

            sleep(Duration::from_secs(3)).await;
            generate_new_cache = false;
            cache
                .async_get(&cache_key, async {
                    generate_new_cache = true;
                    Ok(GotRegions::new(
                        vec![chaotic_up_domains_region()],
                        Duration::from_secs(1),
                    ))
                })
                .await?;
            sleep(Duration::from_secs(3)).await;
            assert!(generate_new_cache);

            Ok(())
        }

        #[qiniu_utils::async_runtime::test]
        async fn test_regions_cache_auto_persistent() -> anyhow::Result<()> {
            env_logger::builder().is_test(true).try_init().ok();

            let temp_file = NamedTempFile::new()?;
            let temp_file_path = temp_file.into_temp_path();
            let cache = RegionsCache::load_or_create_from(&temp_file_path, true, Duration::from_secs(120));
            let cache_key_1 = CacheKey::new_from_endpoint_and_ak_and_bucket(
                &Endpoints::builder("fake.uc.qiniu.com".parse()?).build(),
                "fakebucket".into(),
                "fakeaccesskey".into(),
            );
            let cache_key_2 = CacheKey::new_from_endpoint_and_ak_and_bucket(
                &Endpoints::builder("fake.uc2.qiniu.com".parse()?).build(),
                "fakebucket".into(),
                "fakeaccesskey".into(),
            );

            let regions_1 = vec![Region::builder("test")
                .add_up_preferred_endpoint("fakedomain_1.withport.com".parse()?)
                .build()];
            assert_eq!(
                cache
                    .async_get(&cache_key_1, async {
                        Ok(GotRegions::new(regions_1.to_owned(), Duration::from_secs(2)))
                    })
                    .await?
                    .len(),
                1
            );
            assert!(cache.async_cache.exists(&cache_key_1).await);

            let regions_2 = vec![Region::builder("test")
                .add_up_preferred_endpoint("fakedomain_2.withport.com".parse()?)
                .build()];
            cache
                .async_set(
                    cache_key_1.to_owned(),
                    GotRegions::new(regions_2.to_owned(), Duration::from_secs(2)),
                )
                .await;
            assert!(cache.async_cache.exists(&cache_key_1).await);

            drop(cache);

            sleep(Duration::from_secs(1)).await;

            let cache = RegionsCache::load_or_create_from(&temp_file_path, true, Duration::from_secs(120));
            assert_eq!(
                cache.async_get(&cache_key_1, async { unreachable!() }).await?,
                regions_2.to_owned().into()
            );
            cache.async_remove(&cache_key_1).await;
            assert!(!cache.async_cache.exists(&cache_key_1).await);
            drop(cache);
            sleep(Duration::from_secs(1)).await;

            let cache = RegionsCache::load_or_create_from(&temp_file_path, true, Duration::from_secs(120));
            assert!(!cache.async_cache.exists(&cache_key_1).await);

            assert_eq!(
                cache
                    .async_get(&cache_key_1, async {
                        Ok(GotRegions::new(regions_1.to_owned(), Duration::from_secs(1)))
                    })
                    .await?
                    .len(),
                1
            );
            assert_eq!(
                cache
                    .async_get(&cache_key_2, async {
                        Ok(GotRegions::new(regions_2.to_owned(), Duration::from_secs(1)))
                    })
                    .await?
                    .len(),
                1
            );
            assert!(cache.async_cache.exists(&cache_key_1).await);
            assert!(cache.async_cache.exists(&cache_key_2).await);

            cache.async_clear().await;
            assert!(!cache.async_cache.exists(&cache_key_1).await);
            assert!(!cache.async_cache.exists(&cache_key_2).await);

            sleep(Duration::from_secs(2)).await;
            drop(cache);

            let cache = RegionsCache::load_or_create_from(&temp_file_path, true, Duration::from_secs(120));
            assert!(!cache.async_cache.exists(&cache_key_1).await);
            assert!(!cache.async_cache.exists(&cache_key_2).await);

            assert_eq!(
                cache
                    .async_get(&cache_key_1, async {
                        Ok(GotRegions::new(regions_1.to_owned(), Duration::from_secs(1)))
                    })
                    .await?
                    .len(),
                1
            );
            assert_eq!(
                cache
                    .async_get(&cache_key_2, async {
                        Ok(GotRegions::new(regions_2.to_owned(), Duration::from_secs(1)))
                    })
                    .await?
                    .len(),
                1
            );
            sleep(Duration::from_secs(1)).await;
            assert!(cache.async_cache.exists(&cache_key_1).await);
            assert!(cache.async_cache.exists(&cache_key_2).await);
            sleep(Duration::from_secs(1)).await;
            drop(cache);

            let cache = RegionsCache::load_or_create_from(&temp_file_path, true, Duration::from_secs(120));
            assert!(!cache.async_cache.exists(&cache_key_1).await);
            assert!(!cache.async_cache.exists(&cache_key_2).await);

            Ok(())
        }

        #[qiniu_utils::async_runtime::test]
        async fn test_regions_cache_without_auto_persistent() -> anyhow::Result<()> {
            env_logger::builder().is_test(true).try_init().ok();

            let temp_file = NamedTempFile::new()?;
            let temp_file_path = temp_file.into_temp_path();
            let cache = RegionsCache::load_or_create_from(&temp_file_path, false, Duration::from_secs(120));
            let cache_key = CacheKey::new_from_endpoint_and_ak_and_bucket(
                &Endpoints::builder("fake.uc.qiniu.com".parse()?).build(),
                "fakebucket".into(),
                "fakeaccesskey".into(),
            );
            let regions = vec![Region::builder("test")
                .add_up_preferred_endpoint("fakedomain_1.withport.com".parse()?)
                .build()];
            assert_eq!(
                cache
                    .async_get(&cache_key, async {
                        Ok(GotRegions::new(regions, Duration::from_secs(120)))
                    })
                    .await?
                    .len(),
                1
            );
            assert!(cache.async_cache.exists(&cache_key).await);
            drop(cache);

            let cache = RegionsCache::load_or_create_from(&temp_file_path, false, Duration::from_secs(120));
            assert!(!cache.async_cache.exists(&cache_key).await);

            Ok(())
        }
    }
}
