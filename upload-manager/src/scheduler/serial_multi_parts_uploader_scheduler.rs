use super::{
    super::{ConcurrencyProvider, FixedDataPartitionProvider, ResumableRecorder},
    DataPartitionProvider, DataSource, MultiPartsUploader, MultiPartsUploaderScheduler, ObjectParams,
};
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::num::NonZeroU64;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug)]
pub struct SerialMultiPartsUploaderScheduler<M> {
    data_partition_provider: Box<dyn DataPartitionProvider>,
    multi_parts_uploader: M,
}

impl<M: MultiPartsUploader> MultiPartsUploaderScheduler for SerialMultiPartsUploaderScheduler<M> {
    type MultiPartsUploader = M;

    fn new(multi_parts_uploader: Self::MultiPartsUploader) -> Self {
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

    fn set_concurrency_provider(&mut self, _concurrency_provider: impl ConcurrencyProvider + 'static) {}

    fn set_data_partition_provider(&mut self, data_partition_provider: impl DataPartitionProvider + 'static) {
        self.data_partition_provider = Box::new(data_partition_provider);
    }

    fn upload<D: DataSource<<<Self::MultiPartsUploader as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> ApiResult<Value>{
        return _upload(self, source, params);

        fn _upload<
            M: MultiPartsUploader,
            D: DataSource<<M::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static,
        >(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            source: D,
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
            scheduler.multi_parts_uploader.complete_parts(initialized, parts)
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload<D: DataSource<<<Self::MultiPartsUploader as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>>{
        return Box::pin(async move { _upload(self, source, params).await });

        async fn _upload<
            M: MultiPartsUploader,
            D: DataSource<<M::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static,
        >(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            source: D,
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
                .async_complete_parts(initialized, parts)
                .await
        }
    }
}
