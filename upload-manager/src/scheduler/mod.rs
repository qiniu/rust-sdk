use super::{ConcurrencyProvider, DataPartitionProvider, DataSource, MultiPartsUploader, ObjectParams};
use crate::{data_source::FileDataSource, data_source::UnseekableDataSource};
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::{fmt::Debug, io::Read, path::Path};

#[cfg(feature = "async")]
use {
    crate::data_source::{AsyncDataSource, AsyncFileDataSource, AsyncUnseekableDataSource},
    futures::{future::BoxFuture, AsyncRead},
};

/// 分片上传调度器接口
///
/// 负责分片上传的调度，包括初始化分片信息、上传分片、完成分片上传。
pub trait MultiPartsUploaderScheduler: Send + Sync + Debug {
    /// 分片上传器
    type MultiPartsUploader: MultiPartsUploader;

    /// 创建分片上传调度器
    fn new(multi_parts_uploader: Self::MultiPartsUploader) -> Self;

    /// 设置并发数提供者
    fn set_concurrency_provider(&mut self, concurrency_provider: impl ConcurrencyProvider + 'static) -> &mut Self;

    /// 设置分片大小提供者
    fn set_data_partition_provider(
        &mut self,
        data_partition_provider: impl DataPartitionProvider + 'static,
    ) -> &mut Self;

    /// 上传数据源
    ///
    /// 该方法的异步版本为 [`Self::async_upload`]。
    fn upload<D: DataSource<<Self::MultiPartsUploader as MultiPartsUploader>::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> ApiResult<Value>;

    /// 异步上传数据源
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload<D: AsyncDataSource<<Self::MultiPartsUploader as MultiPartsUploader>::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>>;
}

/// 分片上传调度器扩展接口
pub trait MultiPartsUploaderSchedulerExt: MultiPartsUploaderScheduler {
    /// 上传指定路径的文件
    fn upload_path(&self, path: impl AsRef<Path>, params: ObjectParams) -> ApiResult<Value> {
        self.upload(FileDataSource::new(path.as_ref()), params)
    }

    /// 上传输入流的数据
    fn upload_reader<R: Read + Debug + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> ApiResult<Value> {
        self.upload(UnseekableDataSource::new(reader), params)
    }

    /// 异步上传指定路径的文件
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_path<'a>(
        &'a self,
        path: impl AsRef<Path> + Send + Sync + 'a,
        params: ObjectParams,
    ) -> BoxFuture<'a, ApiResult<Value>> {
        Box::pin(async move { self.async_upload(AsyncFileDataSource::new(path.as_ref()), params).await })
    }

    /// 异步上传输入流的数据
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_reader<R: AsyncRead + Unpin + Debug + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>> {
        Box::pin(async move { self.async_upload(AsyncUnseekableDataSource::new(reader), params).await })
    }
}

impl<T: MultiPartsUploaderScheduler> MultiPartsUploaderSchedulerExt for T {}

mod serial_multi_parts_uploader_scheduler;
pub use serial_multi_parts_uploader_scheduler::SerialMultiPartsUploaderScheduler;
mod concurrent_multi_parts_uploader_scheduler;
pub use concurrent_multi_parts_uploader_scheduler::ConcurrentMultiPartsUploaderScheduler;
