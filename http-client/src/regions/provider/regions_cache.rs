use super::{
    super::{
        super::{
            cache::{Cache, CacheController},
            ApiResult,
        },
        Region,
    },
    cache_key::CacheKey,
};
use std::{
    env::temp_dir,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Clone)]
pub(super) struct RegionsCache {
    inner: Cache<CacheKey, Vec<Region>>,
}

impl RegionsCache {
    pub(super) fn load_or_create_from(
        path: &Path,
        auto_persistent: bool,
        cache_lifetime: Duration,
        shrink_interval: Duration,
    ) -> Self {
        Self {
            inner: Cache::load_or_create_from(
                path,
                auto_persistent,
                cache_lifetime,
                shrink_interval,
            ),
        }
    }

    pub(super) fn default_load_or_create_from(
        auto_persistent: bool,
        cache_lifetime: Duration,
        shrink_interval: Duration,
    ) -> Self {
        Self::load_or_create_from(
            &Self::default_persistent_path(),
            auto_persistent,
            cache_lifetime,
            shrink_interval,
        )
    }

    pub(super) fn default_persistent_path() -> PathBuf {
        let mut path = dirs::cache_dir().unwrap_or_else(temp_dir);
        path.push(".qiniu-rust-sdk");
        path.push("regions-cache.json");
        path
    }

    pub(super) fn in_memory(cache_lifetime: Duration, shrink_interval: Duration) -> Self {
        Self {
            inner: Cache::in_memory(cache_lifetime, shrink_interval),
        }
    }

    pub(super) fn get(
        &self,
        key: &CacheKey,
        f: impl FnMut() -> ApiResult<Vec<Region>> + Send + Sync + 'static,
    ) -> ApiResult<Vec<Region>> {
        self.inner.get(key, f)
    }

    #[allow(dead_code)]
    pub(super) fn set(&self, key: CacheKey, regions: Vec<Region>) {
        self.inner.set(key, regions)
    }

    #[allow(dead_code)]
    pub(super) fn remove(&self, key: &CacheKey) {
        self.inner.remove(key)
    }
}

