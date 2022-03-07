use super::{
    ConcurrencyProvider, DataPartitionProvider, DataSource, MultiPartsUploader, ObjectParams,
    ResumableRecorder,
};
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::fmt::Debug;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

pub trait MultiPartsUploaderScheduler: Send + Sync + Debug {
    type MultiPartsUploader: MultiPartsUploader;

    fn new(multi_parts_uploader: Self::MultiPartsUploader) -> Self;

    fn set_concurrency_provider(
        &mut self,
        concurrency_provider: impl ConcurrencyProvider + 'static,
    );
    fn set_data_partition_provider(
        &mut self,
        data_partition_provider: impl DataPartitionProvider + 'static,
    );

    fn upload<
        D: DataSource<<<Self::MultiPartsUploader as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static,
    >(
        &self,
        source: D,
        params: ObjectParams,
    ) -> ApiResult<Value>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload<
        D: DataSource<<<Self::MultiPartsUploader as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static,
    >(
        &self,
        source: D,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>>;
}

mod serial_multi_parts_uploader_scheduler;
pub use serial_multi_parts_uploader_scheduler::SerialMultiPartsUploaderScheduler;
