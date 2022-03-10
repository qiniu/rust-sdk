use super::{
    ConcurrencyProvider, DataPartitionProvider, DataSource, MultiPartsUploader, ObjectParams, ResumableRecorder,
};
use crate::{FileDataSource, UnseekableDataSource};
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::{fmt::Debug, io::Read, path::Path};

#[cfg(feature = "async")]
use {
    crate::AsyncUnseekableDataSource,
    futures::{future::BoxFuture, AsyncRead},
};

pub trait MultiPartsUploaderScheduler: Send + Sync + Debug {
    type MultiPartsUploader: MultiPartsUploader;

    fn new(multi_parts_uploader: Self::MultiPartsUploader) -> Self;

    fn set_concurrency_provider(&mut self, concurrency_provider: impl ConcurrencyProvider + 'static);
    fn set_data_partition_provider(&mut self, data_partition_provider: impl DataPartitionProvider + 'static);

    fn upload<D: DataSource<<<Self::MultiPartsUploader as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static>(&self, source: D, params: ObjectParams) -> ApiResult<Value>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload<D: DataSource<<<Self::MultiPartsUploader as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static>(&self, source: D, params: ObjectParams) -> BoxFuture<ApiResult<Value>>;
}

pub trait MultiPartsUploaderSchedulerExt: MultiPartsUploaderScheduler
where
    <<Self::MultiPartsUploader as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm: Send,
{
    fn upload_path(&self, path: &Path, params: ObjectParams) -> ApiResult<Value> {
        self.upload(FileDataSource::new(path), params)
    }

    fn upload_reader<R: Read + Debug + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> ApiResult<Value> {
        self.upload(UnseekableDataSource::new(reader), params)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_path<'a>(&'a self, path: &'a Path, params: ObjectParams) -> BoxFuture<'a, ApiResult<Value>> {
        Box::pin(async move { self.async_upload(FileDataSource::new(path), params).await })
    }

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

impl<T: MultiPartsUploaderScheduler> MultiPartsUploaderSchedulerExt for T where
    <<T::MultiPartsUploader as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm: Send
{
}

mod serial_multi_parts_uploader_scheduler;
pub use serial_multi_parts_uploader_scheduler::SerialMultiPartsUploaderScheduler;
mod concurrent_multi_parts_uploader_scheduler;
pub use concurrent_multi_parts_uploader_scheduler::ConcurrentMultiPartsUploaderScheduler;
