use super::{
    regions_cache::{CacheKey, RegionsCache},
    {
        super::{
            super::{
                APIResult, CacheController, HTTPClient, PersistentResult, ResponseError,
                ResponseErrorKind,
            },
            Endpoints, ServiceName,
        },
        structs::ResponseBody,
        Region, RegionProvider,
    },
};
use qiniu_credential::AccessKey;
use qiniu_upload_token::BucketName;
use std::{any::Any, convert::TryFrom, fmt::Debug, path::Path, sync::Arc, time::Duration};

#[cfg(feature = "async")]
use {async_std::task::spawn, futures::future::BoxFuture};

const DEFAULT_SHRINK_INTERVAL: Duration = Duration::from_secs(86400);
const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(86400);

#[derive(Debug, Clone)]
pub struct BucketRegionsQueryer {
    inner: Arc<BucketRegionsQueryerInner>,
}

#[derive(Debug)]
struct BucketRegionsQueryerInner {
    http_client: HTTPClient,
    uc_endpoints: Endpoints,
    cache: RegionsCache,
}

#[derive(Debug)]
pub struct BucketRegionsQueryerBuilder {
    http_client: HTTPClient,
    uc_endpoints: Endpoints,
    cache_lifetime: Duration,
    shrink_interval: Duration,
}

impl BucketRegionsQueryer {
    #[inline]
    pub fn builder(
        http_client: HTTPClient,
        uc_endpoints: impl Into<Endpoints>,
    ) -> BucketRegionsQueryerBuilder {
        BucketRegionsQueryerBuilder::new(http_client, uc_endpoints.into())
    }

    #[inline]
    pub fn query(
        &self,
        access_key: impl Into<AccessKey>,
        bucket_name: impl Into<BucketName>,
    ) -> BucketRegionsProvider {
        let access_key = access_key.into();
        let bucket_name = bucket_name.into();
        BucketRegionsProvider {
            inner: Arc::new(BucketRegionsProviderInner {
                queryer: self.to_owned(),
                access_key: access_key.to_owned(),
                bucket_name: bucket_name.to_owned(),
                cache_key: CacheKey::new_from_endpoint_and_ak_and_bucket(
                    &self.inner.uc_endpoints,
                    bucket_name,
                    access_key,
                ),
            }),
        }
    }
}

impl BucketRegionsQueryerBuilder {
    #[inline]
    pub fn new(http_client: HTTPClient, uc_endpoints: impl Into<Endpoints>) -> Self {
        Self {
            http_client,
            uc_endpoints: uc_endpoints.into(),
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
    ) -> PersistentResult<BucketRegionsQueryer> {
        Ok(BucketRegionsQueryer {
            inner: Arc::new(BucketRegionsQueryerInner {
                cache: RegionsCache::load_or_create_from(
                    path.as_ref(),
                    auto_persistent,
                    self.cache_lifetime,
                    self.shrink_interval,
                )?,
                http_client: self.http_client,
                uc_endpoints: self.uc_endpoints,
            }),
        })
    }

    #[inline]
    pub fn default_load_or_create_from(
        self,
        auto_persistent: bool,
    ) -> PersistentResult<BucketRegionsQueryer> {
        Ok(BucketRegionsQueryer {
            inner: Arc::new(BucketRegionsQueryerInner {
                cache: RegionsCache::default_load_or_create_from(
                    auto_persistent,
                    self.cache_lifetime,
                    self.shrink_interval,
                )?,
                http_client: self.http_client,
                uc_endpoints: self.uc_endpoints,
            }),
        })
    }

    #[inline]
    pub fn in_memory(self) -> BucketRegionsQueryer {
        BucketRegionsQueryer {
            inner: Arc::new(BucketRegionsQueryerInner {
                cache: RegionsCache::in_memory(self.cache_lifetime, self.shrink_interval),
                http_client: self.http_client,
                uc_endpoints: self.uc_endpoints,
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
    cache_key: CacheKey,
    access_key: AccessKey,
    bucket_name: BucketName,
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
        let provider = self.to_owned();
        self.inner
            .queryer
            .inner
            .cache
            .get(&self.inner.cache_key, move || provider.do_sync_query())
    }

    /// 异步返回七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get(&self) -> BoxFuture<APIResult<Region>> {
        let provider = self.to_owned();
        Box::pin(async move { spawn(async move { provider.get() }).await })
    }

    /// 异步返回多个七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get_all(&self) -> BoxFuture<APIResult<Vec<Region>>> {
        let provider = self.to_owned();
        Box::pin(async move { spawn(async move { provider.get_all() }).await })
    }

    #[inline]
    fn cache_controller(&self) -> Option<&dyn CacheController> {
        Some(&self.inner.queryer.inner.cache)
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_region_provider(&self) -> &dyn RegionProvider {
        self
    }
}

impl BucketRegionsProvider {
    fn do_sync_query(&self) -> APIResult<Vec<Region>> {
        let body: ResponseBody = self
            .inner
            .queryer
            .inner
            .http_client
            .get(
                &[ServiceName::Uc, ServiceName::Api],
                self.inner.queryer.inner.uc_endpoints.to_owned(),
            )
            .path("/v4/query")
            .append_query_pair("ak", self.inner.access_key.as_str())
            .append_query_pair("bucket", self.inner.bucket_name.as_str())
            .accept_json()
            .call()?
            .parse_json()?
            .into_body();
        body.into_hosts()
            .into_iter()
            .map(|host| {
                Region::try_from(host)
                    .map_err(|err| ResponseError::new(ResponseErrorKind::ParseResponseError, err))
            })
            .collect()
    }
}

#[cfg(all(test, feature = "isahc", feature = "async"))]
mod tests {
    use super::{super::super::Endpoint, *};
    use futures::channel::oneshot::channel;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value as JSONValue};
    use std::{
        error::Error,
        result::Result,
        str::FromStr,
        sync::atomic::{AtomicUsize, Ordering::Relaxed},
    };
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
    async fn test_query_regions_of_bucket() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        const ACCESS_KEY: &str = "0123456789001234567890";
        const BUCKET_NAME: &str = "test-bucket";

        let called = Arc::new(AtomicUsize::new(0));
        let routes = path!("v4" / "query")
            .and(warp::query::<UcQueryParams>())
            .map({
                let called = called.to_owned();
                move |params: UcQueryParams| {
                    assert_eq!(&params.ak, ACCESS_KEY);
                    assert_eq!(&params.bucket, BUCKET_NAME);
                    called.fetch_add(1, Relaxed);
                    let mut response =
                        Response::new(get_response_json_body().to_string().into_bytes().into());
                    response
                        .headers_mut()
                        .insert("X-Reqid", HeaderValue::from_static("FAKE_REQ_ID"));
                    response
                }
            });

        starts_with_server!(addr, routes, {
            let queryer = BucketRegionsQueryer::builder(
                HTTPClient::build_isahc()?.use_https(false).build(),
                vec![Endpoint::from(addr)],
            )
            .in_memory();

            for _ in 0..2 {
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
            }

            assert_eq!(called.load(Relaxed), 1);
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
