use super::{
    super::{
        super::{ApiResult, Endpoints, HttpClient},
        cache_key::CacheKey,
    },
    regions_cache::RegionsCache,
    GetOptions, GotRegion, GotRegions, RegionsProvider,
};
use qiniu_credential::CredentialProvider;
use std::{path::Path, time::Duration};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

const DEFAULT_SHRINK_INTERVAL: Duration = Duration::from_secs(86400);
const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(86400);

/// 七牛所有区域信息查询器
///
/// ### 七牛所有区域信息查询器使用示例
///
/// ##### 阻塞代码示例
///
/// ```
/// use qiniu_credential::Credential;
/// use qiniu_http_client::{AllRegionsProvider, RegionsProvider};
///
/// # fn example() -> anyhow::Result<()> {
/// let credential = Credential::new("abcdefghklmnopq", "1234567890");
/// let regions = AllRegionsProvider::builder(credential)
///     .build()
///     .get_all(Default::default())?;
/// #     Ok(())
/// # }
/// ```
///
/// ##### 异步代码示例
///
/// ```
/// # async fn example() -> anyhow::Result<()> {
/// use qiniu_credential::Credential;
/// use qiniu_http_client::{AllRegionsProvider, RegionsProvider};
///
/// let credential = Credential::new("abcdefghklmnopq", "1234567890");
/// let regions = AllRegionsProvider::builder(credential)
///     .build()
///     .async_get_all(Default::default())
///     .await?;
/// #     Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct AllRegionsProvider {
    cache_key: CacheKey,
    provider: inner::AllRegionsProvider,
    cache: RegionsCache,
}

impl AllRegionsProvider {
    /// 创建七牛所有区域信息查询器
    #[inline]
    pub fn new(credential_provider: impl CredentialProvider + 'static) -> Self {
        Self::builder(credential_provider).build()
    }

    /// 构建七牛所有区域信息查询器
    #[inline]
    pub fn builder(credential_provider: impl CredentialProvider + 'static) -> AllRegionsProviderBuilder {
        AllRegionsProviderBuilder {
            credential_provider: Box::new(credential_provider),
            cache_lifetime: DEFAULT_CACHE_LIFETIME,
            shrink_interval: DEFAULT_SHRINK_INTERVAL,
            uc_endpoints: None,
            http_client: None,
        }
    }
}

impl RegionsProvider for AllRegionsProvider {
    fn get(&self, opts: GetOptions) -> ApiResult<GotRegion> {
        self.get_all(opts)
            .map(|regions| regions.try_into().expect("Regions API returns empty regions"))
    }

    fn get_all(&self, opts: GetOptions) -> ApiResult<GotRegions> {
        self.cache.get(&self.cache_key, || self.provider.get_all(opts))
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get(&self, opts: GetOptions) -> BoxFuture<'_, ApiResult<GotRegion>> {
        Box::pin(async move {
            self.async_get_all(opts)
                .await
                .map(|regions| regions.try_into().expect("Regions API returns empty regions"))
        })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get_all(&self, opts: GetOptions) -> BoxFuture<'_, ApiResult<GotRegions>> {
        Box::pin(async move {
            self.cache
                .async_get(&self.cache_key, self.provider.async_get_all(opts))
                .await
        })
    }
}

/// 七牛所有区域信息查询构建器
#[derive(Clone, Debug)]
pub struct AllRegionsProviderBuilder {
    cache_lifetime: Duration,
    shrink_interval: Duration,
    http_client: Option<HttpClient>,
    uc_endpoints: Option<Endpoints>,
    credential_provider: Box<dyn CredentialProvider>,
}

impl AllRegionsProviderBuilder {
    /// 缓存时长
    #[inline]
    pub fn cache_lifetime(mut self, cache_lifetime: Duration) -> Self {
        self.cache_lifetime = cache_lifetime;
        self
    }

    /// 清理间隔时长
    #[inline]
    pub fn shrink_interval(mut self, shrink_interval: Duration) -> Self {
        self.shrink_interval = shrink_interval;
        self
    }

