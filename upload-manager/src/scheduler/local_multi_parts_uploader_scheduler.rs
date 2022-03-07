use qiniu_apis::http_client::ApiResult;
use serde_json::Value;

use super::{
    super::{ConcurrencyProvider, FixedDataPartitionProvider, ResumableRecorder, UploadManager},
    DataPartitionProvider, DataSource, MultiPartsUploader, MultiPartsUploaderScheduler,
    ObjectParams,
};
use std::num::NonZeroU64;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

pub struct LocalMultiPartsUploaderScheduler<M> {
    data_partition_provider: Box<dyn DataPartitionProvider>,
    multi_parts_uploader: M,
}

impl<M: MultiPartsUploader> MultiPartsUploaderScheduler for LocalMultiPartsUploaderScheduler<M> {
    type MultiPartsUploader = M;

    fn new(
        upload_manager: UploadManager,
        resumable_recorder: <Self::MultiPartsUploader as MultiPartsUploader>::ResumableRecorder,
    ) -> Self {
        Self {
            data_partition_provider: Box::new(
                FixedDataPartitionProvider::new_with_non_zero_part_size(
                    #[allow(unsafe_code)]
                    unsafe {
                        NonZeroU64::new_unchecked(1 << 22)
                    },
                ),
            ),
            multi_parts_uploader: M::new(upload_manager, resumable_recorder),
        }
    }

    fn set_concurrency_provider(
        &mut self,
        _concurrency_provider: impl ConcurrencyProvider + 'static,
    ) {
    }

    fn set_data_partition_provider(
        &mut self,
        data_partition_provider: impl DataPartitionProvider + 'static,
    ) {
        self.data_partition_provider = Box::new(data_partition_provider);
    }

    fn upload<D: DataSource<<<Self::MultiPartsUploader as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> ApiResult<Value>{
        let initialized = self.multi_parts_uploader.initialize_parts(source, params)?;
        let mut parts = Vec::new();
        while let Some(uploaded_part) = self
            .multi_parts_uploader
            .upload_part(&initialized, &self.data_partition_provider)?
        {
            parts.push(uploaded_part);
        }
        self.multi_parts_uploader.complete_parts(initialized, parts)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload<D: DataSource<<<Self::MultiPartsUploader as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static>(
        &self,
        _source: D,
        _params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>>{
        unreachable!()
    }
}
