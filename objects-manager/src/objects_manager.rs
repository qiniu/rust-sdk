use super::Bucket;
use qiniu_apis::{
    credential::CredentialProvider,
    http_client::{
        BucketName, BucketRegionsQueryer, BucketRegionsQueryerBuilder, Endpoints, HttpClient,
        RegionProvider,
    },
    Client as QiniuApiClient,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ObjectsManager(Arc<ObjectsManagerInner>);

#[derive(Debug)]
struct ObjectsManagerInner {
    client: QiniuApiClient,
    credential: Arc<dyn CredentialProvider>,
    queryer: BucketRegionsQueryer,
}

impl ObjectsManager {
    #[inline]
    pub fn builder(credential: impl CredentialProvider + 'static) -> ObjectsManagerBuilder {
        ObjectsManagerBuilder::new(credential)
    }

    #[inline]
    pub fn new(credential: impl CredentialProvider + 'static) -> Self {
        Self::builder(credential).build()
    }

    #[inline]
    pub fn client(&self) -> &QiniuApiClient {
        &self.0.client
    }

    #[inline]
    pub fn credential(&self) -> &dyn CredentialProvider {
        &self.0.credential
    }

    #[inline]
    pub fn queryer(&self) -> &BucketRegionsQueryer {
        &self.0.queryer
    }

    #[inline]
    pub fn bucket(&self, name: BucketName) -> Bucket {
        self._bucket_with_region(name, None)
    }

    #[inline]
    pub fn bucket_with_region(
        &self,
        name: BucketName,
        region_provider: impl RegionProvider + 'static,
    ) -> Bucket {
        self._bucket_with_region(name, Some(Box::new(region_provider)))
    }

    fn _bucket_with_region(
        &self,
        name: BucketName,
        region_provider: Option<Box<dyn RegionProvider>>,
    ) -> Bucket {
        Bucket::new(name, self.to_owned(), region_provider)
    }
}

#[derive(Debug, Clone)]
pub struct ObjectsManagerBuilder {
    api_client: Option<QiniuApiClient>,
    http_client: Option<HttpClient>,
    credential: Arc<dyn CredentialProvider>,
    queryer_builder: Option<BucketRegionsQueryerBuilder>,
    queryer: Option<BucketRegionsQueryer>,
}

impl ObjectsManagerBuilder {
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

    #[inline]
    pub fn api_client(mut self, api_client: QiniuApiClient) -> Self {
        self.api_client = Some(api_client);
        self
    }

    pub fn http_client(mut self, http_client: HttpClient) -> Self {
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
    pub fn queryer(mut self, queryer: BucketRegionsQueryer) -> Self {
        self.queryer = Some(queryer);
        self
    }

    pub fn uc_endpoints(mut self, endpoints: impl Into<Endpoints>) -> Self {
        if let Some(queryer_builder) = self.queryer_builder.as_mut() {
            queryer_builder.uc_endpoints(endpoints);
        } else {
            let mut queryer_builder = BucketRegionsQueryer::builder();
            queryer_builder.uc_endpoints(endpoints);
            self.queryer_builder = Some(queryer_builder);
        }
        self
    }

    pub fn build(mut self) -> ObjectsManager {
        ObjectsManager(Arc::new(ObjectsManagerInner {
            client: self
                .api_client
                .or_else(|| self.http_client.map(QiniuApiClient::new))
                .unwrap_or_default(),
            credential: self.credential,
            queryer: self
                .queryer
                .or_else(|| self.queryer_builder.as_mut().map(|builder| builder.build()))
                .unwrap_or_default(),
        }))
    }
}