    /// 设置 HTTP 客户端
    #[inline]
    pub fn http_client(mut self, http_client: HttpClient) -> Self {
        self.http_client = Some(http_client);
        self
    }

    /// 是否启用 HTTPS 协议
    ///
    /// 默认为 HTTPS 协议
    pub fn use_https(self, use_https: bool) -> Self {
        self.http_client(HttpClient::build_default().use_https(use_https).build())
    }

    /// 设置存储空间管理终端地址列表
    #[inline]
    pub fn uc_endpoints(mut self, uc_endpoints: impl Into<Endpoints>) -> Self {
        self.uc_endpoints = Some(uc_endpoints.into());
        self
    }

    /// 从文件系统加载或构建七牛所有区域查询器
    ///
    /// 可以选择是否启用自动持久化缓存功能
    pub fn load_or_create_from(self, path: impl AsRef<Path>, auto_persistent: bool) -> AllRegionsProvider {
        AllRegionsProvider {
            cache: RegionsCache::load_or_create_from(
                path.as_ref(),
                auto_persistent,
                self.cache_lifetime,
                self.shrink_interval,
            ),
            cache_key: self.new_cache_key(),
            provider: self.new_regions_provider(),
        }
    }

    /// 从默认文件系统路径加载或构建七牛所有区域查询器，并启用自动持久化缓存功能
    #[inline]
    pub fn build(self) -> AllRegionsProvider {
        self.default_load_or_create_from(true)
    }

    /// 从默认文件系统路径加载或构建七牛所有区域查询器
    ///
    /// 可以选择是否启用自动持久化缓存功能
    pub fn default_load_or_create_from(self, auto_persistent: bool) -> AllRegionsProvider {
        AllRegionsProvider {
            cache: RegionsCache::default_load_or_create_from(
                auto_persistent,
                self.cache_lifetime,
                self.shrink_interval,
            ),
            cache_key: self.new_cache_key(),
            provider: self.new_regions_provider(),
        }
    }

    /// 构建七牛所有区域查询器
    ///
    /// 不启用文件系统持久化缓存
    pub fn in_memory(self) -> AllRegionsProvider {
        AllRegionsProvider {
            cache: RegionsCache::in_memory(self.cache_lifetime, self.shrink_interval),
            cache_key: self.new_cache_key(),
            provider: self.new_regions_provider(),
        }
    }

    fn new_cache_key(&self) -> CacheKey {
        CacheKey::new_from_endpoint(
            if let Some(uc_endpoints) = self.uc_endpoints.as_ref() {
                uc_endpoints
            } else {
                Endpoints::public_uc_endpoints()
            },
            None,
        )
    }

    fn new_regions_provider(self) -> inner::AllRegionsProvider {
        let mut builder = inner::AllRegionsProvider::builder(self.credential_provider);
        if let Some(http_client) = self.http_client {
            builder = builder.http_client(http_client);
        }
        if let Some(uc_endpoints) = self.uc_endpoints {
            builder = builder.uc_endpoints(uc_endpoints);
        }
        builder.build()
    }
}

mod inner {
    use super::super::{
        super::{
            super::{ApiResult, Authorization, HttpClient, Response, ResponseError},
            Endpoints, ServiceName,
        },
        structs::ResponseBody,
        GetOptions, GotRegion, GotRegions, Region, RegionsProvider,
    };
    use qiniu_credential::CredentialProvider;
    use std::{convert::TryFrom, fmt::Debug};

    #[cfg(feature = "async")]
    use futures::future::BoxFuture;

    #[derive(Debug, Clone)]
    pub(super) struct AllRegionsProvider {
        credential_provider: Box<dyn CredentialProvider>,
        http_client: HttpClient,
        uc_endpoints: Endpoints,
    }

    impl AllRegionsProvider {
        #[inline]
        pub(super) fn builder(credential_provider: impl CredentialProvider + 'static) -> AllRegionsProviderBuilder {
            AllRegionsProviderBuilder::new(credential_provider)
        }

