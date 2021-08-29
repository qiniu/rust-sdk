use super::{super::super::APIResult, structs::DEFAULT_CACHE_LIFETIME, Region, RegionProvider};
use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

#[cfg(feature = "async")]
use {async_std::task::spawn, futures::future::BoxFuture};

#[derive(Debug)]
struct Cache {
    body: Vec<Region>,
    cached_at: Instant,
}

#[derive(Debug, Clone)]
pub struct CachedRegionsProvider<P: RegionProvider> {
    inner: Arc<CachedRegionsProviderInner<P>>,
}

#[derive(Debug)]
struct CachedRegionsProviderInner<P: RegionProvider> {
    provider: P,
    cache_lifetime: Duration,
    cache: RwLock<Option<Cache>>,
}

impl<P: RegionProvider> RegionProvider for CachedRegionsProvider<P> {
    fn get(&self) -> APIResult<Region> {
        self.get_all().map(|regions| {
            regions
                .into_iter()
                .next()
                .expect("Regions API returns empty regions")
        })
    }

    #[inline]
    fn get_all(&self) -> APIResult<Vec<Region>> {
        self.do_sync_query()
    }

    /// 异步返回七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get(&self) -> BoxFuture<APIResult<Region>> {
        Box::pin(async move {
            self.async_get_all().await.map(|regions| {
                regions
                    .into_iter()
                    .next()
                    .expect("Regions API returns empty regions")
            })
        })
    }

    /// 异步返回多个七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get_all(&self) -> BoxFuture<APIResult<Vec<Region>>> {
        Box::pin(async move { self.do_async_query().await })
    }

    #[inline]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[inline]
    fn as_region_provider(&self) -> &dyn RegionProvider {
        self
    }
}

impl<P: RegionProvider> CachedRegionsProvider<P> {
    #[inline]
    pub fn cache_for(cache_lifetime: Duration, provider: P) -> Self {
        Self {
            inner: Arc::new(CachedRegionsProviderInner {
                provider,
                cache_lifetime,
                cache: Default::default(),
            }),
        }
    }

    #[inline]
    pub fn cache(provider: P) -> Self {
        Self::cache_for(DEFAULT_CACHE_LIFETIME, provider)
    }

    fn do_sync_query(&self) -> APIResult<Vec<Region>> {
        let rlock = self.inner.cache.read().unwrap();
        return if let Some(cache) = &*rlock {
            if cache.cached_at.elapsed() > self.inner.cache_lifetime {
                drop(rlock);
                do_sync_query_with_wlock(&self.inner)
            } else {
                Ok(cache.body.to_owned())
            }
        } else {
            drop(rlock);
            do_sync_query_with_wlock(&self.inner)
        };

        fn do_sync_query_with_wlock<P: RegionProvider>(
            inner: &CachedRegionsProviderInner<P>,
        ) -> APIResult<Vec<Region>> {
            let mut wlock = inner.cache.write().unwrap();
            if let Some(cache) = &mut *wlock {
                if cache.cached_at.elapsed() > inner.cache_lifetime {
                    let body = inner.provider.get_all()?;
                    *cache = Cache {
                        body: body.to_owned(),
                        cached_at: Instant::now(),
                    };
                    Ok(body)
                } else {
                    Ok(cache.body.to_owned())
                }
            } else {
                let body = inner.provider.get_all()?;
                *wlock = Some(Cache {
                    body: body.to_owned(),
                    cached_at: Instant::now(),
                });
                Ok(body)
            }
        }
    }

    #[inline]
    #[cfg(feature = "async")]
    async fn do_async_query(&self) -> APIResult<Vec<Region>> {
        let ctx = Self {
            inner: self.inner.to_owned(),
        };
        spawn(async move { ctx.do_sync_query() }).await
    }
}
