use super::{
    super::{
        super::{APIResult, Client, ResponseError, ResponseErrorKind},
        Endpoint, ServiceName,
    },
    structs::{ResponseBody, DEFAULT_CACHE_LIFETIME},
    Region, RegionProvider,
};
use dashmap::DashMap;
use std::{
    any::Any,
    convert::TryFrom,
    fmt::Debug,
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(feature = "async")]
use {async_std::task::spawn_blocking, futures::future::BoxFuture};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct CacheKey {
    bucket_name: Box<str>,
    access_key: Box<str>,
}

#[derive(Debug, Clone)]
struct CacheValue {
    body: Vec<Region>,
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

#[derive(Debug)]
pub struct BucketRegionsQueryerBuilder {
    http_client: Client,
    uc_endpoints: Vec<Endpoint>,
    cache_lifetime: Duration,
}

impl BucketRegionsQueryer {
    #[inline]
    pub fn builder(
        http_client: Client,
        uc_endpoints: impl Into<Vec<Endpoint>>,
    ) -> BucketRegionsQueryerBuilder {
        BucketRegionsQueryerBuilder::new(http_client, uc_endpoints)
    }

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

    fn do_sync_query(&self, access_key: &str, bucket_name: &str) -> APIResult<Vec<Region>> {
        let cache_result: APIResult<_> = self
            .inner
            .cache
            .entry(cache_key(access_key, bucket_name))
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
        return Ok(cache_value.value().body.to_owned());

        #[inline]
        fn cache_key(access_key: &str, bucket_name: &str) -> CacheKey {
            CacheKey {
                access_key: access_key.into(),
                bucket_name: bucket_name.into(),
            }
        }
    }

    fn _do_sync_query(&self, access_key: &str, bucket_name: &str) -> APIResult<Vec<Region>> {
        let body: ResponseBody = self
            .inner
            .http_client
            .get(ServiceName::Uc, self.inner.uc_endpoints.to_owned())
            .path("/v4/query")
            .append_query_pair("ak", access_key)
            .append_query_pair("bucket", bucket_name)
            .accept_json()
            .call()?
            .parse_json()?;
        body.into_hosts()
            .into_iter()
            .map(|host| {
                Region::try_from(host)
                    .map_err(|err| ResponseError::new(ResponseErrorKind::ParseResponseError, err))
            })
            .collect()
    }

    #[cfg(feature = "async")]
    async fn do_async_query(&self, access_key: &str, bucket_name: &str) -> APIResult<Vec<Region>> {
        let ctx = self.to_owned();
        let access_key = access_key.to_owned();
        let bucket_name = bucket_name.to_owned();

        spawn_blocking(move || ctx.do_sync_query(&access_key, &bucket_name)).await
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
                .next()
                .expect("Regions Query API returns empty regions")
        })
    }

    #[inline]
    fn get_all(&self) -> APIResult<Vec<Region>> {
        self.inner
            .queryer
            .do_sync_query(&self.inner.access_key, &self.inner.bucket_name)
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
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_region_provider(&self) -> &dyn RegionProvider {
        self
    }
}

#[cfg(all(test, feature = "curl", feature = "async"))]
mod tests {
    use super::*;
    use futures::channel::oneshot::channel;
    use serde::Serialize;
    use serde_json::{json, Value as JSONValue};
    use std::{error::Error, result::Result, str::FromStr};
    use tokio::task::spawn;
    use warp::{http::header::HeaderValue, path, reply::Response, Filter};

    macro_rules! starts_with_server {
        ($addr:ident, $routes:ident, $code:block) => {{
            let (tx, rx) = channel();
            let ($addr, server) =
                warp::serve($routes).bind_with_graceful_shutdown(([127, 0, 0, 1], 0), async move {
                    rx.await.ok();
                });
            let handler = spawn(server);
            $code;
            tx.send(()).ok();
            handler.await.ok();
        }};
    }

    #[derive(Deserialize, Serialize)]
    struct UcQueryParams {
        ak: String,
        bucket: String,
    }

