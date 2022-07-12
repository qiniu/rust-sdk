use super::{
    super::{
        Concurrency, ConcurrencyProvider, ConcurrencyProviderFeedback, DataPartitionProvider, DataSource,
        FixedConcurrencyProvider, FixedDataPartitionProvider, MultiPartsUploader, ObjectParams, UploadedPart,
    },
    MultiPartsUploaderScheduler,
};
use qiniu_apis::http_client::{ApiResult, ResponseError, ResponseErrorKind};
use rayon::ThreadPoolBuilder;
use serde_json::Value;
use std::{
    num::{NonZeroU64, NonZeroUsize},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex,
    },
    time::Instant,
};

#[cfg(feature = "async")]
use {
    super::AsyncDataSource,
    async_std::task::spawn,
    futures::future::{join_all, BoxFuture},
    std::sync::Arc,
};

/// 并行分片上传调度器
///
/// 在阻塞模式下创建线程池负责上传分片，在异步模式下使用 async-std 的线程池负责上传分片。
///
/// ### 用并行分片上传调度器上传文件
///
/// ##### 阻塞代码示例
///
/// ```
/// use qiniu_upload_manager::{
///     apis::credential::Credential, prelude::*, ConcurrentMultiPartsUploaderScheduler,
///     FileSystemResumableRecorder, MultiPartsV2Uploader, ObjectParams, UploadManager,
///     UploadTokenSigner,
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
/// let mut scheduler = ConcurrentMultiPartsUploaderScheduler::new(MultiPartsV2Uploader::new(
///     upload_manager,
///     FileSystemResumableRecorder::<Sha1>::default(),
/// ));
/// scheduler.upload_path("/home/qiniu/test.png", params)?;
/// # Ok(())
/// # }
/// ```
///
/// ##### 异步代码示例
///
/// ```
/// use qiniu_upload_manager::{
///     apis::credential::Credential, prelude::*, ConcurrentMultiPartsUploaderScheduler,
///     FileSystemResumableRecorder, MultiPartsV2Uploader, ObjectParams, UploadManager,
///     UploadTokenSigner,
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
/// let mut scheduler = ConcurrentMultiPartsUploaderScheduler::new(MultiPartsV2Uploader::new(
///     upload_manager,
///     FileSystemResumableRecorder::<Sha1>::default(),
/// ));
/// scheduler.async_upload_path("/home/qiniu/test.png", params).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ConcurrentMultiPartsUploaderScheduler<M: MultiPartsUploader> {
    data_partition_provider: Box<dyn DataPartitionProvider>,
    concurrency_provider: Box<dyn ConcurrencyProvider>,
    multi_parts_uploader: M,
}

impl<M: MultiPartsUploader> ConcurrentMultiPartsUploaderScheduler<M> {
    /// 创建并行分片上传调度器
    #[inline]
    pub fn new(multi_parts_uploader: M) -> Self {
        Self {
            data_partition_provider: Box::new(FixedDataPartitionProvider::new_with_non_zero_part_size(
                #[allow(unsafe_code)]
                unsafe {
                    NonZeroU64::new_unchecked(1 << 22)
                },
            )),
            concurrency_provider: Box::new(FixedConcurrencyProvider::new_with_non_zero_concurrency(
                #[allow(unsafe_code)]
                unsafe {
                    NonZeroUsize::new_unchecked(4)
                },
            )),
            multi_parts_uploader,
        }
    }

    /// 获取并发数提供者
    pub fn concurrency_provider(&self) -> &dyn ConcurrencyProvider {
        &self.concurrency_provider
    }

    /// 获取分片大小提供者
    #[inline]
    pub fn data_partition_provider(&self) -> &dyn DataPartitionProvider {
        &self.data_partition_provider
    }
}

