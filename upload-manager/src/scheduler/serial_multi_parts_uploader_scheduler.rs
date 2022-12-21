use super::{
    super::{
        multi_parts_uploader::{MultiPartsUploaderExt, PartsExpiredError},
        ConcurrencyProvider, DataPartitionProvider, DataSource, FixedDataPartitionProvider, MultiPartsUploader,
        ObjectParams, ReinitializeOptions, UploadedPart,
    },
    utils::{
        keep_original_region_options, need_to_retry, no_region_tried_error, remove_used_region_from_regions,
        specify_region_options, UploadPartsError, UploadResumedPartsError,
    },
    MultiPartsUploaderScheduler,
};
use qiniu_apis::http_client::{ApiResult, ResponseError};
use serde_json::Value;
use std::num::NonZeroU64;

#[cfg(feature = "async")]
use {
    super::AsyncDataSource,
    futures::future::{BoxFuture, OptionFuture},
};

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
        return match _resume_and_upload(self, source.to_owned(), params.to_owned()) {
            None => match _try_to_upload_to_all_regions(self, source, params, None) {
                Ok(None) => Err(no_region_tried_error()),
                Ok(Some(value)) => Ok(value),
                Err(err) => Err(err),
            },
            Some(Err(UploadPartsError { err, .. })) if !need_to_retry(&err) => Err(err),
            Some(Err(UploadPartsError { initialized, err })) => {
                match _try_to_upload_to_all_regions(self, source, params, initialized) {
                    Ok(None) => Err(err),
                    Ok(Some(value)) => Ok(value),
                    Err(err) => Err(err),
                }
            }
            Some(Ok(value)) => Ok(value),
        };

        fn _resume_and_upload<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            source: Box<dyn DataSource<M::HashAlgorithm>>,
            params: ObjectParams,
        ) -> Option<Result<Value, UploadPartsError<M::InitializedParts>>> {
            _upload_resumed_parts(scheduler, source, params).map(|result| match result {
                Ok(value) => Ok(value),
                Err(UploadResumedPartsError {
                    err,
                    resumed: true,
                    initialized: Some(mut initialized),
                }) if err.extensions().get::<PartsExpiredError>().is_some() => {
                    match _reinitialize_and_upload_again(scheduler, &mut initialized, keep_original_region_options()) {
                        Some(Ok(value)) => Ok(value),
                        Some(Err(err)) => Err(UploadPartsError::new(err, Some(initialized))),
                        None => Err(UploadPartsError::new(err, Some(initialized))),
                    }
                }
                Err(UploadResumedPartsError { err, initialized, .. }) => Err(UploadPartsError::new(err, initialized)),
            })
        }

        fn _upload_resumed_parts<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            source: Box<dyn DataSource<M::HashAlgorithm>>,
            params: ObjectParams,
        ) -> Option<Result<Value, UploadResumedPartsError<M::InitializedParts>>> {
            scheduler
                .multi_parts_uploader
                .try_to_resume_parts(source, params)
                .map(|initialized| {
                    _upload_after_initialize(scheduler, &initialized)
                        .map_err(|(err, resumed)| UploadResumedPartsError::new(err, resumed, Some(initialized)))
                })
        }

        fn _try_to_upload_to_all_regions<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            source: Box<dyn DataSource<M::HashAlgorithm>>,
            params: ObjectParams,
            mut initialized: Option<M::InitializedParts>,
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
                match _upload_after_reinitialize(scheduler, &new_initialized) {
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
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            initialized: &M::InitializedParts,
        ) -> Result<Value, (ResponseError, bool)> {
            let mut parts = Vec::with_capacity(4);
            let mut resumed = false;
            loop {
                match scheduler
                    .multi_parts_uploader
                    .upload_part(initialized, &scheduler.data_partition_provider)
                {
                    Ok(Some(uploaded_part)) => {
                        if uploaded_part.resumed() {
                            resumed = true;
                        }
                        parts.push(uploaded_part);
                    }
                    Ok(None) => break,
                    Err(err) => return Err((err, resumed)),
                }
            }
            scheduler
                .multi_parts_uploader
                .complete_parts(initialized, &parts)
                .map_err(|err| (err, resumed))
        }

        fn _reinitialize_and_upload_again<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            initialized: &mut M::InitializedParts,
            reinitialize_options: ReinitializeOptions,
        ) -> Option<ApiResult<Value>> {
            scheduler
                .multi_parts_uploader
                .reinitialize_parts(initialized, reinitialize_options)
                .ok()
                .map(|_| _upload_after_reinitialize(scheduler, initialized))
        }

        fn _upload_after_reinitialize<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            initialized: &M::InitializedParts,
        ) -> ApiResult<Value> {
            let mut parts = Vec::with_capacity(4);
            while let Some(uploaded_part) = scheduler
                .multi_parts_uploader
                .upload_part(initialized, &scheduler.data_partition_provider)?
            {
                parts.push(uploaded_part);
            }
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
            match _resume_and_upload(self, source.to_owned(), params.to_owned()).await {
                None => match _try_to_upload_to_all_regions(self, source, params, None).await {
                    Ok(None) => Err(no_region_tried_error()),
                    Ok(Some(value)) => Ok(value),
                    Err(err) => Err(err),
                },
                Some(Err(UploadPartsError { err, .. })) if !need_to_retry(&err) => Err(err),
                Some(Err(UploadPartsError { initialized, err })) => {
                    match _try_to_upload_to_all_regions(self, source, params, initialized).await {
                        Ok(None) => Err(err),
                        Ok(Some(value)) => Ok(value),
                        Err(err) => Err(err),
                    }
                }
                Some(Ok(value)) => Ok(value),
            }
        });

        async fn _resume_and_upload<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            source: Box<dyn AsyncDataSource<M::HashAlgorithm>>,
            params: ObjectParams,
        ) -> Option<Result<Value, UploadPartsError<M::AsyncInitializedParts>>> {
            OptionFuture::from(
                _upload_resumed_parts(scheduler, source, params)
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

        async fn _upload_resumed_parts<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            source: Box<dyn AsyncDataSource<M::HashAlgorithm>>,
            params: ObjectParams,
        ) -> Option<Result<Value, UploadResumedPartsError<M::AsyncInitializedParts>>> {
            OptionFuture::from(
                scheduler
                    .multi_parts_uploader
                    .try_to_async_resume_parts(source, params)
                    .await
                    .map(|initialized| async move {
                        _upload_after_initialize(scheduler, &initialized)
                            .await
                            .map_err(|(err, resumed)| UploadResumedPartsError::new(err, resumed, Some(initialized)))
                    }),
            )
            .await
        }

        async fn _try_to_upload_to_all_regions<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            source: Box<dyn AsyncDataSource<M::HashAlgorithm>>,
            params: ObjectParams,
            mut initialized: Option<M::AsyncInitializedParts>,
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
                match _upload_after_reinitialize(scheduler, &new_initialized).await {
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

        async fn _upload_after_initialize<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            initialized: &M::AsyncInitializedParts,
        ) -> Result<Value, (ResponseError, bool)> {
            let mut parts = Vec::with_capacity(4);
            let mut resumed = false;
            loop {
                match scheduler
                    .multi_parts_uploader
                    .async_upload_part(initialized, &scheduler.data_partition_provider)
                    .await
                {
                    Ok(Some(uploaded_part)) => {
                        if uploaded_part.resumed() {
                            resumed = true;
                        }
                        parts.push(uploaded_part);
                    }
                    Ok(None) => break,
                    Err(err) => return Err((err, resumed)),
                }
            }
            scheduler
                .multi_parts_uploader
                .async_complete_parts(initialized, &parts)
                .await
                .map_err(|err| (err, resumed))
        }

        async fn _reinitialize_and_upload_again<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            initialized: &mut M::AsyncInitializedParts,
            reinitialize_options: ReinitializeOptions,
        ) -> Option<ApiResult<Value>> {
            OptionFuture::from(
                scheduler
                    .multi_parts_uploader
                    .async_reinitialize_parts(initialized, reinitialize_options)
                    .await
                    .ok()
                    .map(|_| _upload_after_reinitialize(scheduler, initialized)),
            )
            .await
        }

        async fn _upload_after_reinitialize<M: MultiPartsUploader>(
            scheduler: &SerialMultiPartsUploaderScheduler<M>,
            initialized: &M::AsyncInitializedParts,
        ) -> ApiResult<Value> {
            let mut parts = Vec::with_capacity(4);
            while let Some(uploaded_part) = scheduler
                .multi_parts_uploader
                .async_upload_part(initialized, &scheduler.data_partition_provider)
                .await?
            {
                parts.push(uploaded_part);
            }
            scheduler
                .multi_parts_uploader
                .async_complete_parts(initialized, &parts)
                .await
        }
    }
}

