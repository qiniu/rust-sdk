use super::{
    super::{
        super::{ApiResult, HttpClient, Response, ResponseError},
        cache_key::CacheKey,
        Endpoints, ServiceName,
    },
    regions_cache::RegionsCache,
    structs::ResponseBody,
    GetOptions, GotRegion, GotRegions, Region, RegionsProvider,
};
use qiniu_credential::AccessKey;
use qiniu_upload_token::BucketName;
use std::{convert::TryFrom, fmt::Debug, mem::take, path::Path, time::Duration};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

const DEFAULT_SHRINK_INTERVAL: Duration = Duration::from_secs(86400);
const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(86400);

/// 存储空间相关区域查询器
///
/// 查询存储空间相关区域，如果返回多个区域，则第一个区域是该存储空间所在区域，其他区域都是多活区域。
///
/// ### 存储空间域名查询器使用示例
///
/// ##### 阻塞代码示例
///
/// ```
/// # fn example() -> anyhow::Result<()> {
/// use qiniu_credential::Credential;
/// use qiniu_http_client::{Authorization, BucketRegionsQueryer, HttpClient, RegionsProviderEndpoints, ServiceName};
/// use serde_json::Value;
///
/// let credential = Credential::new("abcdefghklmnopq", "1234567890");
/// let value: Value = HttpClient::default()
///     .get(
///         &[ServiceName::Rs],
///         RegionsProviderEndpoints::new(
///             BucketRegionsQueryer::new().query(credential.access_key().to_owned(), "test-bucket"),
///         ),
///     )
///     .path("/stat/dGVzdC1idWNrZXQ6dGVzdC1rZXk=")
///     .authorization(Authorization::v2(credential))
///     .accept_json()
///     .call()?
///     .parse_json()?
///     .into_body();
/// # Ok(())
/// # }
/// ```
///
/// ##### 异步代码示例
///
/// ```
/// # async fn example() -> anyhow::Result<()> {
/// use qiniu_credential::Credential;
/// use qiniu_http_client::{Authorization, BucketRegionsQueryer, HttpClient, RegionsProviderEndpoints, ServiceName};
/// use serde_json::Value;
///
/// let credential = Credential::new("abcdefghklmnopq", "1234567890");
/// let value: Value = HttpClient::default()
///     .async_get(
///         &[ServiceName::Rs],
///         RegionsProviderEndpoints::new(
///             BucketRegionsQueryer::new().query(credential.access_key().to_owned(), "test-bucket"),
///         ),
///     )
///     .path("/stat/dGVzdC1idWNrZXQ6dGVzdC1rZXk=")
///     .authorization(Authorization::v2(credential))
///     .accept_json()
///     .call()
///     .await?
///     .parse_json()
///     .await?
///     .into_body();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct BucketRegionsQueryer {
    http_client: HttpClient,
    uc_endpoints: Endpoints,
    cache: RegionsCache,
}

/// 存储空间相关区域查询构建器
#[derive(Debug, Clone)]
pub struct BucketRegionsQueryerBuilder {
    http_client: Option<HttpClient>,
    uc_endpoints: Option<Endpoints>,
    cache_lifetime: Duration,
    shrink_interval: Duration,
}

impl BucketRegionsQueryer {
    /// 创建存储空间相关区域查询构建器
    #[inline]
    pub fn builder() -> BucketRegionsQueryerBuilder {
        BucketRegionsQueryerBuilder::new()
    }

    /// 创建存储空间相关区域查询器
    #[inline]
    pub fn new() -> BucketRegionsQueryer {
        BucketRegionsQueryerBuilder::new().build()
    }

