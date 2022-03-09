use super::{
    super::{
        Concurrency, ConcurrencyProvider, ConcurrencyProviderFeedback, FixedConcurrencyProvider,
        FixedDataPartitionProvider, ResumableRecorder, UploadedPart,
    },
    DataPartitionProvider, DataSource, MultiPartsUploader, MultiPartsUploaderScheduler,
    ObjectParams,
};
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::{
    num::{NonZeroU64, NonZeroUsize},
    time::Instant,
};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug)]
pub struct SerialMultiPartsUploaderScheduler<M> {
    data_partition_provider: Box<dyn DataPartitionProvider>,
    concurrency_provider: Box<dyn ConcurrencyProvider>,
    multi_parts_uploader: M,
}

#[allow(unsafe_code)]
const ONE: Concurrency =
    Concurrency::new_with_non_zero_usize(unsafe { NonZeroUsize::new_unchecked(1) });

impl<M: MultiPartsUploader> MultiPartsUploaderScheduler for SerialMultiPartsUploaderScheduler<M> {
    type MultiPartsUploader = M;

    fn new(multi_parts_uploader: Self::MultiPartsUploader) -> Self {
        Self {
            data_partition_provider: Box::new(
                FixedDataPartitionProvider::new_with_non_zero_part_size(
                    #[allow(unsafe_code)]
                    unsafe {
                        NonZeroU64::new_unchecked(1 << 22)
                    },
                ),
            ),
            concurrency_provider: Box::new(
                FixedConcurrencyProvider::new_with_non_zero_concurrency(
                    #[allow(unsafe_code)]
                    unsafe {
                        NonZeroUsize::new_unchecked(1)
                    },
                ),
            ),
            multi_parts_uploader,
        }
    }

    fn set_concurrency_provider(
        &mut self,
        concurrency_provider: impl ConcurrencyProvider + 'static,
    ) {
        self.concurrency_provider = Box::new(concurrency_provider);
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
        let mut uploaded_size = 0u64;
        let begin_at = Instant::now();
        let result = _upload(self, source, params, &mut uploaded_size);
        let elapsed = begin_at.elapsed();
        if let Some(uploaded_size) = NonZeroU64::new(uploaded_size) {
            self.concurrency_provider
                .feedback(ConcurrencyProviderFeedback::new(
                    ONE,
                    uploaded_size,
                    elapsed,
                    result.as_ref().err(),
                ))
        }
        return result;

        fn _upload<
            M: MultiPartsUploader,
            D: DataSource<<M::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static,
        >(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            source: D,
            params: ObjectParams,
            uploaded_size: &mut u64,
        ) -> ApiResult<Value> {
            let initialized = scheduler
                .multi_parts_uploader
                .initialize_parts(source, params)?;
            let mut parts = Vec::with_capacity(4);
            while let Some(uploaded_part) = scheduler
                .multi_parts_uploader
                .upload_part(&initialized, &scheduler.data_partition_provider)?
            {
                if !uploaded_part.resumed() {
                    *uploaded_size += uploaded_part.size().get();
                }
                parts.push(uploaded_part);
            }
            scheduler
                .multi_parts_uploader
                .complete_parts(initialized, parts)
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload<D: DataSource<<<Self::MultiPartsUploader as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>>{
        return Box::pin(async move {
            let mut uploaded_size = 0u64;
            let begin_at = Instant::now();
            let result = _upload(self, source, params, &mut uploaded_size).await;
            let elapsed = begin_at.elapsed();
            if let Some(uploaded_size) = NonZeroU64::new(uploaded_size) {
                self.concurrency_provider
                    .feedback(ConcurrencyProviderFeedback::new(
                        ONE,
                        uploaded_size,
                        elapsed,
                        result.as_ref().err(),
                    ))
            }
            result
        });

        async fn _upload<
            M: MultiPartsUploader,
            D: DataSource<<M::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static,
        >(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            source: D,
            params: ObjectParams,
            uploaded_size: &mut u64,
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
                if !uploaded_part.resumed() {
                    *uploaded_size += uploaded_part.size().get();
                }
                parts.push(uploaded_part);
            }
            scheduler
                .multi_parts_uploader
                .async_complete_parts(initialized, parts)
                .await
        }
    }
}