#[cfg(feature = "async")]
#[cfg(test)]
mod tests {
    use super::{
        super::super::{
            data_source::AsyncDigestible, AsyncFileDataSource, FileSystemResumableRecorder, MultiPartsV1Uploader,
            MultiPartsV2Uploader, UploadManager, UploadTokenSigner,
        },
        *,
    };
    use anyhow::Result as AnyResult;
    use async_std::task::{sleep, spawn as spawn_task};
    use futures::{
        io::{copy as async_io_copy, sink as async_io_sink},
        AsyncRead,
    };
    use qiniu_apis::{
        credential::Credential,
        http::{
            AsyncRequest, AsyncReset, AsyncResponse, AsyncResponseResult, HeaderValue, HttpCaller, StatusCode,
            SyncRequest, SyncResponseResult,
        },
        http_client::{
            AsyncResponseBody, DirectChooser, ErrorRetrier, HttpClient, LimitedRetrier, NeverRetrier, Region,
            RequestRetrier, StaticRegionsProvider, NO_BACKOFF,
        },
    };
    use qiniu_utils::base64::urlsafe as urlsafe_base64;
    use rand::{thread_rng, RngCore};
    use serde_json::{json, to_vec as json_to_vec};
    use sha1::Sha1;
    use std::{
        io::{copy as io_copy, Read, Result as IoResult},
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::{Duration, SystemTime, UNIX_EPOCH},
    };
    use tempfile::{Builder as TempfileBuilder, TempPath};
    use text_io::scan as scan_text;