        fn do_sync_query(&self) -> ApiResult<GotRegions> {
            handle_response_body(
                self.http_client
                    .get(&[ServiceName::Uc], &self.uc_endpoints)
                    .path("/regions")
                    .authorization(Authorization::v2(&self.credential_provider))
                    .accept_json()
                    .call()?
                    .parse_json::<ResponseBody>()?,
            )
        }

        #[cfg(feature = "async")]
        async fn do_async_query(&self) -> ApiResult<GotRegions> {
            handle_response_body(
                self.http_client
                    .async_get(&[ServiceName::Uc], &self.uc_endpoints)
                    .path("/regions")
                    .authorization(Authorization::v2(&self.credential_provider))
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

    impl RegionsProvider for AllRegionsProvider {
        fn get(&self, opts: GetOptions) -> ApiResult<GotRegion> {
            self.get_all(opts)
                .map(|regions| regions.try_into().expect("Regions API returns empty regions"))
        }

        #[inline]
        fn get_all(&self, _opts: GetOptions) -> ApiResult<GotRegions> {
            self.do_sync_query().map(GotRegions::from)
        }

        #[cfg(feature = "async")]
        #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
        fn async_get(&self, opts: GetOptions) -> BoxFuture<'_, ApiResult<GotRegion>> {
            Box::pin(async move {
                self.async_get_all(opts)
                    .await
                    .map(|regions| regions.try_into().expect("Regions API returns empty regions"))
            })
        }

        #[inline]
        #[cfg(feature = "async")]
        #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
        fn async_get_all(&self, _opts: GetOptions) -> BoxFuture<'_, ApiResult<GotRegions>> {
            Box::pin(async move { self.do_async_query().await.map(GotRegions::from) })
        }
    }

    #[derive(Debug, Clone)]
    pub(super) struct AllRegionsProviderBuilder {
        credential_provider: Box<dyn CredentialProvider>,
        http_client: Option<HttpClient>,
        uc_endpoints: Option<Endpoints>,
    }

    impl AllRegionsProviderBuilder {
        #[inline]
        pub(super) fn new(credential_provider: impl CredentialProvider + 'static) -> Self {
            Self {
                credential_provider: Box::new(credential_provider),
                http_client: None,
                uc_endpoints: None,
            }
        }

        #[inline]
        pub(super) fn http_client(mut self, http_client: HttpClient) -> Self {
            self.http_client = Some(http_client);
            self
        }

        #[inline]
        pub(super) fn uc_endpoints(mut self, uc_endpoints: impl Into<Endpoints>) -> Self {
            self.uc_endpoints = Some(uc_endpoints.into());
            self
        }

        pub(super) fn build(self) -> AllRegionsProvider {
            AllRegionsProvider {
                credential_provider: self.credential_provider,
                http_client: self.http_client.unwrap_or_default(),
                uc_endpoints: self
                    .uc_endpoints
                    .unwrap_or_else(|| Endpoints::public_uc_endpoints().to_owned()),
            }
        }
    }

    #[cfg(all(test, feature = "isahc", feature = "async"))]
    mod tests {
        use super::{
            super::super::super::{super::HttpClient, Endpoint},
            *,
        };
        use futures::channel::oneshot::channel;
        use qiniu_credential::Credential;
        use serde_json::{json, Value as JsonValue};
        use std::{error::Error, result::Result, time::Duration};
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

        #[tokio::test]
        async fn test_get_all_regions() -> Result<(), Box<dyn Error>> {
            const ACCESS_KEY: &str = "0123456789001234567890";
            const SECRET_KEY: &str = "secret01234567890";
            let routes =
                path!("regions")
                    .and(warp::header::value("Authorization"))
                    .map(move |authorization: HeaderValue| {
                        assert!(authorization
                            .to_str()
                            .unwrap()
                            .starts_with("Qiniu 0123456789001234567890:"));
                        let mut response = Response::new(get_response_json_body().to_string().into_bytes().into());
                        response
                            .headers_mut()
                            .insert("X-Reqid", HeaderValue::from_static("FAKE_REQ_ID"));
                        response
                    });

            starts_with_server!(addr, routes, {
                let provider = AllRegionsProvider::builder(Credential::new(ACCESS_KEY, SECRET_KEY))
                    .http_client(HttpClient::build_isahc()?.use_https(false).build())
                    .uc_endpoints(vec![Endpoint::from(addr)])
                    .build();

                let regions = provider.async_get_all(Default::default()).await?;
                assert_eq!(regions.lifetime(), Some(Duration::from_secs(5)));
                assert_eq!(regions.len(), 5);
                assert_eq!(
                    regions.iter().map(|region| region.region_id()).collect::<Vec<_>>(),
                    &["z0", "z1", "z2", "as0", "na0"]
                )
            });
            Ok(())
        }

