use super::{
    super::{
        super::{ApiResult, CacheController, HttpClient, ResponseError},
        Endpoints, ServiceName,
    },
    cache_key::CacheKey,
    regions_cache::RegionsCache,
    structs::ResponseBody,
    GetOptions, GotRegion, GotRegions, Region, RegionProvider,
};
use qiniu_credential::AccessKey;
use qiniu_upload_token::BucketName;
use std::{convert::TryFrom, fmt::Debug, mem::take, path::Path, time::Duration};

#[cfg(feature = "async")]
use {async_std::task::spawn, futures::future::BoxFuture};

const DEFAULT_SHRINK_INTERVAL: Duration = Duration::from_secs(86400);
const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(86400);

#[derive(Debug, Clone)]
pub struct BucketRegionsQueryer {
    http_client: HttpClient,
    uc_endpoints: Endpoints,
    cache: RegionsCache,
}

#[derive(Debug, Clone)]
pub struct BucketRegionsQueryerBuilder {
    http_client: Option<HttpClient>,
    uc_endpoints: Option<Endpoints>,
    cache_lifetime: Duration,
    shrink_interval: Duration,
}

impl BucketRegionsQueryer {
    #[inline]
    pub fn builder() -> BucketRegionsQueryerBuilder {
        BucketRegionsQueryerBuilder::new()
    }

    pub fn query(
        &self,
        access_key: impl Into<AccessKey>,
        bucket_name: impl Into<BucketName>,
    ) -> BucketRegionsProvider {
        let access_key = access_key.into();
        let bucket_name = bucket_name.into();
        BucketRegionsProvider {
            queryer: self.to_owned(),
            access_key: access_key.to_owned(),
            bucket_name: bucket_name.to_owned(),
            cache_key: CacheKey::new_from_endpoint_and_ak_and_bucket(
                &self.uc_endpoints,
                bucket_name,
                access_key,
            ),
        }
    }
}

impl Default for BucketRegionsQueryer {
    #[inline]
    fn default() -> Self {
        Self::builder().default_load_or_create_from(true)
    }
}

impl Default for BucketRegionsQueryerBuilder {
    #[inline]
    fn default() -> Self {
        Self {
            http_client: None,
            uc_endpoints: None,
            cache_lifetime: DEFAULT_CACHE_LIFETIME,
            shrink_interval: DEFAULT_SHRINK_INTERVAL,
        }
    }
}