impl CacheController for RegionsCache {
    #[inline]
    fn clear(&self) {
        self.inner.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{super::super::Endpoints, *};
    use crate::test_utils::chaotic_up_domains_region;
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering::Relaxed},
            Arc,
        },
        thread::sleep,
    };
    use tempfile::NamedTempFile;

    #[test]
    fn test_regions_cache_in_memory() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let cache = RegionsCache::in_memory(Duration::from_secs(1), Duration::from_secs(1));
        let cache_key = CacheKey::new_from_endpoint_and_ak_and_bucket(
            &Endpoints::builder("https://fake.uc.qiniu.com").build(),
            "fakebucket".into(),
            "fakeaccesskey".into(),
        );
        let generate_new_cache = Arc::new(AtomicBool::new(false));
        assert_eq!(
            cache
                .get(&cache_key, {
                    let generate_new_cache = generate_new_cache.to_owned();
                    move || {
                        generate_new_cache.store(true, Relaxed);
                        Ok(vec![chaotic_up_domains_region()])
                    }
                })?
                .len(),
            1
        );
        assert!(generate_new_cache.load(Relaxed));
        assert!(cache.inner.exists(&cache_key));
        cache.get(&cache_key, || unreachable!())?;

        sleep(Duration::from_secs(1));

        let cache_key2 = CacheKey::new_from_endpoint_and_ak_and_bucket(
            &Endpoints::builder("https://fake.uc2.qiniu.com").build(),
            "fakebucket".into(),
            "fakeaccesskey".into(),
        );

        generate_new_cache.store(false, Relaxed);
        assert_eq!(
            cache
                .get(&cache_key2, {
                    let generate_new_cache = generate_new_cache.to_owned();
                    move || {
                        generate_new_cache.store(true, Relaxed);
                        Ok(vec![chaotic_up_domains_region()])
                    }
                })?
                .len(),
            1
        );
        assert!(generate_new_cache.load(Relaxed));
        assert!(cache.inner.exists(&cache_key2));

        sleep(Duration::from_secs(1));
        assert!(!cache.inner.exists(&cache_key));
        assert!(cache.inner.exists(&cache_key2));

        Ok(())
    }

    #[test]
    fn test_regions_cache_auto_persistent() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let temp_file = NamedTempFile::new()?;
        let temp_file_path = temp_file.into_temp_path();
        let cache = RegionsCache::load_or_create_from(
            &temp_file_path,
            true,
            Duration::from_secs(120),
            Duration::from_secs(120),
        );
        let cache_key_1 = CacheKey::new_from_endpoint_and_ak_and_bucket(
            &Endpoints::builder("https://fake.uc.qiniu.com").build(),
            "fakebucket".into(),
            "fakeaccesskey".into(),
        );
        let cache_key_2 = CacheKey::new_from_endpoint_and_ak_and_bucket(
            &Endpoints::builder("https://fake.uc2.qiniu.com").build(),
            "fakebucket".into(),
            "fakeaccesskey".into(),
        );

        let regions_1 = vec![Region::builder("test")
            .push_up_preferred_endpoint("fakedomain_1.withport.com".to_owned())
            .build()];
        assert_eq!(
            cache
                .get(&cache_key_1, {
                    let regions_1 = regions_1.to_owned();
                    move || Ok(regions_1.to_owned())
                })?
                .len(),
            1
        );
        assert!(cache.inner.exists(&cache_key_1));

        let regions_2 = vec![Region::builder("test")
            .push_up_preferred_endpoint("fakedomain_2.withport.com".to_owned())
            .build()];
        cache.set(cache_key_1.to_owned(), regions_2.to_owned());
        assert!(cache.inner.exists(&cache_key_1));
        drop(cache);

        let cache = RegionsCache::load_or_create_from(
            &temp_file_path,
            true,
            Duration::from_secs(120),
            Duration::from_secs(120),
        );
        assert_eq!(cache.get(&cache_key_1, || unreachable!())?, regions_2);
        cache.remove(&cache_key_1);
        assert!(!cache.inner.exists(&cache_key_1));
        drop(cache);

        let cache = RegionsCache::load_or_create_from(
            &temp_file_path,
            true,
            Duration::from_secs(120),
            Duration::from_secs(120),
        );
        assert!(!cache.inner.exists(&cache_key_1));

        assert_eq!(
            cache
                .get(&cache_key_1, {
                    let regions_1 = regions_1.to_owned();
                    move || Ok(regions_1.to_owned())
                })?
                .len(),
            1
        );
        assert_eq!(
            cache
                .get(&cache_key_2, {
                    let regions_2 = regions_2.to_owned();
                    move || Ok(regions_2.to_owned())
                })?
                .len(),
            1
        );
        assert!(cache.inner.exists(&cache_key_1));
        assert!(cache.inner.exists(&cache_key_2));

        sleep(Duration::from_secs(1));

        cache.clear();
        assert!(!cache.inner.exists(&cache_key_1));
        assert!(!cache.inner.exists(&cache_key_2));

        sleep(Duration::from_secs(1));
        drop(cache);

        let cache = RegionsCache::load_or_create_from(
            &temp_file_path,
            true,
            Duration::from_secs(120),
            Duration::from_secs(120),
        );
        assert!(!cache.inner.exists(&cache_key_1));
        assert!(!cache.inner.exists(&cache_key_2));

        assert_eq!(
            cache
                .get(&cache_key_1, move || Ok(regions_1.to_owned()))?
                .len(),
            1
        );
        assert_eq!(
            cache
                .get(&cache_key_2, move || Ok(regions_2.to_owned()))?
                .len(),
            1
        );
        sleep(Duration::from_secs(1));
        assert!(cache.inner.exists(&cache_key_1));
        assert!(cache.inner.exists(&cache_key_2));
        drop(cache);

        let cache = RegionsCache::load_or_create_from(
            &temp_file_path,
            true,
            Duration::from_secs(1),
            Duration::from_secs(120),
        );
        assert!(!cache.inner.exists(&cache_key_1));
        assert!(!cache.inner.exists(&cache_key_2));

        Ok(())
    }

    #[test]
    fn test_regions_cache_without_auto_persistent() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let temp_file = NamedTempFile::new()?;
        let temp_file_path = temp_file.into_temp_path();
        let cache = RegionsCache::load_or_create_from(
            &temp_file_path,
            false,
            Duration::from_secs(120),
            Duration::from_secs(120),
        );
        let cache_key = CacheKey::new_from_endpoint_and_ak_and_bucket(
            &Endpoints::builder("https://fake.uc.qiniu.com").build(),
            "fakebucket".into(),
            "fakeaccesskey".into(),
        );
        let regions = vec![Region::builder("test")
            .push_up_preferred_endpoint("fakedomain_1.withport.com".to_owned())
            .build()];
        assert_eq!(
            cache.get(&cache_key, move || Ok(regions.to_owned()))?.len(),
            1
        );
        assert!(cache.inner.exists(&cache_key));
        drop(cache);

        let cache = RegionsCache::load_or_create_from(
            &temp_file_path,
            false,
            Duration::from_secs(120),
            Duration::from_secs(120),
        );
        assert!(!cache.inner.exists(&cache_key));

        Ok(())
    }
}
