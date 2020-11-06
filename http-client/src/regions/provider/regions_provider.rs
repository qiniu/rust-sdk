use super::{
    super::{
        super::{APIResult, Authorization, Client, ResponseError, ResponseErrorKind},
        Endpoint, ServiceName,
    },
    structs::{ResponseBody, DEFAULT_CACHE_LIFETIME},
    Region, RegionProvider,
};
use qiniu_credential::CredentialProvider;
use std::{
    any::Any,
    convert::TryFrom,
    fmt::Debug,
    mem::drop,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

#[cfg(feature = "async")]
use {async_std::task::spawn_blocking, futures::future::BoxFuture};

#[derive(Debug, Clone)]
pub struct RegionsProvider {
    inner: Arc<RegionsProviderInner>,
}

#[derive(Debug)]
struct RegionsProviderInner {
    credential_provider: Arc<dyn CredentialProvider>,
    http_client: Client,
    uc_endpoints: Box<[Endpoint]>,
    cache_lifetime: Duration,
    cache: RwLock<Option<Cache>>,
}

#[derive(Debug)]
struct Cache {
    body: Vec<Region>,
    cached_at: Instant,
}

impl RegionsProvider {
    #[inline]
    pub fn builder(
        http_client: Client,
        uc_endpoints: impl Into<Vec<Endpoint>>,
        credential_provider: Arc<dyn CredentialProvider>,
    ) -> RegionsProviderBuilder {
        RegionsProviderBuilder::new(http_client, uc_endpoints, credential_provider)
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

        fn do_sync_query_with_wlock(inner: &RegionsProviderInner) -> APIResult<Vec<Region>> {
            let mut wlock = inner.cache.write().unwrap();
            if let Some(cache) = &mut *wlock {
                if cache.cached_at.elapsed() > inner.cache_lifetime {
                    let body = _do_sync_query(&inner)?;
                    *cache = Cache {
                        body: body.to_owned(),
                        cached_at: Instant::now(),
                    };
                    Ok(body)
                } else {
                    Ok(cache.body.to_owned())
                }
            } else {
                let body = _do_sync_query(&inner)?;
                *wlock = Some(Cache {
                    body: body.to_owned(),
                    cached_at: Instant::now(),
                });
                Ok(body)
            }
        }

        fn _do_sync_query(inner: &RegionsProviderInner) -> APIResult<Vec<Region>> {
            let body: ResponseBody = inner
                .http_client
                .get(ServiceName::Uc, inner.uc_endpoints.to_owned())
                .path("/regions")
                .authorization(Authorization::v2(inner.credential_provider.to_owned()))
                .accept_json()
                .call()?
                .parse_json()?;
            body.into_hosts()
                .into_iter()
                .map(|host| {
                    Region::try_from(host).map_err(|err| {
                        ResponseError::new(ResponseErrorKind::ParseResponseError, err)
                    })
                })
                .collect()
        }
    }

    #[cfg(feature = "async")]
    async fn do_async_query(&self) -> APIResult<Vec<Region>> {
        let ctx = self.to_owned();

        spawn_blocking(move || ctx.do_sync_query()).await
    }
}
impl RegionProvider for RegionsProvider {
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

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_region_provider(&self) -> &dyn RegionProvider {
        self
    }
}

#[derive(Debug)]
pub struct RegionsProviderBuilder {
    credential_provider: Arc<dyn CredentialProvider>,
    http_client: Client,
    uc_endpoints: Vec<Endpoint>,
    cache_lifetime: Duration,
}

impl RegionsProviderBuilder {
    #[inline]
    pub fn new(
        http_client: Client,
        uc_endpoints: impl Into<Vec<Endpoint>>,
        credential_provider: Arc<dyn CredentialProvider>,
    ) -> Self {
        Self {
            http_client,
            credential_provider,
            uc_endpoints: uc_endpoints.into(),
            cache_lifetime: DEFAULT_CACHE_LIFETIME,
        }
    }

    pub fn cache_lifetime(mut self, cache_lifetime: Duration) -> Self {
        self.cache_lifetime = cache_lifetime;
        self
    }

    pub fn build(self) -> RegionsProvider {
        RegionsProvider {
            inner: Arc::new(RegionsProviderInner {
                http_client: self.http_client,
                credential_provider: self.credential_provider,
                uc_endpoints: self.uc_endpoints.into_boxed_slice(),
                cache: Default::default(),
                cache_lifetime: self.cache_lifetime,
            }),
        }
    }
}
