use super::{
    super::{
        ConcurrencyProvider, DataPartitionProvider, DataSource, FixedDataPartitionProvider, MultiPartsUploader,
        ObjectParams,
    },
    MultiPartsUploaderScheduler,
};
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::num::NonZeroU64;

#[cfg(feature = "async")]
use {super::AsyncDataSource, futures::future::BoxFuture};

/// 串行分片上传调度器
///
/// 不启动任何线程，仅在本地串行上传分片。
///
/// ### 用串行分片上传调度器上传文件
///
/// ###### 阻塞代码示例
///
/// ```
/// use qiniu_upload_manager::{
///     apis::credential::Credential, prelude::*, FileSystemResumableRecorder, MultiPartsV2Uploader,
///     ObjectParams, SerialMultiPartsUploaderScheduler, UploadManager, UploadTokenSigner,
/// };
/// use std::time::Duration;
/// use sha1::Sha1;
///
/// # fn example() -> anyhow::Result<()> {
/// let bucket_name = "test-bucket";
/// let object_name = "test-object";
/// let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
///     Credential::new("abcdefghklmnopq", "1234567890"),
///     bucket_name,
///     Duration::from_secs(3600),
/// ))
/// .build();
/// let params = ObjectParams::builder().object_name(object_name).file_name(object_name).build();
/// let mut scheduler = SerialMultiPartsUploaderScheduler::new(MultiPartsV2Uploader::new(
///     upload_manager,
///     FileSystemResumableRecorder::<Sha1>::default(),
/// ));
/// scheduler.upload_path("/home/qiniu/test.png", params)?;
/// # Ok(())
/// # }
/// ```
///
/// ###### 异步代码示例
///
/// ```
/// use qiniu_upload_manager::{
///     apis::credential::Credential, prelude::*, FileSystemResumableRecorder, MultiPartsV2Uploader,
///     ObjectParams, SerialMultiPartsUploaderScheduler, UploadManager, UploadTokenSigner,
/// };
/// use std::time::Duration;
/// use sha1::Sha1;
///
/// # async fn example() -> anyhow::Result<()> {
/// let bucket_name = "test-bucket";
/// let object_name = "test-object";
/// let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
///     Credential::new("abcdefghklmnopq", "1234567890"),
///     bucket_name,
///     Duration::from_secs(3600),
/// ))
/// .build();
/// let params = ObjectParams::builder().object_name(object_name).file_name(object_name).build();
/// let mut scheduler = SerialMultiPartsUploaderScheduler::new(MultiPartsV2Uploader::new(
///     upload_manager,
///     FileSystemResumableRecorder::<Sha1>::default(),
/// ));
/// scheduler.async_upload_path("/home/qiniu/test.png", params).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SerialMultiPartsUploaderScheduler<M: MultiPartsUploader> {
    data_partition_provider: Box<dyn DataPartitionProvider>,
    multi_parts_uploader: M,
}

impl<M: MultiPartsUploader> SerialMultiPartsUploaderScheduler<M> {
    /// 创建串行分片上传调度器
    #[inline]
    pub fn new(multi_parts_uploader: M) -> Self {
        Self {
            data_partition_provider: Box::new(FixedDataPartitionProvider::new_with_non_zero_part_size(
                #[allow(unsafe_code)]
                unsafe {
                    NonZeroU64::new_unchecked(1 << 22)
                },
            )),
            multi_parts_uploader,
        }
    }

    /// 获取分片大小提供者
    #[inline]
    pub fn data_partition_provider(&self) -> &dyn DataPartitionProvider {
        &self.data_partition_provider
    }
}

impl<M: MultiPartsUploader> MultiPartsUploaderScheduler<M::HashAlgorithm> for SerialMultiPartsUploaderScheduler<M> {
    fn set_concurrency_provider(&mut self, _concurrency_provider: Box<dyn ConcurrencyProvider>) {}

    fn set_data_partition_provider(&mut self, data_partition_provider: Box<dyn DataPartitionProvider>) {
        self.data_partition_provider = data_partition_provider;
    }

    fn upload(&self, source: Box<dyn DataSource<M::HashAlgorithm>>, params: ObjectParams) -> ApiResult<Value> {
        return _upload(self, source, params);

        fn _upload<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            source: Box<dyn DataSource<M::HashAlgorithm>>,
            params: ObjectParams,
        ) -> ApiResult<Value> {
            let initialized = scheduler.multi_parts_uploader.initialize_parts(source, params)?;
            let mut parts = Vec::with_capacity(4);
            while let Some(uploaded_part) = scheduler
                .multi_parts_uploader
                .upload_part(&initialized, &scheduler.data_partition_provider)?
            {
                parts.push(uploaded_part);
            }
            scheduler.multi_parts_uploader.complete_parts(&initialized, &parts)
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload(
        &self,
        source: Box<dyn AsyncDataSource<M::HashAlgorithm>>,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>> {
        return Box::pin(async move { _upload(self, source, params).await });

        async fn _upload<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            source: Box<dyn AsyncDataSource<M::HashAlgorithm>>,
            params: ObjectParams,
        ) -> ApiResult<Value> {
            let initialized = scheduler
                .multi_parts_uploader
                .async_initialize_parts(source, params)
                .await?;
            let mut parts = Vec::with_capacity(4);
            while let Some(uploaded_part) = scheduler
                .multi_parts_uploader
                .async_upload_part(&initialized, &scheduler.data_partition_provider)
                .await?
            {
                parts.push(uploaded_part);
            }
            scheduler
                .multi_parts_uploader
                .async_complete_parts(&initialized, &parts)
                .await
        }
    }
}
