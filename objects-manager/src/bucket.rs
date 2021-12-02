use super::ObjectsManager;
use once_cell::sync::OnceCell;
use qiniu_apis::http_client::{BucketName, BucketRegionsProvider, RegionProvider};
use std::{io::Result as IOResult, sync::Arc};

#[cfg(feature = "async")]
use async_once_cell::OnceCell as AsyncOnceCell;

#[derive(Debug, Clone)]
pub struct Bucket(Arc<BucketInner>);

#[derive(Debug)]
struct BucketInner {
    name: BucketName,
    objects_manager: ObjectsManager,
    region_provider: Option<Box<dyn RegionProvider>>,
    bucket_regions_provider: OnceCell<BucketRegionsProvider>,

    #[cfg(feature = "async")]
    async_bucket_regions_provider: AsyncOnceCell<BucketRegionsProvider>,
}

impl Bucket {
    pub(super) fn new(
        name: BucketName,
        objects_manager: ObjectsManager,
        region_provider: Option<Box<dyn RegionProvider>>,
    ) -> Self {
        Self(Arc::new(BucketInner {
            name,
            objects_manager,
            region_provider,
            bucket_regions_provider: Default::default(),

            #[cfg(feature = "async")]
            async_bucket_regions_provider: AsyncOnceCell::new(),
        }))
    }

    #[inline]
    pub fn name(&self) -> &BucketName {
        &self.0.name
    }

    fn region_provider(&self) -> IOResult<&dyn RegionProvider> {
        self.0
            .region_provider
            .as_ref()
            .map(|r| Ok(r as &dyn RegionProvider))
            .unwrap_or_else(|| {
                self.0
                    .bucket_regions_provider
                    .get_or_try_init(|| {
                        Ok(self.0.objects_manager.queryer().query(
                            self.0
                                .objects_manager
                                .credential()
                                .get(&Default::default())?
                                .access_key()
                                .to_owned(),
                            self.name().to_owned(),
                        ))
                    })
                    .map(|r| r as &dyn RegionProvider)
            })
    }

    #[cfg(feature = "async")]
    async fn async_region_provider(&self) -> IOResult<&dyn RegionProvider> {
        return if let Some(region_provider) = self.0.region_provider.as_ref() {
            Ok(region_provider)
        } else {
            self.0
                .async_bucket_regions_provider
                .get_or_try_init(create_region_provider(&self.0.objects_manager, self.name()))
                .await
                .map(|r| r as &dyn RegionProvider)
        };

        async fn create_region_provider(
            objects_manager: &ObjectsManager,
            bucket_name: &BucketName,
        ) -> IOResult<BucketRegionsProvider> {
            Ok(objects_manager.queryer().query(
                objects_manager
                    .credential()
                    .async_get(&Default::default())
                    .await?
                    .access_key()
                    .to_owned(),
                bucket_name.to_owned(),
            ))
        }
    }
}