    /// 查询存储空间相关区域
    pub fn query(&self, access_key: impl Into<AccessKey>, bucket_name: impl Into<BucketName>) -> BucketRegionsProvider {
        let access_key = access_key.into();
        let bucket_name = bucket_name.into();
        BucketRegionsProvider {
            queryer: self.to_owned(),
            access_key: access_key.to_owned(),
            bucket_name: bucket_name.to_owned(),
            cache_key: CacheKey::new_from_endpoint_and_ak_and_bucket(&self.uc_endpoints, bucket_name, access_key),
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
    /// 创建存储空间相关区域查询构建器
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 HTTP 客户端
    #[inline]
    pub fn http_client(&mut self, http_client: HttpClient) -> &mut Self {
        self.http_client = Some(http_client);
        self
    }

    /// 是否启用 HTTPS 协议
    ///
    /// 默认为 HTTPS 协议
    pub fn use_https(&mut self, use_https: bool) -> &mut Self {
        self.http_client(HttpClient::build_default().use_https(use_https).build())
    }

    /// 设置存储空间管理终端地址列表
    #[inline]
    pub fn uc_endpoints(&mut self, uc_endpoints: impl Into<Endpoints>) -> &mut Self {
        self.uc_endpoints = Some(uc_endpoints.into());
        self
    }

    /// 缓存时长
    #[inline]
    pub fn cache_lifetime(&mut self, cache_lifetime: Duration) -> &mut Self {
        self.cache_lifetime = cache_lifetime;
        self
    }

    /// 清理间隔时长
    #[inline]
    pub fn shrink_interval(&mut self, shrink_interval: Duration) -> &mut Self {
        self.shrink_interval = shrink_interval;
        self
    }

    /// 从文件系统加载或构建存储空间相关区域查询器
    ///
    /// 可以选择是否启用自动持久化缓存功能
    pub fn load_or_create_from(&mut self, path: impl AsRef<Path>, auto_persistent: bool) -> BucketRegionsQueryer {
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

    /// 从默认文件系统路径加载或构建存储空间相关区域查询器，并启用自动持久化缓存功能
    #[inline]
    pub fn build(&mut self) -> BucketRegionsQueryer {
        self.default_load_or_create_from(true)
    }

    /// 从默认文件系统路径加载或构建存储空间相关区域查询器
    ///
    /// 可以选择是否启用自动持久化缓存功能
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

    /// 构建存储空间相关区域查询器
    ///
    /// 不启用文件系统持久化缓存
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

/// 存储空间相关区域获取器
#[derive(Debug, Clone)]
pub struct BucketRegionsProvider {
    queryer: BucketRegionsQueryer,
    cache_key: CacheKey,
    access_key: AccessKey,
    bucket_name: BucketName,
}

impl RegionsProvider for BucketRegionsProvider {
    fn get(&self, opts: GetOptions) -> ApiResult<GotRegion> {
        self.get_all(opts)
            .map(|regions| regions.try_into().expect("Regions Query API returns empty regions"))
    }

    #[inline]
    fn get_all(&self, _opts: GetOptions) -> ApiResult<GotRegions> {
        self.queryer.cache.get(&self.cache_key, || self.do_sync_query())
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get(&self, opts: GetOptions) -> BoxFuture<'_, ApiResult<GotRegion>> {
        Box::pin(async move {
            self.async_get_all(opts)
                .await
                .map(|regions| regions.try_into().expect("Regions Query API returns empty regions"))
        })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get_all(&self, _opts: GetOptions) -> BoxFuture<'_, ApiResult<GotRegions>> {
        Box::pin(async move {
            self.queryer
                .cache
                .async_get(&self.cache_key, self.do_async_query())
                .await
        })
    }
}

impl BucketRegionsProvider {
    fn do_sync_query(&self) -> ApiResult<GotRegions> {
        handle_response_body(
            self.queryer
                .http_client
                .get(&[ServiceName::Uc, ServiceName::Api], &self.queryer.uc_endpoints)
                .path("/v4/query")
                .append_query_pair("ak", self.access_key.as_str())
                .append_query_pair("bucket", self.bucket_name.as_str())
                .accept_json()
                .call()?
                .parse_json::<ResponseBody>()?,
        )
    }

    #[cfg(feature = "async")]
    async fn do_async_query(&self) -> ApiResult<GotRegions> {
        handle_response_body(
            self.queryer
                .http_client
                .async_get(&[ServiceName::Uc, ServiceName::Api], &self.queryer.uc_endpoints)
                .path("/v4/query")
                .append_query_pair("ak", self.access_key.as_str())
                .append_query_pair("bucket", self.bucket_name.as_str())
                .accept_json()
                .call()
                .await?
                .parse_json::<ResponseBody>()
                .await?,
        )
    }
}

fn handle_response_body(response: Response<ResponseBody>) -> ApiResult<GotRegions> {
    let (parts, body) = response.into_parts_and_body();
    let hosts = body.into_hosts();
    let min_lifetime = hosts.iter().map(|host| host.lifetime()).min();
    let mut got_regions = hosts
        .into_iter()
        .map(|host| Region::try_from(host).map_err(|err| ResponseError::from_endpoint_parse_error(err, &parts)))
        .collect::<ApiResult<GotRegions>>()?;
    *got_regions.lifetime_mut() = min_lifetime;
    Ok(got_regions)
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
            let ($addr, server) = warp::serve($routes).bind_with_graceful_shutdown(([127, 0, 0, 1], 0), async move {
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
        let routes = path!("v4" / "query").and(warp::query::<UcQueryParams>()).map({
            let called = called.to_owned();
            move |params: UcQueryParams| {
                assert_eq!(&params.ak, ACCESS_KEY);
                assert_eq!(&params.bucket, BUCKET_NAME);
                called.fetch_add(1, Relaxed);
                let mut response = Response::new(get_response_json_body().to_string().into_bytes().into());
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
                let got_regions = provider.async_get_all(Default::default()).await?;
                assert_eq!(got_regions.lifetime(), Some(Duration::from_secs(5)));
                let mut regions = got_regions.into_regions().into_iter();
                assert_eq!(regions.len(), 2);
                let region = regions.next().unwrap();
                assert_eq!(region.region_id(), "z0");
                assert_eq!(region.s3_region_id(), "cn-east-1");
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
                assert_eq!(region.s3_region_id(), "cn-north-1");
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
              "ttl": 5,
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
              "ttl": 5,
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
