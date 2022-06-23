use super::{
    upload_token::UploadTokenSigner, AutoUploader, AutoUploaderBuilder, FormUploader, MultiPartsUploader,
    MultiPartsV1Uploader, MultiPartsV2Uploader, ResumableRecorder, SinglePartUploader,
};
use assert_impl::assert_impl;
use digest::Digest;
use qiniu_apis::{
    http_client::{BucketRegionsQueryer, BucketRegionsQueryerBuilder, Endpoints, HttpClient},
    Client as QiniuApiClient,
};
use std::sync::Arc;

/// 上传管理器
#[derive(Debug, Clone)]
pub struct UploadManager(Arc<UploadManagerInner>);

#[derive(Debug)]
struct UploadManagerInner {
    upload_token_signer: UploadTokenSigner,
    client: QiniuApiClient,
    queryer: BucketRegionsQueryer,
}

impl UploadManager {
    /// 创建上传管理构建器
    #[inline]
    pub fn builder(upload_token: impl Into<UploadTokenSigner>) -> UploadManagerBuilder {
        UploadManagerBuilder::new(upload_token)
    }

    /// 创建上传管理器
    #[inline]
    pub fn new(upload_token: impl Into<UploadTokenSigner>) -> Self {
        Self::builder(upload_token).build()
    }

    /// 获取上传凭证签发器
    #[inline]
    pub fn upload_token(&self) -> &UploadTokenSigner {
        &self.0.upload_token_signer
    }

    /// 获取七牛 API 调用客户端
    #[inline]
    pub fn client(&self) -> &QiniuApiClient {
        &self.0.client
    }

    /// 获取存储空间相关区域查询器
    #[inline]
    pub fn queryer(&self) -> &BucketRegionsQueryer {
        &self.0.queryer
    }

    /// 创建默认的单请求上传器
    #[inline]
    pub fn single_part_uploader(&self) -> impl SinglePartUploader {
        self.form_uploader()
    }

    /// 创建表单上传器
    #[inline]
    pub fn form_uploader(&self) -> FormUploader {
        FormUploader::new(self.to_owned())
    }

    /// 创建默认的分片上传器
    #[inline]
    pub fn multi_parts_uploader<H: Digest + Send + 'static, R: ResumableRecorder<HashAlgorithm = H> + 'static>(
        &self,
        resumable_recorder: R,
    ) -> impl MultiPartsUploader<HashAlgorithm = H> {
        self.multi_parts_v2_uploader(resumable_recorder)
    }

    /// 创建分片上传器 V1
    #[inline]
    pub fn multi_parts_v1_uploader<H: Digest + Send + 'static, R: ResumableRecorder<HashAlgorithm = H> + 'static>(
        &self,
        resumable_recorder: R,
    ) -> MultiPartsV1Uploader<H> {
        MultiPartsV1Uploader::<H>::new(self.to_owned(), resumable_recorder)
    }

    /// 创建分片上传器 V2
    #[inline]
    pub fn multi_parts_v2_uploader<H: Digest + Send + 'static, R: ResumableRecorder<HashAlgorithm = H> + 'static>(
        &self,
        resumable_recorder: R,
    ) -> MultiPartsV2Uploader<H> {
        MultiPartsV2Uploader::<H>::new(self.to_owned(), resumable_recorder)
    }

    /// 创建自动上传器
    #[inline]
    pub fn auto_uploader<H: Digest + Send + 'static>(&self) -> AutoUploader<H> {
        AutoUploader::<H>::new(self.to_owned())
    }

    /// 创建自动上传构建器
    #[inline]
    pub fn auto_uploader_builder<H: Digest + Send + 'static>(&self) -> AutoUploaderBuilder<H> {
        AutoUploader::<H>::builder(self.to_owned())
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// 上传管理构建器
#[derive(Debug)]
pub struct UploadManagerBuilder {
    api_client: Option<QiniuApiClient>,
    http_client: Option<HttpClient>,
    queryer_builder: Option<BucketRegionsQueryerBuilder>,
    queryer: Option<BucketRegionsQueryer>,
    upload_token_signer: UploadTokenSigner,
}

impl UploadManagerBuilder {
    /// 创建上传管理构建器
    #[inline]
    pub fn new(upload_token_signer: impl Into<UploadTokenSigner>) -> Self {
        Self {
            upload_token_signer: upload_token_signer.into(),
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

    /// 构建上传管理器
    pub fn build(&mut self) -> UploadManager {
        let upload_token_provider = self.upload_token_signer.to_owned();
        let api_client = self.api_client.take();
        let http_client = self.http_client.take();
        let queryer = self.queryer.take();
        let mut queryer_builder = self.queryer_builder.take();
        UploadManager(Arc::new(UploadManagerInner {
            upload_token_signer: upload_token_provider,
            client: api_client
                .or_else(|| http_client.map(QiniuApiClient::new))
                .unwrap_or_default(),
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