impl<M: MultiPartsUploader + 'static> MultiPartsUploaderScheduler<M::HashAlgorithm>
    for ConcurrentMultiPartsUploaderScheduler<M>
{
    fn set_concurrency_provider(&mut self, concurrency_provider: Box<dyn ConcurrencyProvider>) {
        self.concurrency_provider = concurrency_provider;
    }

    fn set_data_partition_provider(&mut self, data_partition_provider: Box<dyn DataPartitionProvider>) {
        self.data_partition_provider = data_partition_provider;
    }

    fn upload(&self, source: Box<dyn DataSource<M::HashAlgorithm>>, params: ObjectParams) -> ApiResult<Value> {
        let uploaded_size = AtomicU64::new(0);
        let concurrency = self.concurrency_provider.concurrency();
        let begin_at = Instant::now();
        let result = _upload(self, source, params, concurrency, &uploaded_size);
        let elapsed = begin_at.elapsed();
        if let Some(uploaded_size) = NonZeroU64::new(uploaded_size.load(Ordering::SeqCst)) {
            let mut builder = ConcurrencyProviderFeedback::builder(concurrency, uploaded_size, elapsed);
            if let Some(err) = result.as_ref().err() {
                builder.error(err);
            }
            self.concurrency_provider.feedback(builder.build())
        }
        return result;

        fn _upload<M: MultiPartsUploader>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            source: Box<dyn DataSource<M::HashAlgorithm>>,
            params: ObjectParams,
            concurrency: Concurrency,
            uploaded_size: &AtomicU64,
        ) -> ApiResult<Value> {
            let initialized = scheduler.multi_parts_uploader.initialize_parts(source, params)?;
            let thread_pool = ThreadPoolBuilder::new()
                .num_threads(concurrency.as_usize())
                .thread_name(|i| {
                    format!(
                        "qiniu.rust-sdk.upload-manager.scheduler.concurrent_multi_parts_uploader_scheduler.{}",
                        i
                    )
                })
                .build()
                .map_err(|err| ResponseError::new(ResponseErrorKind::SystemCallError, err))?;
            let parts = Mutex::new(Vec::with_capacity(4));
            let any_error = AtomicBool::new(false);
            thread_pool.scope_fifo(|s| {
                s.spawn_fifo(|_| {
                    while !any_error.load(Ordering::SeqCst) {
                        match scheduler
                            .multi_parts_uploader
                            .upload_part(&initialized, &scheduler.data_partition_provider)
                        {
                            Ok(Some(uploaded_part)) => {
                                if !uploaded_part.resumed() {
                                    uploaded_size.fetch_add(uploaded_part.size().get(), Ordering::Relaxed);
                                }
                                parts.lock().unwrap().push(Ok(uploaded_part));
                            }
                            Ok(None) => {
                                return;
                            }
                            Err(err) => {
                                parts.lock().unwrap().push(Err(err));
                                any_error.store(false, Ordering::SeqCst);
                                return;
                            }
                        }
                    }
                })
            });
            let parts = parts.into_inner().unwrap().into_iter().collect::<ApiResult<Vec<_>>>()?;
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
        return Box::pin(async move {
            let uploaded_size = Arc::new(AtomicU64::new(0));
            let concurrency = self.concurrency_provider.concurrency();
            let begin_at = Instant::now();
            let result = _upload(self, source, params, concurrency, uploaded_size.to_owned()).await;
            let elapsed = begin_at.elapsed();
            if let Some(uploaded_size) = NonZeroU64::new(uploaded_size.load(Ordering::SeqCst)) {
                let mut builder = ConcurrencyProviderFeedback::builder(concurrency, uploaded_size, elapsed);
                if let Some(err) = result.as_ref().err() {
                    builder.error(err);
                }
                self.concurrency_provider.feedback(builder.build())
            }
            result
        });

        async fn _upload<M: MultiPartsUploader + 'static>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            source: Box<dyn AsyncDataSource<M::HashAlgorithm>>,
            params: ObjectParams,
            concurrency: Concurrency,
            uploaded_size: Arc<AtomicU64>,
        ) -> ApiResult<Value> {
            let initialized = Arc::new(
                scheduler
                    .multi_parts_uploader
                    .async_initialize_parts(source, params)
                    .await?,
            );
            let any_error = Arc::new(AtomicBool::new(false));
            let results = join_all((0..concurrency.as_usize()).map(|_| {
                let scheduler = scheduler.to_owned();
                let initialized = initialized.to_owned();
                let any_error = any_error.to_owned();
                let uploaded_size = uploaded_size.to_owned();
                spawn(async move {
                    let mut parts = Vec::with_capacity(4);
                    while !any_error.load(Ordering::SeqCst) {
                        match scheduler
                            .multi_parts_uploader
                            .async_upload_part(&initialized, &scheduler.data_partition_provider)
                            .await
                        {
                            Ok(Some(uploaded_part)) => {
                                if !uploaded_part.resumed() {
                                    uploaded_size.fetch_add(uploaded_part.size().get(), Ordering::SeqCst);
                                }
                                parts.push(Ok(uploaded_part));
                            }
                            Ok(None) => {
                                break;
                            }
                            Err(err) => {
                                parts.push(Err(err));
                                any_error.store(true, Ordering::SeqCst);
                                break;
                            }
                        }
                    }
                    parts
                })
            }))
            .await;
            let mut parts = Vec::with_capacity(4);
            for parts_results in results {
                for uploaded_part in parts_results {
                    parts.push(uploaded_part?);
                }
            }
            scheduler
                .multi_parts_uploader
                .async_complete_parts(&initialized, &parts)
                .await
        }
    }
}
