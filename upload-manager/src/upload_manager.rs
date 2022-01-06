use super::{
    ConcurrencyProvider, DataPartitionProvider, FixedConcurrencyProvider,
    FixedDataPartitionProvider, FixedThresholdResumablePolicy, FormUploader,
    ResumablePolicyProvider, SinglePartUploader,
};
use qiniu_apis::{
    http_client::{BucketRegionsQueryer, BucketRegionsQueryerBuilder, Endpoints, HttpClient},
    Client as QiniuApiClient,
};
use qiniu_upload_token::UploadTokenProvider;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct UploadManager(Arc<UploadManagerInner>);

#[derive(Debug)]
struct UploadManagerInner {
    upload_token_provider: Box<dyn UploadTokenProvider>,
    data_partition_provider: Box<dyn DataPartitionProvider>,
    concurrency_provider: Box<dyn ConcurrencyProvider>,
    resumable_policy_provider: Box<dyn ResumablePolicyProvider>,
    client: QiniuApiClient,
    queryer: BucketRegionsQueryer,
}

impl UploadManager {
    #[inline]
    pub fn builder(upload_token: impl UploadTokenProvider + 'static) -> UploadManagerBuilder {
        UploadManagerBuilder::new(upload_token)
    }

    #[inline]
    pub fn new(upload_token: impl UploadTokenProvider + 'static) -> Self {
        Self::builder(upload_token).build()
    }

    #[inline]
    pub fn upload_token(&self) -> &dyn UploadTokenProvider {
        &self.0.upload_token_provider
    }

    #[inline]
    pub fn data_partition(&self) -> &dyn DataPartitionProvider {
        &self.0.data_partition_provider
    }

    #[inline]
    pub fn concurrency(&self) -> &dyn ConcurrencyProvider {
        &self.0.concurrency_provider
    }

    #[inline]
    pub fn resumable_policy(&self) -> &dyn ResumablePolicyProvider {
        &self.0.resumable_policy_provider
    }

    #[inline]
    pub fn client(&self) -> &QiniuApiClient {
        &self.0.client
    }

    #[inline]
    pub fn queryer(&self) -> &BucketRegionsQueryer {
        &self.0.queryer
    }

    #[inline]
    pub fn single_part_uploader(&self) -> impl SinglePartUploader {
        self.form_uploader()
    }

    #[inline]
    pub fn form_uploader(&self) -> FormUploader {
        FormUploader::new(self.to_owned())
    }
}

#[derive(Debug)]
pub struct UploadManagerBuilder {
    api_client: Option<QiniuApiClient>,
    http_client: Option<HttpClient>,
    queryer_builder: Option<BucketRegionsQueryerBuilder>,
    queryer: Option<BucketRegionsQueryer>,
    upload_token_provider: Box<dyn UploadTokenProvider>,
    data_partition_provider: Option<Box<dyn DataPartitionProvider>>,
    concurrency_provider: Option<Box<dyn ConcurrencyProvider>>,
    resumable_policy_provider: Option<Box<dyn ResumablePolicyProvider>>,
}

impl UploadManagerBuilder {
    #[inline]
    pub fn new(upload_token: impl UploadTokenProvider + 'static) -> Self {
        Self {
            upload_token_provider: Box::new(upload_token),
            api_client: Default::default(),
            http_client: Default::default(),
            queryer_builder: Default::default(),
            queryer: Default::default(),
            data_partition_provider: Default::default(),
            concurrency_provider: Default::default(),
            resumable_policy_provider: Default::default(),
        }
    }

    #[inline]
    pub fn api_client(&mut self, api_client: QiniuApiClient) -> &mut Self {
        self.api_client = Some(api_client);
        self
    }

    pub fn http_client(&mut self, http_client: HttpClient) -> &mut Self {
        self.http_client = Some(http_client.to_owned());
        if let Some(queryer_builder) = self.queryer_builder.as_mut() {
            queryer_builder.http_client(http_client);
        } else {
            let mut queryer_builder = BucketRegionsQueryer::builder();
            queryer_builder.http_client(http_client);
            self.queryer_builder = Some(queryer_builder);
        }
        self
    }

    #[inline]
    pub fn queryer(&mut self, queryer: BucketRegionsQueryer) -> &mut Self {
        self.queryer = Some(queryer);
        self
    }

    pub fn uc_endpoints(&mut self, endpoints: impl Into<Endpoints>) -> &mut Self {
        if let Some(queryer_builder) = self.queryer_builder.as_mut() {
            queryer_builder.uc_endpoints(endpoints);
        } else {
            let mut queryer_builder = BucketRegionsQueryer::builder();
            queryer_builder.uc_endpoints(endpoints);
            self.queryer_builder = Some(queryer_builder);
        }
        self
    }

    #[inline]
    pub fn data_partition(
        &mut self,
        data_partition: impl DataPartitionProvider + 'static,
    ) -> &mut Self {
        self.data_partition_provider = Some(Box::new(data_partition));
        self
    }

    #[inline]
    pub fn concurrency(&mut self, concurrency: impl ConcurrencyProvider + 'static) -> &mut Self {
        self.concurrency_provider = Some(Box::new(concurrency));
        self
    }

    #[inline]
    pub fn resumable_policy(
        &mut self,
        resumable_policy: impl ResumablePolicyProvider + 'static,
    ) -> &mut Self {
        self.resumable_policy_provider = Some(Box::new(resumable_policy));
        self
    }

    pub fn build(&mut self) -> UploadManager {
        let upload_token_provider = self.upload_token_provider.to_owned();
        let api_client = self.api_client.take();
        let http_client = self.http_client.take();
        let queryer = self.queryer.take();
        let mut queryer_builder = self.queryer_builder.take();
        UploadManager(Arc::new(UploadManagerInner {
            upload_token_provider,
            data_partition_provider: self
                .data_partition_provider
                .take()
                .unwrap_or_else(UploadManager::default_data_partition),
            concurrency_provider: self
                .concurrency_provider
                .take()
                .unwrap_or_else(UploadManager::default_concurrency),
            resumable_policy_provider: self
                .resumable_policy_provider
                .take()
                .unwrap_or_else(UploadManager::default_resumable_policy),
            client: api_client
                .or_else(|| http_client.map(QiniuApiClient::new))
                .unwrap_or_default(),
            queryer: queryer
                .or_else(|| queryer_builder.as_mut().map(|builder| builder.build()))
                .unwrap_or_default(),
        }))
    }
}

impl UploadManager {
    #[inline]
    pub fn default_data_partition() -> Box<dyn DataPartitionProvider> {
        Box::new(FixedDataPartitionProvider::new(1 << 22).unwrap())
    }

    #[inline]
    pub fn default_concurrency() -> Box<dyn ConcurrencyProvider> {
        Box::new(FixedConcurrencyProvider::new(2).unwrap())
    }

    #[inline]
    pub fn default_resumable_policy() -> Box<dyn ResumablePolicyProvider> {
        Box::new(FixedThresholdResumablePolicy::new(1 << 22))
    }
}
