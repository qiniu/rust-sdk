use super::{
    super::{
        super::{APIResult, Client, ResponseError, ResponseErrorKind},
        Endpoint, EndpointParseError, ServiceName,
    },
    Region, RegionProvider,
};
use dashmap::DashMap;
use serde::Deserialize;
use std::{
    any::Any,
    convert::TryFrom,
    fmt::Debug,
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(feature = "async")]
use {async_std::task::block_on, futures::future::BoxFuture};

const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(86400);

#[derive(Debug, Clone, Deserialize)]
struct ResponseBody {
    hosts: Vec<RegionResponseBody>,
}

#[derive(Debug, Clone, Deserialize)]
struct RegionResponseBody {
    region: Box<str>,
    io: DomainsResponseBody,
    up: DomainsResponseBody,
    uc: DomainsResponseBody,
    rs: DomainsResponseBody,
    rsf: DomainsResponseBody,
    api: DomainsResponseBody,
    s3: DomainsResponseBody,
}

#[derive(Debug, Clone, Deserialize)]
struct DomainsResponseBody {
    domains: Box<[Box<str>]>,
    old: Option<Box<[Box<str>]>>,
}

impl TryFrom<RegionResponseBody> for Region {
    type Error = EndpointParseError;
    fn try_from(body: RegionResponseBody) -> Result<Self, Self::Error> {
        let RegionResponseBody {
            region,
            io,
            up,
            uc,
            rs,
            rsf,
            api,
            s3,
        } = body;
        let mut builder = Self::builder(region);

        macro_rules! push_to_builder {
            ($service_name:expr, $push_to_endpoint:ident, $push_to_old_endpoint:ident) => {
                for domain in $service_name.domains.iter() {
                    let endpoint: Endpoint = domain.as_ref().parse()?;
                    builder = builder.$push_to_endpoint(endpoint);
                }
                if let Some(old_domains) = &$service_name.old {
                    for old_domain in old_domains.iter() {
                        let endpoint: Endpoint = old_domain.as_ref().parse()?;
                        builder = builder.$push_to_old_endpoint(endpoint);
                    }
                }
            };
        }
        push_to_builder!(io, push_io_endpoint, push_io_old_endpoint);
        push_to_builder!(up, push_up_endpoint, push_up_old_endpoint);
        push_to_builder!(uc, push_uc_endpoint, push_uc_old_endpoint);
        push_to_builder!(rs, push_rs_endpoint, push_rs_old_endpoint);
        push_to_builder!(rsf, push_rsf_endpoint, push_rsf_old_endpoint);
        push_to_builder!(api, push_api_endpoint, push_api_old_endpoint);
        push_to_builder!(s3, push_s3_endpoint, push_s3_old_endpoint);

        Ok(builder.build())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct CacheKey {
    bucket_name: Box<str>,
    access_key: Box<str>,
}

#[derive(Debug, Clone)]
struct CacheValue {
    body: ResponseBody,
    cached_at: Instant,
}

#[derive(Debug, Clone)]
pub struct BucketRegionsQueryer {
    inner: Arc<BucketRegionsQueryerInner>,
}

#[derive(Debug)]
struct BucketRegionsQueryerInner {
    http_client: Client,
    uc_endpoints: Box<[Endpoint]>,
    cache_lifetime: Duration,
    cache: DashMap<CacheKey, CacheValue>,
}

pub struct BucketRegionsQueryerBuilder {
    http_client: Client,
    uc_endpoints: Vec<Endpoint>,
    cache_lifetime: Duration,
}

impl BucketRegionsQueryer {
    #[inline]
    pub fn query(
        &self,
        access_key: impl Into<String>,
        bucket_name: impl Into<String>,
    ) -> BucketRegionsProvider {
        BucketRegionsProvider {
            inner: Arc::new(BucketRegionsProviderInner {
                queryer: self.to_owned(),
                access_key: access_key.into().into_boxed_str(),
                bucket_name: bucket_name.into().into_boxed_str(),
            }),
        }
    }

    fn do_sync_query(&self, access_key: &str, bucket_name: &str) -> APIResult<ResponseBody> {
        let cache_result: APIResult<_> = self
            .inner
            .cache
            .entry(Self::cache_key(access_key, bucket_name))
            .and_modify(|cache_value| {
                if cache_value.cached_at.elapsed() > self.inner.cache_lifetime {
                    if let Ok(body) = self._do_sync_query(access_key, bucket_name) {
                        let cached_at = Instant::now();
                        *cache_value = CacheValue { body, cached_at };
                    }
                }
            })
            .or_try_insert_with(|| {
                let body = self._do_sync_query(access_key, bucket_name)?;
                let cached_at = Instant::now();
                Ok(CacheValue { body, cached_at })
            });
        let cache_value = cache_result?;
        Ok(cache_value.value().body.to_owned())
    }

    fn _do_sync_query(&self, access_key: &str, bucket_name: &str) -> APIResult<ResponseBody> {
        self.inner
            .http_client
            .get(ServiceName::Uc, self.inner.uc_endpoints.to_owned())
            .append_query_pair("ak", access_key)
            .append_query_pair("bucket", bucket_name)
            .accept_json()
            .call()?
            .parse_json()
    }

    #[cfg(feature = "async")]
    async fn do_async_query(&self, access_key: &str, bucket_name: &str) -> APIResult<ResponseBody> {
        let cache_result: APIResult<_> = self
            .inner
            .cache
            .entry(Self::cache_key(access_key, bucket_name))
            .and_modify(|cache_value| {
                if cache_value.cached_at.elapsed() > self.inner.cache_lifetime {
                    if let Ok(body) =
                        block_on(async { self._do_async_query(access_key, bucket_name).await })
                    {
                        let cached_at = Instant::now();
                        *cache_value = CacheValue { body, cached_at };
                    }
                }
            })
            .or_try_insert_with(|| {
                let body = block_on(async { self._do_async_query(access_key, bucket_name).await })?;
                let cached_at = Instant::now();
                Ok(CacheValue { body, cached_at })
            });
        let cache_value = cache_result?;
        Ok(cache_value.value().body.to_owned())
    }

    #[cfg(feature = "async")]
    async fn _do_async_query(
        &self,
        access_key: &str,
        bucket_name: &str,
    ) -> APIResult<ResponseBody> {
        self.inner
            .http_client
            .get(ServiceName::Uc, self.inner.uc_endpoints.to_owned())
            .append_query_pair("ak", access_key)
            .append_query_pair("bucket", bucket_name)
            .accept_json()
            .async_call()
            .await?
            .parse_json()
            .await
    }

    #[inline]
    fn cache_key(access_key: &str, bucket_name: &str) -> CacheKey {
        CacheKey {
            access_key: access_key.into(),
            bucket_name: bucket_name.into(),
        }
    }
}

impl BucketRegionsQueryerBuilder {
    #[inline]
    pub fn new(http_client: Client, uc_endpoints: impl Into<Vec<Endpoint>>) -> Self {
        Self {
            http_client,
            uc_endpoints: uc_endpoints.into(),
            cache_lifetime: DEFAULT_CACHE_LIFETIME,
        }
    }

    pub fn cache_lifetime(mut self, cache_lifetime: Duration) -> Self {
        self.cache_lifetime = cache_lifetime;
        self
    }

    pub fn build(self) -> BucketRegionsQueryer {
        BucketRegionsQueryer {
            inner: Arc::new(BucketRegionsQueryerInner {
                http_client: self.http_client,
                uc_endpoints: self.uc_endpoints.into_boxed_slice(),
                cache: Default::default(),
                cache_lifetime: self.cache_lifetime,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BucketRegionsProvider {
    inner: Arc<BucketRegionsProviderInner>,
}

#[derive(Debug)]
struct BucketRegionsProviderInner {
    queryer: BucketRegionsQueryer,
    access_key: Box<str>,
    bucket_name: Box<str>,
}

impl RegionProvider for BucketRegionsProvider {
    fn get(&self) -> APIResult<Region> {
        self.get_all().map(|regions| {
            regions
                .into_iter()
                .nth(0)
                .expect("Regions Query API returns empty regions")
        })
    }

    #[inline]
    fn get_all(&self) -> APIResult<Vec<Region>> {
        self.inner
            .queryer
            .do_sync_query(&self.inner.access_key, &self.inner.bucket_name)
            .and_then(|body| {
                body.hosts
                    .into_iter()
                    .map(|host| {
                        Region::try_from(host).map_err(|err| {
                            ResponseError::new(ResponseErrorKind::ParseResponseError, err)
                        })
                    })
                    .collect()
            })
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
                    .nth(0)
                    .expect("Regions Query API returns empty regions")
            })
        })
    }

    /// 异步返回多个七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get_all(&self) -> BoxFuture<APIResult<Vec<Region>>> {
        Box::pin(async move {
            self.inner
                .queryer
                .do_async_query(&self.inner.access_key, &self.inner.bucket_name)
                .await
                .and_then(|body| {
                    body.hosts
                        .into_iter()
                        .map(|host| {
                            Region::try_from(host).map_err(|err| {
                                ResponseError::new(ResponseErrorKind::ParseResponseError, err)
                            })
                        })
                        .collect()
                })
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_region_provider(&self) -> &dyn RegionProvider {
        self
    }
}
