use super::{
    batch_operations::BatchOperations,
    callbacks::Callbacks,
    list::{ListIter, ListVersion},
    operation::{
        CopyObject, CopyObjectBuilder, DeleteObject, DeleteObjectBuilder, Entry,
        ModifyObjectLifeCycle, ModifyObjectLifeCycleBuilder, ModifyObjectMetadata,
        ModifyObjectMetadataBuilder, ModifyObjectStatus, ModifyObjectStatusBuilder, MoveObject,
        MoveObjectBuilder, ObjectType, SetObjectType, SetObjectTypeBuilder, StatObject,
        StatObjectBuilder, UnfreezeObject, UnfreezeObjectBuilder,
    },
    ObjectsManager,
};
use mime::Mime;
use once_cell::sync::OnceCell;
use qiniu_apis::{
    http::ResponseParts,
    http_client::{
        BucketName, BucketRegionsProvider, CallbackResult, RegionProvider, RequestBuilderParts,
        ResponseError,
    },
};
use std::{io::Result as IOResult, mem::take, sync::Arc};

#[cfg(feature = "async")]
use {super::list::ListStream, async_once_cell::OnceCell as AsyncOnceCell};

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

    #[inline]
    pub fn objects_manager(&self) -> &ObjectsManager {
        &self.0.objects_manager
    }

    #[inline]
    pub fn list(&self) -> ListBuilder<'_> {
        ListBuilder::new(self)
    }

    #[inline]
    pub fn stat_object<'a>(&'a self, object_name: &'a str) -> StatObjectBuilder<'a> {
        StatObject::builder(Entry::new(self, object_name))
    }

    #[inline]
    pub fn copy_object_to<'a>(
        &'a self,
        from_object_name: &'a str,
        to_bucket_name: &'a str,
        to_object_name: &'a str,
    ) -> CopyObjectBuilder<'a> {
        CopyObject::builder(
            Entry::new(self, from_object_name),
            to_bucket_name,
            to_object_name,
        )
    }

    #[inline]
    pub fn move_object_to<'a>(
        &'a self,
        from_object_name: &'a str,
        to_bucket_name: &'a str,
        to_object_name: &'a str,
    ) -> MoveObjectBuilder<'a> {
        MoveObject::builder(
            Entry::new(self, from_object_name),
            to_bucket_name,
            to_object_name,
        )
    }

    #[inline]
    pub fn delete_object<'a>(&'a self, object_name: &'a str) -> DeleteObjectBuilder<'a> {
        DeleteObject::builder(Entry::new(self, object_name))
    }

    #[inline]
    pub fn unfreeze_object<'a>(
        &'a self,
        object_name: &'a str,
        freeze_after_days: usize,
    ) -> UnfreezeObjectBuilder<'a> {
        UnfreezeObject::builder(Entry::new(self, object_name), freeze_after_days)
    }

    #[inline]
    pub fn set_object_type<'a>(
        &'a self,
        object_name: &'a str,
        object_type: ObjectType,
    ) -> SetObjectTypeBuilder<'a> {
        SetObjectType::builder(Entry::new(self, object_name), object_type)
    }

    #[inline]
    pub fn modify_object_status<'a>(
        &'a self,
        object_name: &'a str,
        disable: bool,
    ) -> ModifyObjectStatusBuilder<'a> {
        ModifyObjectStatus::builder(Entry::new(self, object_name), disable)
    }

    #[inline]
    pub fn modify_object_metadata<'a>(
        &'a self,
        object_name: &'a str,
        mime_type: Mime,
    ) -> ModifyObjectMetadataBuilder<'a> {
        ModifyObjectMetadata::builder(Entry::new(self, object_name), mime_type)
    }

    #[inline]
    pub fn modify_object_life_cycle<'a>(
        &'a self,
        object_name: &'a str,
    ) -> ModifyObjectLifeCycleBuilder<'a> {
        ModifyObjectLifeCycle::builder(Entry::new(self, object_name))
    }

    #[inline]
    pub fn batch_ops(&self) -> BatchOperations<'_> {
        BatchOperations::new(self)
    }

    pub(super) fn region_provider(&self) -> IOResult<&dyn RegionProvider> {
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
    pub(super) async fn async_region_provider(&self) -> IOResult<&dyn RegionProvider> {
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

#[must_use]
pub struct ListBuilder<'a> {
    bucket: &'a Bucket,
    limit: Option<usize>,
    prefix: Option<&'a str>,
    marker: Option<&'a str>,
    version: ListVersion,
    need_parts: bool,
    callbacks: Callbacks<'a>,
}

impl<'a> ListBuilder<'a> {
    fn new(bucket: &'a Bucket) -> Self {
        Self {
            bucket,
            limit: Default::default(),
            prefix: Default::default(),
            marker: Default::default(),
            version: Default::default(),
            need_parts: Default::default(),
            callbacks: Default::default(),
        }
    }

    #[inline]
    pub fn limit(&mut self, limit: usize) -> &mut Self {
        self.limit = Some(limit);
        self
    }

    #[inline]
    pub fn prefix(&mut self, prefix: &'a str) -> &mut Self {
        self.prefix = Some(prefix);
        self
    }

    #[inline]
    pub fn marker(&mut self, marker: &'a str) -> &mut Self {
        self.marker = Some(marker);
        self
    }

    #[inline]
    pub fn version(&mut self, version: ListVersion) -> &mut Self {
        self.version = version;
        self
    }

    #[inline]
    pub fn need_parts(&mut self) -> &mut Self {
        self.need_parts = true;
        self
    }

    #[inline]
    pub fn before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'a,
    ) -> &mut Self {
        self.callbacks.before_request_callback = Some(Box::new(callback));
        self
    }

    #[inline]
    pub fn after_response_ok_callback(
        &mut self,
        callback: impl FnMut(&mut ResponseParts) -> CallbackResult + Send + Sync + 'a,
    ) -> &mut Self {
        self.callbacks.after_response_ok_callback = Some(Box::new(callback));
        self
    }

    #[inline]
    pub fn after_response_error_callback(
        &mut self,
        callback: impl FnMut(&ResponseError) -> CallbackResult + Send + Sync + 'a,
    ) -> &mut Self {
        self.callbacks.after_response_error_callback = Some(Box::new(callback));
        self
    }

    #[inline]
    pub fn iter(&mut self) -> ListIter<'a> {
        let owned = self.take_self();
        ListIter::new(
            owned.bucket,
            owned.limit,
            owned.prefix,
            owned.marker,
            owned.need_parts,
            owned.version,
            owned.callbacks,
        )
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub fn stream(&mut self) -> ListStream<'a> {
        let owned = self.take_self();
        ListStream::new(
            owned.bucket,
            owned.limit,
            owned.prefix,
            owned.marker,
            owned.need_parts,
            owned.version,
            owned.callbacks,
        )
    }

    fn take_self(&mut self) -> Self {
        Self {
            bucket: self.bucket,
            limit: take(&mut self.limit),
            prefix: take(&mut self.prefix),
            marker: take(&mut self.marker),
            need_parts: take(&mut self.need_parts),
            version: take(&mut self.version),
            callbacks: take(&mut self.callbacks),
        }
    }
}