impl BucketRegionsQueryerBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn http_client(&mut self, http_client: HttpClient) -> &mut Self {
        self.http_client = Some(http_client);
        self
    }

    #[inline]
    pub fn uc_endpoints(&mut self, uc_endpoints: impl Into<Endpoints>) -> &mut Self {
        self.uc_endpoints = Some(uc_endpoints.into());
        self
    }

    #[inline]
    pub fn cache_lifetime(&mut self, cache_lifetime: Duration) -> &mut Self {
        self.cache_lifetime = cache_lifetime;
        self
    }

    #[inline]
    pub fn shrink_interval(&mut self, shrink_interval: Duration) -> &mut Self {
        self.shrink_interval = shrink_interval;
        self
    }

    pub fn load_or_create_from(
        &mut self,
        path: impl AsRef<Path>,
        auto_persistent: bool,
    ) -> BucketRegionsQueryer {
        let owned = take(self);
        BucketRegionsQueryer {
            cache: RegionsCache::load_or_create_from(
                path.as_ref(),
                auto_persistent,
                owned.cache_lifetime,
                owned.shrink_interval,
            ),
            http_client: owned.http_client.unwrap_or_default(),
            uc_endpoints: owned
                .uc_endpoints
                .unwrap_or_else(|| Endpoints::public_uc_endpoints().to_owned()),
        }
    }

    #[inline]
    pub fn build(&mut self) -> BucketRegionsQueryer {
        self.default_load_or_create_from(true)
    }

    pub fn default_load_or_create_from(&mut self, auto_persistent: bool) -> BucketRegionsQueryer {
        let owned = take(self);
        BucketRegionsQueryer {
            cache: RegionsCache::default_load_or_create_from(
                auto_persistent,
                owned.cache_lifetime,
                owned.shrink_interval,
            ),
            http_client: owned.http_client.unwrap_or_default(),
            uc_endpoints: owned
                .uc_endpoints
                .unwrap_or_else(|| Endpoints::public_uc_endpoints().to_owned()),
        }
    }

    pub fn in_memory(&mut self) -> BucketRegionsQueryer {
        let owned = take(self);
        BucketRegionsQueryer {
            cache: RegionsCache::in_memory(owned.cache_lifetime, owned.shrink_interval),
            http_client: owned.http_client.unwrap_or_default(),
            uc_endpoints: owned
                .uc_endpoints
                .unwrap_or_else(|| Endpoints::public_uc_endpoints().to_owned()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BucketRegionsProvider {
    queryer: BucketRegionsQueryer,
    cache_key: CacheKey,
    access_key: AccessKey,
    bucket_name: BucketName,
}

impl RegionProvider for BucketRegionsProvider {
    fn get(&self, opts: &GetOptions) -> ApiResult<GotRegion> {
        self.get_all(opts).map(|regions| {
            regions
                .into_regions()
                .into_iter()
                .next()
                .expect("Regions Query API returns empty regions")
                .into()
        })
    }

    #[inline]
    fn get_all(&self, _opts: &GetOptions) -> ApiResult<GotRegions> {
        let provider = self.to_owned();
        self.queryer
            .cache
            .get(&self.cache_key, move || provider.do_sync_query())
            .map(GotRegions::from)
    }

    /// 异步返回七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get<'a>(&'a self, opts: &'a GetOptions) -> BoxFuture<'a, ApiResult<GotRegion>> {
        let provider = self.to_owned();
        let opts = opts.to_owned();
        Box::pin(async move { spawn(async move { provider.get(&opts) }).await })
    }

    /// 异步返回多个七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get_all<'a>(&'a self, opts: &'a GetOptions) -> BoxFuture<'a, ApiResult<GotRegions>> {
        let provider = self.to_owned();
        let opts = opts.to_owned();
        Box::pin(async move { spawn(async move { provider.get_all(&opts) }).await })
    }

    #[inline]
    fn cache_controller(&self) -> Option<&dyn CacheController> {
        Some(&self.queryer.cache)
    }
}

impl BucketRegionsProvider {
    fn do_sync_query(&self) -> ApiResult<Vec<Region>> {
        let (parts, body) = self
            .queryer
            .http_client
            .get(
                &[ServiceName::Uc, ServiceName::Api],
                &self.queryer.uc_endpoints,
            )
            .path("/v4/query")
            .append_query_pair("ak", self.access_key.as_str())
            .append_query_pair("bucket", self.bucket_name.as_str())
            .accept_json()
            .call()?
            .parse_json::<ResponseBody>()?
            .into_parts();
        body.into_hosts()
            .into_iter()
            .map(|host| {
                Region::try_from(host)
                    .map_err(|err| ResponseError::from_endpoint_parse_error(err, &parts))
            })
            .collect()
    }
}

#[cfg(all(test, feature = "isahc", feature = "async"))]
mod tests {
    use super::{super::super::Endpoint, *};
    use futures::channel::oneshot::channel;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value as JsonValue};
    use std::{
        error::Error,
        result::Result,
        str::FromStr,
        sync::{
            atomic::{AtomicUsize, Ordering::Relaxed},
            Arc,
        },
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
            let queryer = BucketRegionsQueryer::builder()
                .http_client(HttpClient::build_isahc()?.use_https(false).build())
                .uc_endpoints(vec![Endpoint::from(addr)])
                .in_memory();

            for _ in 0..2 {
                let provider = queryer.query(ACCESS_KEY, BUCKET_NAME);
                let mut regions = provider
                    .async_get_all(&Default::default())
                    .await?
                    .into_regions()
                    .into_iter();
                assert_eq!(regions.len(), 2);
                let region = regions.next().unwrap();
                assert_eq!(region.region_id(), "z0");
                assert!(region.s3_region_id().is_empty());
                assert_eq!(
                    region.up_preferred_endpoints(),
                    &[
                        Endpoint::from_str("upload.qiniup.com").unwrap(),
                        Endpoint::from_str("up.qiniup.com").unwrap()
                    ]
                );
                assert_eq!(
                    region.up_alternative_endpoints(),
                    &[
                        Endpoint::from_str("upload.qbox.me").unwrap(),
                        Endpoint::from_str("up.qbox.me").unwrap()
                    ]
                );
                assert_eq!(
                    region.io_preferred_endpoints(),
                    &[Endpoint::from_str("iovip.qbox.me").unwrap(),]
                );
                assert!(region.io_alternative_endpoints().is_empty());
                assert_eq!(
                    region.uc_preferred_endpoints(),
                    &[Endpoint::from_str("uc.qbox.me").unwrap(),]
                );
                assert!(region.uc_alternative_endpoints().is_empty());
                assert_eq!(
                    region.rs_preferred_endpoints(),
                    &[Endpoint::from_str("rs-z0.qbox.me").unwrap(),]
                );
                assert!(region.rs_alternative_endpoints().is_empty());
                assert_eq!(
                    region.rsf_preferred_endpoints(),
                    &[Endpoint::from_str("rsf-z0.qbox.me").unwrap(),]
                );
                assert!(region.rsf_alternative_endpoints().is_empty());
                assert_eq!(
                    region.api_preferred_endpoints(),
                    &[Endpoint::from_str("api.qiniu.com").unwrap(),]
                );
                assert!(region.api_alternative_endpoints().is_empty());
                assert_eq!(
                    region.s3_preferred_endpoints(),
                    &[Endpoint::from_str("s3-cn-east-1.qiniucs.com").unwrap(),]
                );
                assert!(region.s3_alternative_endpoints().is_empty());
                let region = regions.next().unwrap();
                assert_eq!(region.region_id(), "z1");
                assert!(region.s3_region_id().is_empty());
                assert_eq!(
                    region.up_preferred_endpoints(),
                    &[
                        Endpoint::from_str("upload-z1.qiniup.com").unwrap(),
                        Endpoint::from_str("up-z1.qiniup.com").unwrap()
                    ]
                );
                assert_eq!(
                    region.up_alternative_endpoints(),
                    &[
                        Endpoint::from_str("upload-z1.qbox.me").unwrap(),
                        Endpoint::from_str("up-z1.qbox.me").unwrap()
                    ]
                );
                assert_eq!(
                    region.io_preferred_endpoints(),
                    &[Endpoint::from_str("iovip-z1.qbox.me").unwrap(),]
                );
                assert!(region.io_alternative_endpoints().is_empty());
                assert_eq!(
                    region.uc_preferred_endpoints(),
                    &[Endpoint::from_str("uc.qbox.me").unwrap(),]
                );
                assert!(region.uc_alternative_endpoints().is_empty());
                assert_eq!(
                    region.rs_preferred_endpoints(),
                    &[Endpoint::from_str("rs-z1.qbox.me").unwrap(),]
                );
                assert!(region.rs_alternative_endpoints().is_empty());
                assert_eq!(
                    region.rsf_preferred_endpoints(),
                    &[Endpoint::from_str("rsf-z1.qbox.me").unwrap(),]
                );
                assert!(region.rsf_alternative_endpoints().is_empty());
                assert_eq!(
                    region.api_preferred_endpoints(),
                    &[Endpoint::from_str("api.qiniu.com").unwrap(),]
                );
                assert!(region.api_alternative_endpoints().is_empty());
                assert_eq!(
                    region.s3_preferred_endpoints(),
                    &[Endpoint::from_str("s3-cn-north-1.qiniucs.com").unwrap(),]
                );
                assert!(region.s3_alternative_endpoints().is_empty());
            }

            assert_eq!(called.load(Relaxed), 1);
        });
        Ok(())
    }

    fn get_response_json_body() -> JsonValue {
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