    const BLOCK_SIZE: u64 = 4 << 20;

    #[async_std::test]
    async fn test_serial_multi_parts_uploader_scheduler_with_async_multi_parts_v1_upload_with_recovery() -> AnyResult<()>
    {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            mkblk_counts: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    if request.url().path().starts_with("/mkblk/") {
                        let blk_size: u64;
                        scan_text!(request.url().path().bytes() => "/mkblk/{}", blk_size);

                        match blk_size {
                            BLOCK_SIZE => {
                                assert_eq!(self.mkblk_counts.fetch_add(1, Ordering::Relaxed), 0);
                            }
                            _ => unreachable!(),
                        }
                        let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                        assert_eq!(body_len, blk_size);
                        let resp_body = json_to_vec(&json!({
                            "ctx": "===0===",
                            "checksum": sha1_of_async_reader(request.body_mut()).await.unwrap(),
                            "offset": blk_size,
                            "host": "http://fakeexample.com",
                            "expired_at": (SystemTime::now()+Duration::from_secs(3600)).duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else if request.url().path().starts_with("/mkfile/") {
                        let resp_body = json_to_vec(&json!({
                            "error": "test error",
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::BAD_REQUEST)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else {
                        unreachable!()
                    }
                })
            }
        }

        let resuming_files_dir = TempfileBuilder::new().tempdir()?;
        let file_path = spawn_task(async { random_file_path(BLOCK_SIZE) }).await?;

        {
            let uploader = SerialMultiPartsUploaderScheduler::new(MultiPartsV1Uploader::new(
                get_upload_manager(FakeHttpCaller::default()),
                FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
            ));
            let file_source = Box::new(AsyncFileDataSource::new(file_path.as_os_str()));
            let params = ObjectParams::builder()
                .region_provider(single_up_domain_region())
                .build();
            uploader.async_upload(file_source, params).await.unwrap_err();
        }

        #[derive(Debug, Default)]
        struct FakeHttpCaller2 {
            mkblk_counts: AtomicUsize,
            mkfile_counts: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller2 {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    if request.url().path().starts_with("/mkblk/") {
                        let blk_size: u64;
                        scan_text!(request.url().path().bytes() => "/mkblk/{}", blk_size);

                        match blk_size {
                            BLOCK_SIZE => {
                                assert_eq!(self.mkblk_counts.fetch_add(1, Ordering::Relaxed), 0);
                            }
                            _ => unreachable!(),
                        }
                        let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                        assert_eq!(body_len, blk_size);
                        let resp_body = json_to_vec(&json!({
                            "ctx": "===0===",
                            "checksum": sha1_of_async_reader(request.body_mut()).await.unwrap(),
                            "offset": blk_size,
                            "host": "http://fakeexample.com",
                            "expired_at": (SystemTime::now()+Duration::from_secs(5)).duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else if request.url().path().starts_with("/mkfile/") {
                        assert!(self.mkfile_counts.fetch_add(1, Ordering::Relaxed) < 2);
                        let resp_body = json_to_vec(&json!({
                            "error": "invalid ctx",
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::from_u16(701).unwrap())
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else {
                        unreachable!()
                    }
                })
            }
        }

        {
            let caller = Arc::new(FakeHttpCaller2::default());
            {
                let uploader = SerialMultiPartsUploaderScheduler::new(MultiPartsV1Uploader::new(
                    get_upload_manager(caller.to_owned()),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                ));
                let file_source = Box::new(AsyncFileDataSource::new(file_path.as_os_str()));
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                uploader.async_upload(file_source, params).await.unwrap_err();
            }
            let caller = Arc::try_unwrap(caller).unwrap();
            assert_eq!(caller.mkblk_counts.into_inner(), 1);
            assert_eq!(caller.mkfile_counts.into_inner(), 2);
        }

        sleep(Duration::from_secs(5)).await;

        #[derive(Debug, Default)]
        struct FakeHttpCaller3 {
            mkblk_counts: AtomicUsize,
            mkfile_counts: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller3 {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    if request.url().path().starts_with("/mkblk/") {
                        let blk_size: u64;
                        scan_text!(request.url().path().bytes() => "/mkblk/{}", blk_size);

                        match blk_size {
                            BLOCK_SIZE => {
                                assert_eq!(self.mkblk_counts.fetch_add(1, Ordering::Relaxed), 0);
                            }
                            _ => unreachable!(),
                        }
                        let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                        assert_eq!(body_len, blk_size);
                        let resp_body = json_to_vec(&json!({
                            "ctx": "===0===",
                            "checksum": sha1_of_async_reader(request.body_mut()).await.unwrap(),
                            "offset": blk_size,
                            "host": "http://fakeexample.com",
                            "expired_at": (SystemTime::now()+Duration::from_secs(3600)).duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else if request.url().path().starts_with("/mkfile/") {
                        assert!(self.mkfile_counts.fetch_add(1, Ordering::Relaxed) < 2);
                        let resp_body = json_to_vec(&json!({
                            "ok": 1,
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else {
                        unreachable!()
                    }
                })
            }
        }

        {
            let caller = Arc::new(FakeHttpCaller3::default());
            {
                let uploader = SerialMultiPartsUploaderScheduler::new(MultiPartsV1Uploader::new(
                    get_upload_manager(caller.to_owned()),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                ));
                let file_source = Box::new(AsyncFileDataSource::new(file_path.as_os_str()));
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let body = uploader.async_upload(file_source, params).await.unwrap();
                assert_eq!(body.get("ok").unwrap().as_i64(), Some(1));
            }
            let caller = Arc::try_unwrap(caller).unwrap();
            assert_eq!(caller.mkblk_counts.into_inner(), 1);
            assert_eq!(caller.mkfile_counts.into_inner(), 1);
        }

        Ok(())
    }

    #[async_std::test]
    async fn test_serial_multi_parts_uploader_scheduler_with_async_multi_parts_v2_upload_with_recovery() -> AnyResult<()>
    {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            init_parts_counts: AtomicUsize,
            upload_part_counts: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            #[cfg(feature = "async")]
            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    if request.url().path() == "/buckets/fakebucket/objects/~/uploads" {
                        assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 0);
                        assert_eq!(self.init_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                        let resp_body = json_to_vec(&json!({
                                "uploadId": "fakeuploadid",
                                "expireAt": (SystemTime::now() + Duration::from_secs(3600)).duration_since(UNIX_EPOCH).unwrap().as_secs(),
                            }))
                            .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else if request
                        .url()
                        .path()
                        .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid/")
                    {
                        let page_number: usize;
                        scan_text!(request.url().path().bytes() => "/buckets/fakebucket/objects/~/uploads/fakeuploadid/{}", page_number);
                        assert_eq!(page_number, 1);
                        let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                        assert_eq!(body_len, BLOCK_SIZE);
                        assert_eq!(self.upload_part_counts.fetch_add(1, Ordering::Relaxed), 0);
                        let resp_body = json_to_vec(&json!({
                            "etag": format!("==={page_number}==="),
                            "md5": "fake-md5",
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else if request
                        .url()
                        .path()
                        .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid")
                    {
                        let resp_body = json_to_vec(&json!({
                            "error": "test error",
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::BAD_REQUEST)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else {
                        unreachable!()
                    }
                })
            }
        }

        let resuming_files_dir = TempfileBuilder::new().tempdir()?;
        let file_path = spawn_task(async { random_file_path(BLOCK_SIZE) }).await?;

        {
            let caller = Arc::new(FakeHttpCaller::default());
            {
                let uploader = SerialMultiPartsUploaderScheduler::new(MultiPartsV2Uploader::new(
                    get_upload_manager(caller.to_owned()),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                ));
                let file_source = Box::new(AsyncFileDataSource::new(file_path.as_os_str()));
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                uploader.async_upload(file_source, params).await.unwrap_err();
            }
            let caller = Arc::try_unwrap(caller).unwrap();
            assert_eq!(caller.init_parts_counts.into_inner(), 1);
            assert_eq!(caller.upload_part_counts.into_inner(), 1);
        }

        #[derive(Debug, Default)]
        struct FakeHttpCaller2 {
            init_parts_counts: AtomicUsize,
            upload_part_counts: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller2 {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            #[cfg(feature = "async")]
            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    if request.url().path() == "/buckets/fakebucket/objects/~/uploads" {
                        assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 0);
                        assert_eq!(self.init_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                        let resp_body = json_to_vec(&json!({
                                "uploadId": "fakeuploadid",
                                "expireAt": (SystemTime::now() + Duration::from_secs(5)).duration_since(UNIX_EPOCH).unwrap().as_secs(),
                            }))
                            .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else if request
                        .url()
                        .path()
                        .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid/")
                    {
                        let page_number: usize;
                        scan_text!(request.url().path().bytes() => "/buckets/fakebucket/objects/~/uploads/fakeuploadid/{}", page_number);
                        assert_eq!(page_number, 1);
                        let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                        assert_eq!(body_len, BLOCK_SIZE);
                        assert_eq!(self.upload_part_counts.fetch_add(1, Ordering::Relaxed), 0);
                        let resp_body = json_to_vec(&json!({
                            "etag": format!("==={page_number}==="),
                            "md5": "fake-md5",
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else if request
                        .url()
                        .path()
                        .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid")
                    {
                        let resp_body = json_to_vec(&json!({
                            "error": "no such uploadId",
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::from_u16(612).unwrap())
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else {
                        unreachable!()
                    }
                })
            }
        }

        {
            let caller = Arc::new(FakeHttpCaller2::default());
            {
                let uploader = SerialMultiPartsUploaderScheduler::new(MultiPartsV2Uploader::new(
                    get_upload_manager(caller.to_owned()),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                ));
                let file_source = Box::new(AsyncFileDataSource::new(file_path.as_os_str()));
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                uploader.async_upload(file_source, params).await.unwrap_err();
            }
            let caller = Arc::try_unwrap(caller).unwrap();
            assert_eq!(caller.init_parts_counts.into_inner(), 1);
            assert_eq!(caller.upload_part_counts.into_inner(), 1);
        }

        sleep(Duration::from_secs(5)).await;

        #[derive(Debug, Default)]
        struct FakeHttpCaller3 {
            init_parts_counts: AtomicUsize,
            upload_part_counts: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller3 {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            #[cfg(feature = "async")]
            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    if request.url().path() == "/buckets/fakebucket/objects/~/uploads" {
                        assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 0);
                        assert_eq!(self.init_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                        let resp_body = json_to_vec(&json!({
                                "uploadId": "fakeuploadid",
                                "expireAt": (SystemTime::now() + Duration::from_secs(5)).duration_since(UNIX_EPOCH).unwrap().as_secs(),
                            }))
                            .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else if request
                        .url()
                        .path()
                        .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid/")
                    {
                        let page_number: usize;
                        scan_text!(request.url().path().bytes() => "/buckets/fakebucket/objects/~/uploads/fakeuploadid/{}", page_number);
                        assert_eq!(page_number, 1);
                        let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                        assert_eq!(body_len, BLOCK_SIZE);
                        assert_eq!(self.upload_part_counts.fetch_add(1, Ordering::Relaxed), 0);
                        let resp_body = json_to_vec(&json!({
                            "etag": format!("==={page_number}==="),
                            "md5": "fake-md5",
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else if request
                        .url()
                        .path()
                        .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid")
                    {
                        let resp_body = json_to_vec(&json!({
                            "ok": 1,
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else {
                        unreachable!()
                    }
                })
            }
        }

        {
            let caller = Arc::new(FakeHttpCaller3::default());
            {
                let uploader = SerialMultiPartsUploaderScheduler::new(MultiPartsV2Uploader::new(
                    get_upload_manager(caller.to_owned()),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                ));
                let file_source = Box::new(AsyncFileDataSource::new(file_path.as_os_str()));
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let body = uploader.async_upload(file_source, params).await.unwrap();
                assert_eq!(body.get("ok").unwrap().as_i64(), Some(1));
            }
            let caller = Arc::try_unwrap(caller).unwrap();
            assert_eq!(caller.init_parts_counts.into_inner(), 1);
            assert_eq!(caller.upload_part_counts.into_inner(), 1);
        }

        #[derive(Debug, Default)]
        struct FakeHttpCaller4 {
            init_parts_counts: AtomicUsize,
            upload_part_counts: AtomicUsize,
            complete_parts_counts: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller4 {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            #[cfg(feature = "async")]
            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    if request.url().path() == "/buckets/fakebucket/objects/~/uploads" {
                        if request.url().host() == Some("fakeup.example.com") {
                            assert_eq!(self.init_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                            assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 0);
                            assert_eq!(self.complete_parts_counts.load(Ordering::Relaxed), 0);
                        } else {
                            assert_eq!(self.init_parts_counts.fetch_add(1, Ordering::Relaxed), 1);
                            assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 1);
                            assert_eq!(self.complete_parts_counts.load(Ordering::Relaxed), 1);
                        }
                        let resp_body = json_to_vec(&json!({
                                "uploadId": "fakeuploadid",
                                "expireAt": (SystemTime::now() + Duration::from_secs(5)).duration_since(UNIX_EPOCH).unwrap().as_secs(),
                            }))
                            .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else if request
                        .url()
                        .path()
                        .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid/")
                    {
                        let page_number: usize;
                        scan_text!(request.url().path().bytes() => "/buckets/fakebucket/objects/~/uploads/fakeuploadid/{}", page_number);
                        assert_eq!(page_number, 1);
                        let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                        assert_eq!(body_len, BLOCK_SIZE);
                        if request.url().host() == Some("fakeup.example.com") {
                            assert_eq!(self.init_parts_counts.load(Ordering::Relaxed), 1);
                            assert_eq!(self.upload_part_counts.fetch_add(1, Ordering::Relaxed), 0);
                            assert_eq!(self.complete_parts_counts.load(Ordering::Relaxed), 0);
                        } else {
                            assert_eq!(self.init_parts_counts.load(Ordering::Relaxed), 2);
                            assert_eq!(self.upload_part_counts.fetch_add(1, Ordering::Relaxed), 1);
                            assert_eq!(self.complete_parts_counts.load(Ordering::Relaxed), 1);
                        }
                        let resp_body = json_to_vec(&json!({
                            "etag": format!("==={page_number}==="),
                            "md5": "fake-md5",
                        }))
                        .unwrap();
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    } else if request
                        .url()
                        .path()
                        .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid")
                    {
                        if request.url().host() == Some("fakeup.example.com") {
                            assert_eq!(self.init_parts_counts.load(Ordering::Relaxed), 1);
                            assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 1);
                            assert_eq!(self.complete_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                            let resp_body = json_to_vec(&json!({
                                "error": "test error",
                            }))
                            .unwrap();
                            Ok(AsyncResponse::builder()
                                .status_code(StatusCode::from_u16(599).unwrap())
                                .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                                .body(AsyncResponseBody::from_bytes(resp_body))
                                .build())
                        } else {
                            assert_eq!(self.init_parts_counts.load(Ordering::Relaxed), 2);
                            assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 2);
                            assert_eq!(self.complete_parts_counts.fetch_add(1, Ordering::Relaxed), 1);
                            let resp_body = json_to_vec(&json!({
                                "ok": 1,
                            }))
                            .unwrap();
                            Ok(AsyncResponse::builder()
                                .status_code(StatusCode::OK)
                                .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                                .body(AsyncResponseBody::from_bytes(resp_body))
                                .build())
                        }
                    } else {
                        unreachable!()
                    }
                })
            }
        }

        {
            let caller = Arc::new(FakeHttpCaller4::default());
            {
                let uploader = SerialMultiPartsUploaderScheduler::new(MultiPartsV2Uploader::new(
                    get_upload_manager_with_retrier(caller.to_owned(), LimitedRetrier::new(ErrorRetrier, 0)),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                ));
                let file_source = Box::new(AsyncFileDataSource::new(file_path.as_os_str()));
                let params = ObjectParams::builder()
                    .region_provider(double_up_domain_region())
                    .build();
                let body = uploader.async_upload(file_source, params).await.unwrap();
                assert_eq!(body.get("ok").unwrap().as_i64(), Some(1));
            }
            let caller = Arc::try_unwrap(caller).unwrap();
            assert_eq!(caller.init_parts_counts.into_inner(), 2);
            assert_eq!(caller.upload_part_counts.into_inner(), 2);
            assert_eq!(caller.complete_parts_counts.into_inner(), 2);
        }

        Ok(())
    }

    async fn size_of_async_reader<R: AsyncRead + AsyncReset + Unpin>(mut reader: &mut R) -> IoResult<u64> {
        let size = async_io_copy(&mut reader, &mut async_io_sink()).await?;
        reader.reset().await?;
        Ok(size)
    }

    async fn sha1_of_async_reader<R: AsyncRead + AsyncReset + Unpin + Send>(reader: &mut R) -> IoResult<String> {
        Ok(urlsafe_base64(
            AsyncDigestible::<Sha1>::digest(reader).await?.as_slice(),
        ))
    }

    fn get_upload_manager(caller: impl HttpCaller + 'static) -> UploadManager {
        get_upload_manager_with_retrier(caller, NeverRetrier)
    }

    fn get_upload_manager_with_retrier(
        caller: impl HttpCaller + 'static,
        retrier: impl RequestRetrier + 'static,
    ) -> UploadManager {
        UploadManager::builder(UploadTokenSigner::new_credential_provider(
            get_credential(),
            "fakebucket",
            Duration::from_secs(100),
        ))
        .http_client(
            HttpClient::builder(caller)
                .chooser(DirectChooser)
                .request_retrier(retrier)
                .backoff(NO_BACKOFF)
                .build(),
        )
        .build()
    }

    fn get_credential() -> Credential {
        Credential::new("fakeaccesskey", "fakesecretkey")
    }

    fn single_up_domain_region() -> Region {
        Region::builder("chaotic")
            .add_up_preferred_endpoint(("fakeup.example.com".to_owned(), 8080).into())
            .build()
    }

    fn double_up_domain_region() -> StaticRegionsProvider {
        let mut provider = StaticRegionsProvider::new(single_up_domain_region());
        provider.append(
            Region::builder("chaotic2")
                .add_up_preferred_endpoint(("fakeup.example2.com".to_owned(), 8080).into())
                .build(),
        );
        provider
    }

    fn random_file_path(size: u64) -> IoResult<TempPath> {
        let mut tempfile = TempfileBuilder::new().tempfile()?;
        let rng = Box::new(thread_rng()) as Box<dyn RngCore>;
        io_copy(&mut rng.take(size), &mut tempfile)?;
        Ok(tempfile.into_temp_path())
    }
}