        fn get_response_json_body() -> JsonValue {
            json!({
                "regions": [
                   {
                     "id": "z0",
                     "ttl": 5,
                     "description": "East China",
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
                     "id": "z1",
                     "ttl": 5,
                     "description": "North China",
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
                   },
                   {
                     "id": "z2",
                     "ttl": 5,
                     "description": "South China",
                     "io": {
                       "domains": [
                         "iovip-z2.qbox.me"
                       ]
                     },
                     "up": {
                       "domains": [
                         "upload-z2.qiniup.com",
                         "up-z2.qiniup.com"
                       ],
                       "old": [
                         "upload-z2.qbox.me",
                         "up-z2.qbox.me"
                       ]
                     },
                     "uc": {
                       "domains": [
                         "uc.qbox.me"
                       ]
                     },
                     "rs": {
                       "domains": [
                         "rs-z2.qbox.me"
                       ]
                     },
                     "rsf": {
                       "domains": [
                         "rsf-z2.qbox.me"
                       ]
                     },
                     "api": {
                       "domains": [
                         "api.qiniu.com"
                       ]
                     },
                     "s3": {
                       "domains": [
                         "s3-cn-south-1.qiniucs.com"
                       ],
                       "region_alias": "cn-south-1"
                     }
                   },
                   {
                     "id": "as0",
                     "ttl": 5,
                     "description": "Southeast Asia",
                     "io": {
                       "domains": [
                         "iovip-as0.qbox.me"
                       ]
                     },
                     "up": {
                       "domains": [
                         "upload-as0.qiniup.com",
                         "up-as0.qiniup.com"
                       ],
                       "old": [
                         "upload-as0.qbox.me",
                         "up-as0.qbox.me"
                       ]
                     },
                     "uc": {
                       "domains": [
                         "uc.qbox.me"
                       ]
                     },
                     "rs": {
                       "domains": [
                         "rs-na0.qbox.me"
                       ]
                     },
                     "rsf": {
                       "domains": [
                         "rsf-na0.qbox.me"
                       ]
                     },
                     "api": {
                       "domains": [
                         "api.qiniu.com"
                       ]
                     },
                     "s3": {
                       "domains": [
                         "s3-ap-southeast-1.qiniucs.com"
                       ],
                       "region_alias": "ap-southeast-1"
                     }
                   },
                   {
                     "id": "na0",
                     "ttl": 5,
                     "description": "North America",
                     "io": {
                       "domains": [
                         "iovip-na0.qbox.me"
                       ]
                     },
                     "up": {
                       "domains": [
                         "upload-na0.qiniup.com",
                         "up-na0.qiniup.com"
                       ],
                       "old": [
                         "upload-na0.qbox.me",
                         "up-na0.qbox.me"
                       ]
                     },
                     "uc": {
                       "domains": [
                         "uc.qbox.me"
                       ]
                     },
                     "rs": {
                       "domains": [
                         "rs-na0.qbox.me"
                       ]
                     },
                     "rsf": {
                       "domains": [
                         "rsf-na0.qbox.me"
                       ]
                     },
                     "api": {
                       "domains": [
                         "api.qiniu.com"
                       ]
                     },
                     "s3": {
                       "domains": [
                         "s3-us-north-1.qiniucs.com"
                       ],
                       "region_alias": "us-north-1"
                     }
                   }
                ]
            })
        }
    }
}
