use super::Bucket;
use assert_impl::assert_impl;
use qiniu_apis::{
    credential::CredentialProvider,
    http_client::{
        BucketName, BucketRegionsQueryer, BucketRegionsQueryerBuilder, Endpoints, HttpClient, RegionsProvider,
    },
    Client as QiniuApiClient,
};
use std::sync::Arc;

/// 七牛对象管理器
#[derive(Debug, Clone)]
pub struct ObjectsManager(Arc<ObjectsManagerInner>);

#[derive(Debug)]
struct ObjectsManagerInner {
    client: QiniuApiClient,
    credential: Arc<dyn CredentialProvider>,
    queryer: BucketRegionsQueryer,
}

impl ObjectsManager {
    /// 创建七牛对象管理构建器
    ///
    /// 必须传入认证信息提供者
    #[inline]
    pub fn builder(credential: impl CredentialProvider + 'static) -> ObjectsManagerBuilder {
        ObjectsManagerBuilder::new(credential)
    }

    /// 创建七牛对象管理器
    ///
    /// 必须传入认证信息提供者
    #[inline]
    pub fn new(credential: impl CredentialProvider + 'static) -> Self {
        Self::builder(credential).build()
    }

    /// 获取七牛 API 调用客户端
    #[inline]
    pub fn client(&self) -> &QiniuApiClient {
        &self.0.client
    }

    /// 获取七牛认证信息提供者
    #[inline]
    pub fn credential(&self) -> &dyn CredentialProvider {
        &self.0.credential
    }

    /// 获取七牛存储空间相关区域查询器
    #[inline]
    pub fn queryer(&self) -> &BucketRegionsQueryer {
        &self.0.queryer
    }

    /// 获取七牛存储空间管理器
    #[inline]
    pub fn bucket(&self, name: impl Into<BucketName>) -> Bucket {
        self._bucket_with_region(name.into(), None)
    }

    /// 获取七牛存储空间管理器
    ///
    /// 可以提供区域信息提供者
    #[inline]
    pub fn bucket_with_region(
        &self,
        name: impl Into<BucketName>,
        region_provider: impl RegionsProvider + 'static,
    ) -> Bucket {
        self._bucket_with_region(name.into(), Some(Box::new(region_provider)))
    }

    fn _bucket_with_region(&self, name: BucketName, region_provider: Option<Box<dyn RegionsProvider>>) -> Bucket {
        Bucket::new(name, self.to_owned(), region_provider)
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// 七牛对象管理构建器
#[derive(Debug, Clone)]
pub struct ObjectsManagerBuilder {
    api_client: Option<QiniuApiClient>,
    http_client: Option<HttpClient>,
    credential: Arc<dyn CredentialProvider>,
    queryer_builder: Option<BucketRegionsQueryerBuilder>,
    queryer: Option<BucketRegionsQueryer>,
}

impl ObjectsManagerBuilder {
    /// 创建七牛对象管理构建器
    #[inline]
    pub fn new(credential: impl CredentialProvider + 'static) -> Self {
        Self {
            credential: Arc::new(credential),
            api_client: Default::default(),
            http_client: Default::default(),
            queryer_builder: Default::default(),
            queryer: Default::default(),
        }
    }

    /// 设置七牛 API 调用客户端
    #[inline]
    pub fn api_client(&mut self, api_client: QiniuApiClient) -> &mut Self {
        self.api_client = Some(api_client);
        self
    }

    /// 设置 HTTP 客户端
    pub fn http_client(&mut self, http_client: HttpClient) -> &mut Self {
        self.http_client = Some(http_client.to_owned());
        self.with_queryer_builder(|queryer_builder| {
            queryer_builder.http_client(http_client);
        })
    }

    /// 是否启用 HTTPS 协议
    ///
    /// 默认为 HTTPS 协议
    pub fn use_https(&mut self, use_https: bool) -> &mut Self {
        self.http_client(HttpClient::build_default().use_https(use_https).build())
            .with_queryer_builder(|queryer_builder| {
                queryer_builder.use_https(use_https);
            })
    }

    /// 设置存储空间相关区域查询器
    #[inline]
    pub fn queryer(&mut self, queryer: BucketRegionsQueryer) -> &mut Self {
        self.queryer = Some(queryer);
        self
    }

    /// 设置存储空间管理终端地址
    pub fn uc_endpoints(&mut self, endpoints: impl Into<Endpoints>) -> &mut Self {
        self.with_queryer_builder(|queryer_builder| {
            queryer_builder.uc_endpoints(endpoints);
        })
    }

    fn with_queryer_builder(&mut self, f: impl FnOnce(&mut BucketRegionsQueryerBuilder)) -> &mut Self {
        if let Some(queryer_builder) = self.queryer_builder.as_mut() {
            f(queryer_builder);
        } else {
            let mut queryer_builder = BucketRegionsQueryer::builder();
            f(&mut queryer_builder);
            self.queryer_builder = Some(queryer_builder);
        }
        self
    }

    /// 构建七牛对象管理器
    pub fn build(&mut self) -> ObjectsManager {
        let api_client = self.api_client.take();
        let http_client = self.http_client.take();
        let queryer = self.queryer.take();
        let mut queryer_builder = self.queryer_builder.take();

        ObjectsManager(Arc::new(ObjectsManagerInner {
            client: api_client
                .or_else(|| http_client.map(QiniuApiClient::new))
                .unwrap_or_default(),
            credential: self.credential.to_owned(),
            queryer: queryer
                .or_else(|| queryer_builder.as_mut().map(|builder| builder.build()))
                .unwrap_or_default(),
        }))
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}
