use super::{
    super::{
        multi_parts_uploader::{MultiPartsUploaderExt, PartsExpiredError},
        Concurrency, ConcurrencyProvider, ConcurrencyProviderFeedback, DataPartitionProvider, DataSource,
        FixedConcurrencyProvider, FixedDataPartitionProvider, MultiPartsUploader, ObjectParams, ReinitializeOptions,
        UploadedPart,
    },
    utils::{
        keep_original_region_options, need_to_retry, no_region_tried_error, remove_used_region_from_regions,
        specify_region_options, UploadPartsError, UploadResumedPartsError,
    },
    MultiPartsUploaderScheduler,
};
use qiniu_apis::http_client::{ApiResult, ResponseError, ResponseErrorKind};
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde_json::Value;
use std::{
    num::{NonZeroU64, NonZeroUsize},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use tap::Tap;

#[cfg(feature = "async")]
use {
    super::AsyncDataSource,
    async_std::task::spawn,
    futures::future::{join_all, BoxFuture, OptionFuture},
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
                default_non_zero_part_size(),
            )),
            concurrency_provider: Box::new(FixedConcurrencyProvider::new_with_non_zero_concurrency(
                default_non_zero_concurrency(),
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
        let concurrency = self.concurrency_provider.concurrency();
        let thread_pool = ThreadPoolBuilder::new()
            .num_threads(concurrency.as_usize())
            .thread_name(|i| {
                format!("qiniu.rust-sdk.upload-manager.scheduler.concurrent_multi_parts_uploader_scheduler.{i}",)
            })
            .build()
            .map_err(|err| ResponseError::new(ResponseErrorKind::SystemCallError, err))?;
        let mut uploaded_size = Default::default();
        let mut elapsed = Default::default();
        return _upload(
            self,
            source,
            params,
            concurrency,
            &thread_pool,
            &mut elapsed,
            &mut uploaded_size,
        )
        .tap(|result| {
            if let Some(uploaded_size) = NonZeroU64::new(uploaded_size) {
                let mut builder = ConcurrencyProviderFeedback::builder(concurrency, uploaded_size, elapsed);
                if let Some(err) = result.as_ref().err() {
                    builder.error(err);
                }
                self.concurrency_provider.feedback(builder.build())
            }
        });

        fn _upload<M: MultiPartsUploader>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            source: Box<dyn DataSource<M::HashAlgorithm>>,
            params: ObjectParams,
            concurrency: Concurrency,
            thread_pool: &ThreadPool,
            elapsed: &mut Duration,
            uploaded_size: &mut u64,
        ) -> ApiResult<Value> {
            match _resume_and_upload(
                scheduler,
                source.to_owned(),
                params.to_owned(),
                concurrency,
                thread_pool,
                elapsed,
                uploaded_size,
            ) {
                None => {
                    match _try_to_upload_to_all_regions(
                        scheduler,
                        source,
                        params,
                        None,
                        concurrency,
                        thread_pool,
                        elapsed,
                        uploaded_size,
                    ) {
                        Ok(None) => Err(no_region_tried_error()),
                        Ok(Some(value)) => Ok(value),
                        Err(err) => Err(err),
                    }
                }
                Some(Err(UploadPartsError { err, .. })) if !need_to_retry(&err) => Err(err),
                Some(Err(UploadPartsError { initialized, err })) => {
                    match _try_to_upload_to_all_regions(
                        scheduler,
                        source,
                        params,
                        initialized,
                        concurrency,
                        thread_pool,
                        elapsed,
                        uploaded_size,
                    ) {
                        Ok(None) => Err(err),
                        Ok(Some(value)) => Ok(value),
                        Err(err) => Err(err),
                    }
                }
                Some(Ok(value)) => Ok(value),
            }
        }

        fn _resume_and_upload<M: MultiPartsUploader>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            source: Box<dyn DataSource<M::HashAlgorithm>>,
            params: ObjectParams,
            concurrency: Concurrency,
            thread_pool: &ThreadPool,
            elapsed: &mut Duration,
            uploaded_size: &mut u64,
        ) -> Option<Result<Value, UploadPartsError<M::InitializedParts>>> {
            _upload_resumed_parts(
                scheduler,
                source,
                params,
                concurrency,
                thread_pool,
                elapsed,
                uploaded_size,
            )
            .map(|result| match result {
                Ok(value) => Ok(value),
                Err(UploadResumedPartsError {
                    err,
                    resumed: true,
                    initialized: Some(mut initialized),
                }) if err.extensions().get::<PartsExpiredError>().is_some() => {
                    match _reinitialize_and_upload_again(
                        scheduler,
                        &mut initialized,
                        keep_original_region_options(),
                        concurrency,
                        thread_pool,
                        elapsed,
                        uploaded_size,
                    ) {
                        Some(Ok(value)) => Ok(value),
                        Some(Err(err)) => Err(UploadPartsError::new(err, Some(initialized))),
                        None => Err(UploadPartsError::new(err, Some(initialized))),
                    }
                }
                Err(UploadResumedPartsError { err, initialized, .. }) => Err(UploadPartsError::new(err, initialized)),
            })
        }

        fn _upload_resumed_parts<M: MultiPartsUploader>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            source: Box<dyn DataSource<M::HashAlgorithm>>,
            params: ObjectParams,
            concurrency: Concurrency,
            thread_pool: &ThreadPool,
            elapsed: &mut Duration,
            uploaded_size: &mut u64,
        ) -> Option<Result<Value, UploadResumedPartsError<M::InitializedParts>>> {
            let begin_at = Instant::now();
            scheduler
                .multi_parts_uploader
                .try_to_resume_parts(source, params)
                .map(|initialized| {
                    _upload_after_initialize(scheduler, &initialized, concurrency, thread_pool, uploaded_size)
                        .map_err(|(err, resumed)| UploadResumedPartsError::new(err, resumed, Some(initialized)))
                })
                .tap(|_| {
                    *elapsed = begin_at.elapsed();
                })
        }

        #[allow(clippy::too_many_arguments)]
        fn _try_to_upload_to_all_regions<M: MultiPartsUploader>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            source: Box<dyn DataSource<M::HashAlgorithm>>,
            params: ObjectParams,
            mut initialized: Option<M::InitializedParts>,
            concurrency: Concurrency,
            thread_pool: &ThreadPool,
            elapsed: &mut Duration,
            uploaded_size: &mut u64,
        ) -> ApiResult<Option<Value>> {
            let mut regions = scheduler
                .multi_parts_uploader
                .get_bucket_regions(&params)
                .map(|r| r.into_regions())?;
            if let Some(initialized) = &initialized {
                remove_used_region_from_regions(&mut regions, initialized);
            }
            let mut last_err = None;
            for region in regions {
                let begin_at = Instant::now();
                let initialized_result = if let Some(mut initialized) = initialized.take() {
                    scheduler
                        .multi_parts_uploader
                        .reinitialize_parts(&mut initialized, specify_region_options(region))
                        .map(|_| initialized)
                } else {
                    scheduler
                        .multi_parts_uploader
                        .initialize_parts(source.to_owned(), params.to_owned())
                };
                let new_initialized = match initialized_result {
                    Ok(new_initialized) => {
                        initialized = Some(new_initialized.to_owned());
                        new_initialized
                    }
                    Err(err) => {
                        let to_retry = need_to_retry(&err);
                        last_err = Some(err);
                        if to_retry {
                            continue;
                        } else {
                            break;
                        }
                    }
                };
                let result =
                    _upload_after_reinitialize(scheduler, &new_initialized, concurrency, thread_pool, uploaded_size);
                *elapsed = begin_at.elapsed();
                match result {
                    Ok(value) => {
                        return Ok(Some(value));
                    }
                    Err(err) => {
                        let to_retry = need_to_retry(&err);
                        last_err = Some(err);
                        if to_retry {
                            continue;
                        } else {
                            break;
                        }
                    }
                }
            }
            last_err.map_or(Ok(None), Err)
        }

        fn _upload_after_initialize<M: MultiPartsUploader>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            initialized: &M::InitializedParts,
            concurrency: Concurrency,
            thread_pool: &ThreadPool,
            uploaded_size: &mut u64,
        ) -> Result<Value, (ResponseError, bool)> {
            let parts = Mutex::new(Vec::with_capacity(4));
            let atomic_uploaded_size = AtomicU64::new(0);
            let atomic_resumed = AtomicBool::new(false);
            let any_error = AtomicBool::new(false);
            thread_pool.scope_fifo(|s| {
                for _ in 0..concurrency.as_usize() {
                    s.spawn_fifo(|_| {
                        while !any_error.load(Ordering::SeqCst) {
                            match scheduler
                                .multi_parts_uploader
                                .upload_part(initialized, &scheduler.data_partition_provider)
                            {
                                Ok(Some(uploaded_part)) => {
                                    if uploaded_part.resumed() {
                                        if !atomic_resumed.load(Ordering::Relaxed) {
                                            atomic_resumed.store(true, Ordering::Relaxed);
                                        }
                                    } else {
                                        atomic_uploaded_size.fetch_add(uploaded_part.size().get(), Ordering::Relaxed);
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
                    });
                }
            });
            *uploaded_size = atomic_uploaded_size.into_inner();
            let resumed = atomic_resumed.into_inner();
            let parts = parts
                .into_inner()
                .unwrap()
                .into_iter()
                .collect::<ApiResult<Vec<_>>>()
                .map_err(|err| (err, resumed))?;
            scheduler
                .multi_parts_uploader
                .complete_parts(initialized, &parts)
                .map_err(|err| (err, resumed))
        }

        fn _reinitialize_and_upload_again<M: MultiPartsUploader>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            initialized: &mut M::InitializedParts,
            reinitialize_options: ReinitializeOptions,
            concurrency: Concurrency,
            thread_pool: &ThreadPool,
            elapsed: &mut Duration,
            uploaded_size: &mut u64,
        ) -> Option<ApiResult<Value>> {
            let begin_at = Instant::now();
            scheduler
                .multi_parts_uploader
                .reinitialize_parts(initialized, reinitialize_options)
                .ok()
                .map(|_| _upload_after_reinitialize(scheduler, initialized, concurrency, thread_pool, uploaded_size))
                .tap(|_| {
                    *elapsed = begin_at.elapsed();
                })
        }

        fn _upload_after_reinitialize<M: MultiPartsUploader>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            initialized: &M::InitializedParts,
            concurrency: Concurrency,
            thread_pool: &ThreadPool,
            uploaded_size: &mut u64,
        ) -> ApiResult<Value> {
            let parts = Mutex::new(Vec::with_capacity(4));
            let atomic_uploaded_size = AtomicU64::new(0);
            let any_error = AtomicBool::new(false);
            thread_pool.scope_fifo(|s| {
                for _ in 0..concurrency.as_usize() {
                    s.spawn_fifo(|_| {
                        while !any_error.load(Ordering::SeqCst) {
                            match scheduler
                                .multi_parts_uploader
                                .upload_part(initialized, &scheduler.data_partition_provider)
                            {
                                Ok(Some(uploaded_part)) => {
                                    if !uploaded_part.resumed() {
                                        atomic_uploaded_size.fetch_add(uploaded_part.size().get(), Ordering::Relaxed);
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
                }
            });
            *uploaded_size = atomic_uploaded_size.into_inner();
            let parts = parts.into_inner().unwrap().into_iter().collect::<ApiResult<Vec<_>>>()?;
            scheduler.multi_parts_uploader.complete_parts(initialized, &parts)
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
            let concurrency = self.concurrency_provider.concurrency();
            let mut uploaded_size = Default::default();
            let mut elapsed = Default::default();
            _upload(self, source, params, concurrency, &mut elapsed, &mut uploaded_size)
                .await
                .tap(|result| {
                    if let Some(uploaded_size) = NonZeroU64::new(uploaded_size) {
                        let mut builder = ConcurrencyProviderFeedback::builder(concurrency, uploaded_size, elapsed);
                        if let Some(err) = result.as_ref().err() {
                            builder.error(err);
                        }
                        self.concurrency_provider.feedback(builder.build())
                    }
                })
        });

        async fn _upload<M: MultiPartsUploader + 'static>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            source: Box<dyn AsyncDataSource<M::HashAlgorithm>>,
            params: ObjectParams,
            concurrency: Concurrency,
            elapsed: &mut Duration,
            uploaded_size: &mut u64,
        ) -> ApiResult<Value> {
            match _resume_and_upload(
                scheduler,
                source.to_owned(),
                params.to_owned(),
                concurrency,
                elapsed,
                uploaded_size,
            )
            .await
            {
                None => {
                    match _try_to_upload_to_all_regions(
                        scheduler,
                        source,
                        params,
                        None,
                        concurrency,
                        elapsed,
                        uploaded_size,
                    )
                    .await
                    {
                        Ok(None) => Err(no_region_tried_error()),
                        Ok(Some(value)) => Ok(value),
                        Err(err) => Err(err),
                    }
                }
                Some(Err(UploadPartsError { err, .. })) if !need_to_retry(&err) => Err(err),
                Some(Err(UploadPartsError { initialized, err })) => {
                    match _try_to_upload_to_all_regions(
                        scheduler,
                        source,
                        params,
                        initialized,
                        concurrency,
                        elapsed,
                        uploaded_size,
                    )
                    .await
                    {
                        Ok(None) => Err(err),
                        Ok(Some(value)) => Ok(value),
                        Err(err) => Err(err),
                    }
                }
                Some(Ok(value)) => Ok(value),
            }
        }

        async fn _resume_and_upload<M: MultiPartsUploader + 'static>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            source: Box<dyn AsyncDataSource<M::HashAlgorithm>>,
            params: ObjectParams,
            concurrency: Concurrency,
            elapsed: &mut Duration,
            uploaded_size: &mut u64,
        ) -> Option<Result<Value, UploadPartsError<M::AsyncInitializedParts>>> {
            OptionFuture::from(
                _upload_resumed_parts(scheduler, source, params, concurrency, elapsed, uploaded_size)
                    .await
                    .map(|result| async move {
                        match result {
                            Ok(value) => Ok(value),
                            Err(UploadResumedPartsError {
                                err,
                                resumed: true,
                                initialized: Some(mut initialized),
                            }) if err.extensions().get::<PartsExpiredError>().is_some() => {
                                match _reinitialize_and_upload_again(
                                    scheduler,
                                    &mut initialized,
                                    keep_original_region_options(),
                                    concurrency,
                                    elapsed,
                                    uploaded_size,
                                )
                                .await
                                {
                                    Some(Ok(value)) => Ok(value),
                                    Some(Err(err)) => Err(UploadPartsError::new(err, Some(initialized))),
                                    None => Err(UploadPartsError::new(err, Some(initialized))),
                                }
                            }
                            Err(UploadResumedPartsError { err, initialized, .. }) => {
                                Err(UploadPartsError::new(err, initialized))
                            }
                        }
                    }),
            )
            .await
        }

        async fn _upload_resumed_parts<M: MultiPartsUploader + 'static>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            source: Box<dyn AsyncDataSource<M::HashAlgorithm>>,
            params: ObjectParams,
            concurrency: Concurrency,
            elapsed: &mut Duration,
            uploaded_size: &mut u64,
        ) -> Option<Result<Value, UploadResumedPartsError<M::AsyncInitializedParts>>> {
            let begin_at = Instant::now();
            OptionFuture::from(
                scheduler
                    .multi_parts_uploader
                    .try_to_async_resume_parts(source, params)
                    .await
                    .map(|initialized| async move {
                        _upload_after_initialize(scheduler, initialized.to_owned(), concurrency, uploaded_size)
                            .await
                            .map_err(|(err, resumed)| UploadResumedPartsError::new(err, resumed, Some(initialized)))
                    }),
            )
            .await
            .tap(|_| {
                *elapsed = begin_at.elapsed();
            })
        }

        async fn _try_to_upload_to_all_regions<M: MultiPartsUploader + 'static>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            source: Box<dyn AsyncDataSource<M::HashAlgorithm>>,
            params: ObjectParams,
            mut initialized: Option<M::AsyncInitializedParts>,
            concurrency: Concurrency,
            elapsed: &mut Duration,
            uploaded_size: &mut u64,
        ) -> ApiResult<Option<Value>> {
            let mut regions = scheduler
                .multi_parts_uploader
                .async_get_bucket_regions(&params)
                .await
                .map(|r| r.into_regions())?;
            if let Some(initialized) = &initialized {
                remove_used_region_from_regions(&mut regions, initialized);
            }
            let mut last_err = None;
            for region in regions {
                let begin_at = Instant::now();
                let initialized_result = if let Some(mut initialized) = initialized.take() {
                    scheduler
                        .multi_parts_uploader
                        .async_reinitialize_parts(&mut initialized, specify_region_options(region))
                        .await
                        .map(|_| initialized)
                } else {
                    scheduler
                        .multi_parts_uploader
                        .async_initialize_parts(source.to_owned(), params.to_owned())
                        .await
                };
                let new_initialized = match initialized_result {
                    Ok(new_initialized) => {
                        initialized = Some(new_initialized.to_owned());
                        new_initialized
                    }
                    Err(err) => {
                        let to_retry = need_to_retry(&err);
                        last_err = Some(err);
                        if to_retry {
                            continue;
                        } else {
                            break;
                        }
                    }
                };
                let result = _upload_after_reinitialize(scheduler, new_initialized, concurrency, uploaded_size).await;
                *elapsed = begin_at.elapsed();
                match result {
                    Ok(value) => {
                        return Ok(Some(value));
                    }
                    Err(err) => {
                        let to_retry = need_to_retry(&err);
                        last_err = Some(err);
                        if to_retry {
                            continue;
                        } else {
                            break;
                        }
                    }
                }
            }
            last_err.map_or(Ok(None), Err)
        }

        async fn _upload_after_initialize<M: MultiPartsUploader + 'static>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            initialized: M::AsyncInitializedParts,
            concurrency: Concurrency,
            uploaded_size: &mut u64,
        ) -> Result<Value, (ResponseError, bool)> {
            let initialized = Arc::new(initialized);
            let atomic_uploaded_size = Arc::new(AtomicU64::new(0));
            let atomic_resumed = Arc::new(AtomicBool::new(false));
            let any_error = Arc::new(AtomicBool::new(false));
            let results = join_all((0..concurrency.as_usize()).map(|_| {
                let scheduler = scheduler.to_owned();
                let initialized = initialized.to_owned();
                let any_error = any_error.to_owned();
                let atomic_uploaded_size = atomic_uploaded_size.to_owned();
                let atomic_resumed = atomic_resumed.to_owned();
                spawn(async move {
                    let mut parts = Vec::with_capacity(4);
                    while !any_error.load(Ordering::SeqCst) {
                        match scheduler
                            .multi_parts_uploader
                            .async_upload_part(&initialized, &scheduler.data_partition_provider)
                            .await
                        {
                            Ok(Some(uploaded_part)) => {
                                if uploaded_part.resumed() {
                                    if !atomic_resumed.load(Ordering::Relaxed) {
                                        atomic_resumed.store(true, Ordering::Relaxed);
                                    }
                                } else {
                                    atomic_uploaded_size.fetch_add(uploaded_part.size().get(), Ordering::Relaxed);
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
            let initialized = Arc::try_unwrap(initialized).unwrap();
            *uploaded_size = Arc::try_unwrap(atomic_uploaded_size).unwrap().into_inner();
            let resumed = Arc::try_unwrap(atomic_resumed).unwrap().into_inner();
            let parts = results
                .into_iter()
                .flatten()
                .collect::<ApiResult<Vec<_>>>()
                .map_err(|err| (err, resumed))?;
            scheduler
                .multi_parts_uploader
                .async_complete_parts(&initialized, &parts)
                .await
                .map_err(|err| (err, resumed))
        }

        async fn _reinitialize_and_upload_again<M: MultiPartsUploader + 'static>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            initialized: &mut M::AsyncInitializedParts,
            reinitialize_options: ReinitializeOptions,
            concurrency: Concurrency,
            elapsed: &mut Duration,
            uploaded_size: &mut u64,
        ) -> Option<ApiResult<Value>> {
            let begin_at = Instant::now();
            OptionFuture::from(
                scheduler
                    .multi_parts_uploader
                    .async_reinitialize_parts(initialized, reinitialize_options)
                    .await
                    .ok()
                    .map(|_| _upload_after_reinitialize(scheduler, initialized.to_owned(), concurrency, uploaded_size)),
            )
            .await
            .tap(|_| {
                *elapsed = begin_at.elapsed();
            })
        }

        async fn _upload_after_reinitialize<M: MultiPartsUploader + 'static>(
            scheduler: &ConcurrentMultiPartsUploaderScheduler<M>,
            initialized: M::AsyncInitializedParts,
            concurrency: Concurrency,
            uploaded_size: &mut u64,
        ) -> ApiResult<Value> {
            let initialized = Arc::new(initialized);
            let atomic_uploaded_size = Arc::new(AtomicU64::new(0));
            let atomic_any_error = Arc::new(AtomicBool::new(false));
            let results = join_all((0..concurrency.as_usize()).map(|_| {
                let scheduler = scheduler.to_owned();
                let initialized = initialized.to_owned();
                let atomic_any_error = atomic_any_error.to_owned();
                let atomic_uploaded_size = atomic_uploaded_size.to_owned();
                spawn(async move {
                    let mut parts = Vec::with_capacity(4);
                    while !atomic_any_error.load(Ordering::SeqCst) {
                        match scheduler
                            .multi_parts_uploader
                            .async_upload_part(&initialized, &scheduler.data_partition_provider)
                            .await
                        {
                            Ok(Some(uploaded_part)) => {
                                if !uploaded_part.resumed() {
                                    atomic_uploaded_size.fetch_add(uploaded_part.size().get(), Ordering::Relaxed);
                                }
                                parts.push(Ok(uploaded_part));
                            }
                            Ok(None) => {
                                break;
                            }
                            Err(err) => {
                                parts.push(Err(err));
                                atomic_any_error.store(true, Ordering::SeqCst);
                                break;
                            }
                        }
                    }
                    parts
                })
            }))
            .await;
            let initialized = Arc::try_unwrap(initialized).unwrap();
            *uploaded_size = Arc::try_unwrap(atomic_uploaded_size).unwrap().into_inner();
            let parts = results.into_iter().flatten().collect::<ApiResult<Vec<_>>>()?;
            scheduler
                .multi_parts_uploader
                .async_complete_parts(&initialized, &parts)
                .await
        }
    }
}

fn default_non_zero_concurrency() -> NonZeroUsize {
    #[allow(unsafe_code)]
    unsafe {
        NonZeroUsize::new_unchecked(4)
    }
}

fn default_non_zero_part_size() -> NonZeroU64 {
    #[allow(unsafe_code)]
    unsafe {
        NonZeroU64::new_unchecked(1 << 22)
    }
}
