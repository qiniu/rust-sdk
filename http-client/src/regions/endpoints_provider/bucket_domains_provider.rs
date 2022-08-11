use super::{
    super::{
        super::{Authorization, HttpClient},
        cache_key::CacheKey,
    },
    endpoints_cache::EndpointsCache,
    ApiResult, Endpoint, Endpoints, EndpointsProvider, GetOptions as EndpointsGetOptions, ServiceName,
};
use qiniu_credential::{Credential, CredentialProvider};
use qiniu_upload_token::BucketName;
use std::{borrow::Cow, mem::take, path::Path, sync::Arc, time::Duration};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

const DEFAULT_SHRINK_INTERVAL: Duration = Duration::from_secs(3600);
const DEFAULT_CACHE_LIFETIME: Duration = Duration::from_secs(3600);

/// 存储空间绑定域名查询器
///
/// 查询该存储空间绑定的域名。
///
/// ### 存储空间绑定域名查询器使用示例
///
/// ##### 阻塞代码示例
///
/// ```
/// use qiniu_credential::Credential;
/// use qiniu_http_client::{Authorization, BucketDomainsQueryer, HttpClient};
///
/// # fn example() -> anyhow::Result<()> {
/// let credential = Credential::new("abcdefghklmnopq", "1234567890");
/// let response = HttpClient::default()
///     .get(
///         &[],
///         BucketDomainsQueryer::new().query(credential.to_owned(), "test-bucket"),
///     )
///     .path("/test-key")
///     .use_https(false)
///     .authorization(Authorization::download(credential))
///     .call()?;
/// # Ok(())
/// # }
/// ```
///
/// ##### 异步代码示例
///
/// ```
/// # async fn example() -> anyhow::Result<()> {
/// use qiniu_credential::Credential;
/// use qiniu_http_client::{Authorization, BucketDomainsQueryer, HttpClient};
///
/// let credential = Credential::new("abcdefghklmnopq", "1234567890");
/// let response = HttpClient::default()
///     .async_get(
///         &[],
///         BucketDomainsQueryer::new().query(credential.to_owned(), "test-bucket"),
///     )
///     .path("/test-key")
///     .use_https(false)
///     .authorization(Authorization::download(credential))
///     .call()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct BucketDomainsQueryer {
    http_client: HttpClient,
    uc_endpoints: Endpoints,
    cache: EndpointsCache,
}

/// 存储空间绑定域名查询构建器
#[derive(Debug, Clone)]
pub struct BucketDomainsQueryerBuilder {
    http_client: Option<HttpClient>,
    uc_endpoints: Option<Endpoints>,
    cache_lifetime: Duration,
    shrink_interval: Duration,
}

impl BucketDomainsQueryer {
    /// 创建存储空间绑定域名查询构建器
    #[inline]
    pub fn builder() -> BucketDomainsQueryerBuilder {
        BucketDomainsQueryerBuilder::new()
    }

    /// 创建存储空间绑定域名查询器
    #[inline]
    pub fn new() -> BucketDomainsQueryer {
        BucketDomainsQueryerBuilder::new().build()
    }

    /// 查询存储空间相关域名
    pub fn query(
        &self,
        credential: impl CredentialProvider + 'static,
        bucket_name: impl Into<BucketName>,
    ) -> BucketDomainsProvider {
        BucketDomainsProvider {
            queryer: self.to_owned(),
            credential: Arc::new(credential),
            bucket_name: bucket_name.into(),
        }
    }
}

impl Default for BucketDomainsQueryer {
    #[inline]
    fn default() -> Self {
        Self::builder().default_load_or_create_from(true)
    }
}

impl Default for BucketDomainsQueryerBuilder {
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

impl BucketDomainsQueryerBuilder {
    /// 创建存储空间绑定域名查询构建器
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