    #[tokio::test]
    async fn test_get_all_regions() -> Result<(), Box<dyn Error>> {
        const ACCESS_KEY: &str = "0123456789001234567890";
        const BUCKET_NAME: &str = "test-bucket";

        let routes = path!("v4" / "query")
            .and(warp::query::<UcQueryParams>())
            .map(move |params: UcQueryParams| {
                assert_eq!(&params.ak, ACCESS_KEY);
                assert_eq!(&params.bucket, BUCKET_NAME);
                let mut response =
                    Response::new(get_response_json_body().to_string().into_bytes().into());
                response
                    .headers_mut()
                    .insert("X-Reqid", HeaderValue::from_static("FAKE_REQ_ID"));
                response
            });

        starts_with_server!(addr, routes, {
            let queryer = BucketRegionsQueryer::builder(
                Client::builder().use_https(false).build(),
                vec![Endpoint::from(addr)],
            )
            .build();
            let provider = queryer.query(ACCESS_KEY, BUCKET_NAME);
            let mut regions = provider.async_get_all().await?.into_iter();
            assert_eq!(regions.len(), 2);
            let region = regions.next().unwrap();
            assert_eq!(region.region_id(), "z0");
            assert!(region.s3_region_id().is_empty());
            assert_eq!(
                region.up_endpoints(),
                &[
                    Endpoint::from_str("upload.qiniup.com").unwrap(),
                    Endpoint::from_str("up.qiniup.com").unwrap()
                ]
            );
            assert_eq!(
                region.up_old_endpoints(),
                &[
                    Endpoint::from_str("upload.qbox.me").unwrap(),
                    Endpoint::from_str("up.qbox.me").unwrap()
                ]
            );
            assert_eq!(
                region.io_endpoints(),
                &[Endpoint::from_str("iovip.qbox.me").unwrap(),]
            );
            assert!(region.io_old_endpoints().is_empty());
            assert_eq!(
                region.uc_endpoints(),
                &[Endpoint::from_str("uc.qbox.me").unwrap(),]
            );
            assert!(region.uc_old_endpoints().is_empty());
            assert_eq!(
                region.rs_endpoints(),
                &[Endpoint::from_str("rs-z0.qbox.me").unwrap(),]
            );
            assert!(region.rs_old_endpoints().is_empty());
            assert_eq!(
                region.rsf_endpoints(),
                &[Endpoint::from_str("rsf-z0.qbox.me").unwrap(),]
            );
            assert!(region.rsf_old_endpoints().is_empty());
            assert_eq!(
                region.api_endpoints(),
                &[Endpoint::from_str("api.qiniu.com").unwrap(),]
            );
            assert!(region.api_old_endpoints().is_empty());
            assert_eq!(
                region.s3_endpoints(),
                &[Endpoint::from_str("s3-cn-east-1.qiniucs.com").unwrap(),]
            );
            assert!(region.s3_old_endpoints().is_empty());
            let region = regions.next().unwrap();
            assert_eq!(region.region_id(), "z1");
            assert!(region.s3_region_id().is_empty());
            assert_eq!(
                region.up_endpoints(),
                &[
                    Endpoint::from_str("upload-z1.qiniup.com").unwrap(),
                    Endpoint::from_str("up-z1.qiniup.com").unwrap()
                ]
            );
            assert_eq!(
                region.up_old_endpoints(),
                &[
                    Endpoint::from_str("upload-z1.qbox.me").unwrap(),
                    Endpoint::from_str("up-z1.qbox.me").unwrap()
                ]
            );
            assert_eq!(
                region.io_endpoints(),
                &[Endpoint::from_str("iovip-z1.qbox.me").unwrap(),]
            );
            assert!(region.io_old_endpoints().is_empty());
            assert_eq!(
                region.uc_endpoints(),
                &[Endpoint::from_str("uc.qbox.me").unwrap(),]
            );
            assert!(region.uc_old_endpoints().is_empty());
            assert_eq!(
                region.rs_endpoints(),
                &[Endpoint::from_str("rs-z1.qbox.me").unwrap(),]
            );
            assert!(region.rs_old_endpoints().is_empty());
            assert_eq!(
                region.rsf_endpoints(),
                &[Endpoint::from_str("rsf-z1.qbox.me").unwrap(),]
            );
            assert!(region.rsf_old_endpoints().is_empty());
            assert_eq!(
                region.api_endpoints(),
                &[Endpoint::from_str("api.qiniu.com").unwrap(),]
            );
            assert!(region.api_old_endpoints().is_empty());
            assert_eq!(
                region.s3_endpoints(),
                &[Endpoint::from_str("s3-cn-north-1.qiniucs.com").unwrap(),]
            );
            assert!(region.s3_old_endpoints().is_empty());
        });
        Ok(())
    }

    fn get_response_json_body() -> JSONValue {
        json!({
          "hosts": [
            {
              "region": "z0",
              "ttl": 86400,
              "io": {
                "domains": [
                  "iovip.qbox.me"
                ]
              },
              "up": {
                "domains": [
                  "upload.qiniup.com",
                  "up.qiniup.com"
                ],
                "old": [
                  "upload.qbox.me",
                  "up.qbox.me"
                ]
              },
              "uc": {
                "domains": [
                  "uc.qbox.me"
                ]
              },
              "rs": {
                "domains": [
                  "rs-z0.qbox.me"
                ]
              },
              "rsf": {
                "domains": [
                  "rsf-z0.qbox.me"
                ]
              },
              "api": {
                "domains": [
                  "api.qiniu.com"
                ]
              },
              "s3": {
                "domains": [
                  "s3-cn-east-1.qiniucs.com"
                ],
                "region_alias": "cn-east-1"
              }
            },
            {
              "region": "z1",
              "ttl": 86400,
              "io": {
                "domains": [
                  "iovip-z1.qbox.me"
                ]
              },
              "up": {
                "domains": [
                  "upload-z1.qiniup.com",
                  "up-z1.qiniup.com"
                ],
                "old": [
                  "upload-z1.qbox.me",
                  "up-z1.qbox.me"
                ]
              },
              "uc": {
                "domains": [
                  "uc.qbox.me"
                ]
              },
              "rs": {
                "domains": [
                  "rs-z1.qbox.me"
                ]
              },
              "rsf": {
                "domains": [
                  "rsf-z1.qbox.me"
                ]
              },
              "api": {
                "domains": [
                  "api.qiniu.com"
                ]
              },
              "s3": {
                "domains": [
                  "s3-cn-north-1.qiniucs.com"
                ],
                "region_alias": "cn-north-1"
              }
            }
          ]
        })
    }
}
