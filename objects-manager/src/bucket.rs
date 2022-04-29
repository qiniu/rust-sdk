use super::{
    batch_operations::BatchOperations,
    callbacks::Callbacks,
    list::{ListIter, ListVersion},
    mime::Mime,
    operation::{
        CopyObject, CopyObjectBuilder, DeleteObject, DeleteObjectBuilder, Entry, ModifyObjectLifeCycle,
        ModifyObjectLifeCycleBuilder, ModifyObjectMetadata, ModifyObjectMetadataBuilder, ModifyObjectStatus,
        ModifyObjectStatusBuilder, MoveObject, MoveObjectBuilder, SetObjectType, SetObjectTypeBuilder, StatObject,
        StatObjectBuilder, UnfreezeObject, UnfreezeObjectBuilder,
    },
    ObjectsManager,
};
use anyhow::Result as AnyResult;
use assert_impl::assert_impl;
use once_cell::sync::OnceCell;
use qiniu_apis::{
    http::ResponseParts,
    http_client::{BucketName, BucketRegionsProvider, RegionsProvider, RequestBuilderParts, ResponseError},
    upload_token::FileType,
};
use std::{borrow::Cow, io::Result as IOResult, mem::take, sync::Arc};

#[cfg(feature = "async")]
use {super::list::ListStream, async_once_cell::OnceCell as AsyncOnceCell};

/// 七牛存储空间管理器
#[derive(Debug, Clone)]
pub struct Bucket(Arc<BucketInner>);

#[derive(Debug)]
struct BucketInner {
    name: BucketName,
    objects_manager: ObjectsManager,
    region_provider: Option<Box<dyn RegionsProvider>>,
    bucket_regions_provider: OnceCell<BucketRegionsProvider>,

    #[cfg(feature = "async")]
    async_bucket_regions_provider: AsyncOnceCell<BucketRegionsProvider>,
}

impl Bucket {
    pub(super) fn new(
        name: BucketName,
        objects_manager: ObjectsManager,
        region_provider: Option<Box<dyn RegionsProvider>>,
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

    /// 获取存储空间名称
    #[inline]
    pub fn name(&self) -> &BucketName {
        &self.0.name
    }

    /// 获取对象管理器
    #[inline]
    pub fn objects_manager(&self) -> &ObjectsManager {
        &self.0.objects_manager
    }

    /// 创建列举操作构建器
    ///
    /// ##### 阻塞代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    /// use futures::stream::TryStreamExt;
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    /// let mut iter = bucket.list().iter();
    /// while let Some(object) = iter.next() {
    ///     let object = object?;
    ///     println!("fsize: {:?}", object.get_size_as_u64());
    ///     println!("hash: {:?}", object.get_hash_as_str());
    ///     println!("mime_type: {:?}", object.get_mime_type_as_str());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ##### 异步代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    /// use futures::stream::TryStreamExt;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    /// let mut stream = bucket.list().stream();
    /// while let Some(object) = stream.try_next().await? {
    ///     println!("fsize: {:?}", object.get_size_as_u64());
    ///     println!("hash: {:?}", object.get_hash_as_str());
    ///     println!("mime_type: {:?}", object.get_mime_type_as_str());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn list(&self) -> ListBuilder<'_> {
        ListBuilder::new(self)
    }