    /// 从文件系统加载或构建存储空间绑定域名查询器
    ///
    /// 可以选择是否启用自动持久化缓存功能
    pub fn load_or_create_from(&mut self, path: impl AsRef<Path>, auto_persistent: bool) -> BucketDomainsQueryer {
        let owned = take(self);
        BucketDomainsQueryer {
            cache: EndpointsCache::load_or_create_from(
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

    /// 从默认文件系统路径加载或构建存储空间绑定域名查询器，并启用自动持久化缓存功能
    #[inline]
    pub fn build(&mut self) -> BucketDomainsQueryer {
        self.default_load_or_create_from(true)
    }

    /// 从默认文件系统路径加载或构建存储空间绑定域名查询器
    ///
    /// 可以选择是否启用自动持久化缓存功能
    pub fn default_load_or_create_from(&mut self, auto_persistent: bool) -> BucketDomainsQueryer {
        let owned = take(self);
        BucketDomainsQueryer {
            cache: EndpointsCache::default_load_or_create_from(
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

    /// 构建存储空间绑定域名查询器
    ///
    /// 不启用文件系统持久化缓存
    pub fn in_memory(&mut self) -> BucketDomainsQueryer {
        let owned = take(self);
        BucketDomainsQueryer {
            cache: EndpointsCache::in_memory(owned.cache_lifetime, owned.shrink_interval),
            http_client: owned.http_client.unwrap_or_default(),
            uc_endpoints: owned
                .uc_endpoints
                .unwrap_or_else(|| Endpoints::public_uc_endpoints().to_owned()),
        }
    }
}

/// 存储空间绑定域名获取器
#[derive(Debug, Clone)]
pub struct BucketDomainsProvider {
    queryer: BucketDomainsQueryer,
    credential: Arc<dyn CredentialProvider>,
    bucket_name: BucketName,
}

impl EndpointsProvider for BucketDomainsProvider {
    fn get_endpoints<'e>(&'e self, _options: EndpointsGetOptions<'_>) -> ApiResult<Cow<'e, Endpoints>> {
        let credential = self.credential.get(Default::default())?;
        self.queryer
            .cache
            .get(&self.make_cache_key(&credential), || self.do_sync_query())
            .map(Cow::Owned)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get_endpoints<'a>(
        &'a self,
        _options: EndpointsGetOptions<'_>,
    ) -> BoxFuture<'a, ApiResult<Cow<'a, Endpoints>>> {
        Box::pin(async move {
            let credential = self.credential.async_get(Default::default()).await?;
            self.queryer
                .cache
                .async_get(&self.make_cache_key(&credential), self.do_async_query())
                .await
                .map(Cow::Owned)
        })
    }
}

impl BucketDomainsProvider {
    fn make_cache_key(&self, credential: &Credential) -> CacheKey {
        CacheKey::new_from_endpoint_and_ak_and_bucket(
            &self.queryer.uc_endpoints,
            self.bucket_name.to_owned(),
            credential.access_key().to_owned(),
        )
    }

    fn do_sync_query(&self) -> ApiResult<Endpoints> {
        let endpoints: Endpoints = self
            .queryer
            .http_client
            .get(&[ServiceName::Uc], &self.queryer.uc_endpoints)
            .path("/v2/domains")
            .authorization(Authorization::v2(&self.credential))
            .append_query_pair("tbl", self.bucket_name.as_str())
            .accept_json()
            .call()?
            .parse_json::<Vec<String>>()?
            .into_body()
            .into_iter()
            .map(Endpoint::from)
            .collect();
        Ok(endpoints)
    }

    #[cfg(feature = "async")]
    async fn do_async_query(&self) -> ApiResult<Endpoints> {
        let endpoints: Endpoints = self
            .queryer
            .http_client
            .async_get(&[ServiceName::Uc], &self.queryer.uc_endpoints)
            .path("/v2/domains")
            .authorization(Authorization::v2(&self.credential))
            .append_query_pair("tbl", self.bucket_name.as_str())
            .accept_json()
            .call()
            .await?
            .parse_json::<Vec<String>>()
            .await?
            .into_body()
            .into_iter()
            .map(Endpoint::from)
            .collect();
        Ok(endpoints)
    }
}