    /// 创建对象元信息获取操作构建器
    ///
    /// ##### 阻塞代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// let response = bucket.stat_object("test-key").call()?;
    /// let object = response.into_body();
    /// println!("fsize: {}", object.get_size_as_u64());
    /// println!("hash: {}", object.get_hash_as_str());
    /// println!("mime_type: {}", object.get_mime_type_as_str());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ##### 异步代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// let response = bucket.stat_object("test-key").async_call().await?;
    /// let object = response.into_body();
    /// println!("fsize: {}", object.get_size_as_u64());
    /// println!("hash: {}", object.get_hash_as_str());
    /// println!("mime_type: {}", object.get_mime_type_as_str());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn stat_object<'a>(&'a self, object_name: &'a str) -> StatObjectBuilder<'a> {
        StatObject::builder(Entry::new(self, object_name))
    }

    /// 创建对象复制操作构建器
    ///
    /// ##### 阻塞代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.copy_object_to("test-key", "test-bucket-2", "test-key").call()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ##### 异步代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.copy_object_to("test-key", "test-bucket-2", "test-key").async_call().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn copy_object_to<'a>(
        &'a self,
        from_object_name: &'a str,
        to_bucket_name: &'a str,
        to_object_name: &'a str,
    ) -> CopyObjectBuilder<'a> {
        CopyObject::builder(Entry::new(self, from_object_name), to_bucket_name, to_object_name)
    }

    /// 创建对象移动操作构建器
    ///
    /// ##### 阻塞代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.move_object_to("test-key", "test-bucket-2", "test-key").call()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ##### 异步代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.move_object_to("test-key", "test-bucket-2", "test-key").async_call().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn move_object_to<'a>(
        &'a self,
        from_object_name: &'a str,
        to_bucket_name: &'a str,
        to_object_name: &'a str,
    ) -> MoveObjectBuilder<'a> {
        MoveObject::builder(Entry::new(self, from_object_name), to_bucket_name, to_object_name)
    }

    /// 创建对象删除操作构建器
    ///
    /// ##### 阻塞代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.delete_object("test-key").call()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ##### 异步代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.delete_object("test-key").async_call().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn delete_object<'a>(&'a self, object_name: &'a str) -> DeleteObjectBuilder<'a> {
        DeleteObject::builder(Entry::new(self, object_name))
    }

    /// 创建对象解冻操作构建器
    ///
    /// ##### 阻塞代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.restore_archived_object("test-key", 1).call()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ##### 异步代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.restore_archived_object("test-key", 1).async_call().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn restore_archived_object<'a>(
        &'a self,
        object_name: &'a str,
        freeze_after_days: usize,
    ) -> UnfreezeObjectBuilder<'a> {
        UnfreezeObject::builder(Entry::new(self, object_name), freeze_after_days)
    }

    /// 创建对象类型设置操作构建器
    ///
    /// ##### 阻塞代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::{credential::Credential, upload_token::FileType}, ObjectsManager};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.set_object_type("test-key", FileType::Archive).call()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ##### 异步代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::{credential::Credential, upload_token::FileType}, ObjectsManager};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.set_object_type("test-key", FileType::Archive).async_call().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn set_object_type<'a>(&'a self, object_name: &'a str, object_type: FileType) -> SetObjectTypeBuilder<'a> {
        SetObjectType::builder(Entry::new(self, object_name), object_type)
    }

    /// 创建对象状态设置操作构建器
    ///
    /// ##### 阻塞代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.modify_object_status("test-key", true).call()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ##### 异步代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.modify_object_status("test-key", true).async_call().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn modify_object_status<'a>(&'a self, object_name: &'a str, disable: bool) -> ModifyObjectStatusBuilder<'a> {
        ModifyObjectStatus::builder(Entry::new(self, object_name), disable)
    }

    /// 创建对象元信息设置操作构建器
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager, mime::APPLICATION_JSON};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.modify_object_metadata("test-key", APPLICATION_JSON).async_call().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn modify_object_metadata<'a>(
        &'a self,
        object_name: &'a str,
        mime_type: Mime,
    ) -> ModifyObjectMetadataBuilder<'a> {
        ModifyObjectMetadata::builder(Entry::new(self, object_name), mime_type)
    }

    /// 创建对象生命周期设置操作构建器
    ///
    /// ##### 阻塞代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{AfterDays, apis::credential::Credential, ObjectsManager};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.modify_object_life_cycle("test-key").delete_after_days(AfterDays::new(5)).call()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ##### 异步代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{AfterDays, apis::credential::Credential, ObjectsManager};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    ///
    /// bucket.modify_object_life_cycle("test-key").delete_after_days(AfterDays::new(5)).async_call().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn modify_object_life_cycle<'a>(&'a self, object_name: &'a str) -> ModifyObjectLifeCycleBuilder<'a> {
        ModifyObjectLifeCycle::builder(Entry::new(self, object_name))
    }

    /// 创建批量操作
    ///
    /// ##### 阻塞代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager, OperationProvider};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    /// let mut ops = bucket.batch_ops();
    /// ops.add_operation(bucket.stat_object("test-file-1"));
    /// ops.add_operation(bucket.stat_object("test-file-2"));
    /// ops.add_operation(bucket.stat_object("test-file-3"));
    /// ops.add_operation(bucket.stat_object("test-file-4"));
    /// ops.add_operation(bucket.stat_object("test-file-5"));
    /// let mut iter = ops.call();
    /// while let Some(object) = iter.next() {
    ///     let object = object?;
    ///     println!("fsize: {:?}", object.get_size_as_u64());
    ///     println!("hash: {:?}", object.get_hash_as_str());
    ///     println!("mime_type: {:?}", object.get_mime_type_as_str());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ##### 异步代码示例
    ///
    /// ```
    /// use qiniu_objects_manager::{apis::credential::Credential, ObjectsManager, OperationProvider};
    /// use futures::stream::TryStreamExt;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let credential = Credential::new("abcdefghklmnopq", "1234567890");
    /// let object_manager = ObjectsManager::new(credential);
    /// let bucket = object_manager.bucket("test-bucket");
    /// let mut ops = bucket.batch_ops();
    /// ops.add_operation(bucket.stat_object("test-file-1"));
    /// ops.add_operation(bucket.stat_object("test-file-2"));
    /// ops.add_operation(bucket.stat_object("test-file-3"));
    /// ops.add_operation(bucket.stat_object("test-file-4"));
    /// ops.add_operation(bucket.stat_object("test-file-5"));
    /// let mut stream = ops.async_call();
    /// while let Some(object) = stream.try_next().await? {
    ///     println!("fsize: {:?}", object.get_size_as_u64());
    ///     println!("hash: {:?}", object.get_hash_as_str());
    ///     println!("mime_type: {:?}", object.get_mime_type_as_str());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn batch_ops(&self) -> BatchOperations<'_> {
        BatchOperations::new(self)
    }

    pub(super) fn region_provider(&self) -> IOResult<&dyn RegionsProvider> {
        self.0
            .region_provider
            .as_ref()
            .map(|r| Ok(r as &dyn RegionsProvider))
            .unwrap_or_else(|| {
                self.0
                    .bucket_regions_provider
                    .get_or_try_init(|| {
                        Ok(self.0.objects_manager.queryer().query(
                            self.0
                                .objects_manager
                                .credential()
                                .get(Default::default())?
                                .access_key()
                                .to_owned(),
                            self.name().to_owned(),
                        ))
                    })
                    .map(|r| r as &dyn RegionsProvider)
            })
    }

    #[cfg(feature = "async")]
    pub(super) async fn async_region_provider(&self) -> IOResult<&dyn RegionsProvider> {
        return if let Some(region_provider) = self.0.region_provider.as_ref() {
            Ok(region_provider)
        } else {
            self.0
                .async_bucket_regions_provider
                .get_or_try_init(create_region_provider(&self.0.objects_manager, self.name()))
                .await
                .map(|r| r as &dyn RegionsProvider)
        };

        async fn create_region_provider(
            objects_manager: &ObjectsManager,
            bucket_name: &BucketName,
        ) -> IOResult<BucketRegionsProvider> {
            Ok(objects_manager.queryer().query(
                objects_manager
                    .credential()
                    .async_get(Default::default())
                    .await?
                    .access_key()
                    .to_owned(),
                bucket_name.to_owned(),
            ))
        }
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// 列举操作构建器
///
/// 可以通过 [`Bucket::list`] 方法获取该构建器。
#[must_use]
#[derive(Debug)]
pub struct ListBuilder<'a> {
    bucket: &'a Bucket,
    limit: Option<usize>,
    prefix: Option<Cow<'a, str>>,
    marker: Option<Cow<'a, str>>,
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
    /// 设置列举限制
    pub fn limit(&mut self, limit: usize) -> &mut Self {
        self.limit = Some(limit);
        self
    }

    #[inline]
    /// 设置对象名称前缀匹配字符串
    pub fn prefix(&mut self, prefix: impl Into<Cow<'a, str>>) -> &mut Self {
        self.prefix = Some(prefix.into());
        self
    }

    #[inline]
    /// 设置上一次列举返回的位置标记
    pub fn marker(&mut self, marker: impl Into<Cow<'a, str>>) -> &mut Self {
        self.marker = Some(marker.into());
        self
    }

    #[inline]
    /// 设置列举 API 版本
    pub fn version(&mut self, version: ListVersion) -> &mut Self {
        self.version = version;
        self
    }

    /// 设置是否需要返回分片信息
    #[inline]
    pub fn need_parts(&mut self) -> &mut Self {
        self.need_parts = true;
        self
    }

    /// 设置请求前回调函数
    #[inline]
    pub fn before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.callbacks.insert_before_request_callback(callback);
        self
    }

    /// 设置响应成功回调函数
    #[inline]
    pub fn after_response_ok_callback(
        &mut self,
        callback: impl FnMut(&mut ResponseParts) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.callbacks.insert_after_response_ok_callback(callback);
        self
    }

    /// 设置响应失败回调函数
    #[inline]
    pub fn after_response_error_callback(
        &mut self,
        callback: impl FnMut(&ResponseError) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.callbacks.insert_after_response_error_callback(callback);
        self
    }

    /// 创建对象列举迭代器
    ///
    /// 对象列举迭代器采用阻塞 API 列举对象信息
    ///
    /// 该方法的的异步版本为 [`Self::stream`]。
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

    /// 创建对象列举流
    ///
    /// 对象列举流采用异步 API 列举对象信息
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

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}
