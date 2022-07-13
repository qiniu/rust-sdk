use super::{
    super::{
        callbacks::{Callbacks, UploadingProgressInfo},
        data_source::{Digestible, SourceKey},
        upload_token::OwnedUploadTokenProviderOrReferenced,
        AppendOnlyResumableRecorderMedium, DataPartitionProvider, DataPartitionProviderFeedback, DataSourceReader,
        LimitedDataPartitionProvider, UploaderWithCallbacks,
    },
    progress::{Progresses, ProgressesKey},
    up_endpoints::UpEndpoints,
    v1::make_callback_error,
    DataSource, InitializedParts, MultiPartsUploader, MultiPartsUploaderWithCallbacks, ObjectParams, ResumableRecorder,
    UploadManager, UploadedPart,
};
use anyhow::Result as AnyResult;
use dashmap::DashMap;
use digest::Digest;
use qiniu_apis::{
    base_types::StringMap,
    credential::AccessKey,
    http::{Reset, ResponseParts},
    http_client::{
        ApiResult, BucketRegionsProvider, Endpoint, EndpointsProvider, RegionsProviderEndpoints, RequestBuilderParts,
        Response, ResponseError,
    },
    storage::{
        self,
        resumable_upload_v2_complete_multipart_upload::{
            PartInfo, PathParams as CompletePartsPathParams, RequestBody as CompletePartsRequestBody,
            SyncRequestBuilder as SyncCompletePartsRequestBuilder,
        },
        resumable_upload_v2_initiate_multipart_upload::{
            PathParams as InitPartsPathParams, ResponseBody as InitPartsResponseBody,
            SyncRequestBuilder as SyncInitPartsRequestBuilder,
        },
        resumable_upload_v2_upload_part::{
            PathParams as UploadPartPathParams, ResponseBody as UploadPartResponseBody,
            SyncRequestBuilder as SyncUploadPartRequestBuilder,
        },
    },
};
use qiniu_upload_token::{BucketName, ObjectName};
use qiniu_utils::base64::urlsafe as urlsafe_base64;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::Sha1;
use std::{
    fmt::{self, Debug},
    io::{BufRead, BufReader, Read, Result as IoResult, Write},
    iter::FromIterator,
    num::{NonZeroU64, NonZeroUsize},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "async")]
use {
    super::super::{
        data_source::{AsyncDataSource, AsyncDigestible},
        AppendOnlyAsyncResumableRecorderMedium, AsyncDataSourceReader,
    },
    futures::{
        future::{BoxFuture, OptionFuture},
        io::{AsyncRead, BufReader as AsyncBufReader},
        lock::Mutex as AsyncMutex,
        AsyncBufReadExt, AsyncWriteExt, StreamExt, TryStreamExt,
    },
    qiniu_apis::{
        http::AsyncReset,
        storage::{
            resumable_upload_v2_complete_multipart_upload::AsyncRequestBuilder as AsyncCompletePartsRequestBuilder,
            resumable_upload_v2_initiate_multipart_upload::AsyncRequestBuilder as AsyncInitPartsRequestBuilder,
            resumable_upload_v2_upload_part::AsyncRequestBuilder as AsyncUploadPartRequestBuilder,
        },
    },
};

/// 分片上传器 V2
///
/// 不推荐直接使用这个上传器，而是可以借助 [`crate::MultiPartsUploaderScheduler`] 来方便地实现分片上传。
pub struct MultiPartsV2Uploader<H: Digest = Sha1> {
    upload_manager: UploadManager,
    callbacks: Callbacks<'static>,
    resumable_recorder: Arc<dyn ResumableRecorder<HashAlgorithm = H>>,
}

/// 被 分片上传器 V2 初始化的分片信息
pub struct MultiPartsV2UploaderInitializedObject<S> {
    upload_id: String,
    source: S,
    params: ObjectParams,
    progresses: Progresses,
    recovered_records: MultiPartsV2ResumableRecorderRecords,
    initialized_at: SystemTime,
    up_endpoints: UpEndpoints,
}

impl<S> MultiPartsV2UploaderInitializedObject<S> {
    /// 获得上传 ID
    #[inline]
    pub fn upload_id(&self) -> &str {
        &self.upload_id
    }

    /// 获得初始化时间
    #[inline]
    pub fn initialized_at(&self) -> SystemTime {
        self.initialized_at
    }
}

impl<S: Debug + Send + Sync> InitializedParts for MultiPartsV2UploaderInitializedObject<S> {
    #[inline]
    fn params(&self) -> &ObjectParams {
        &self.params
    }
}

impl<S: Debug> Debug for MultiPartsV2UploaderInitializedObject<S> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiPartsV2UploaderInitializedObject")
            .field("upload_id", &self.upload_id)
            .field("source", &self.source)
            .field("params", &self.params)
            .field("progresses", &self.progresses)
            .field("recovered_records", &self.recovered_records)
            .field("initialized_at", &self.initialized_at)
            .field("up_endpoints", &self.up_endpoints)
            .finish()
    }
}

/// 已经通过 分片上传器 V2 上传的分片信息
#[derive(Debug, Clone)]
pub struct MultiPartsV2UploaderUploadedPart {
    response_body: UploadPartResponseBody,
    uploaded_size: NonZeroU64,
    offset: u64,
    part_number: NonZeroUsize,
    resumed: bool,
}

impl MultiPartsV2UploaderUploadedPart {
    /// 获取响应体
    #[inline]
    pub fn response_body(&self) -> &UploadPartResponseBody {
        &self.response_body
    }

    /// 获取分片大小
    #[inline]
    pub fn part_number(&self) -> NonZeroUsize {
        self.part_number
    }
}

impl UploadedPart for MultiPartsV2UploaderUploadedPart {
    #[inline]
    fn size(&self) -> NonZeroU64 {
        self.uploaded_size
    }

    #[inline]
    fn offset(&self) -> u64 {
        self.offset
    }

    #[inline]
    fn resumed(&self) -> bool {
        self.resumed
    }
}

impl<H: Digest> UploaderWithCallbacks for MultiPartsV2Uploader<H> {
    #[inline]
    fn on_before_request<F: Fn(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_before_request_callback(callback);
        self
    }

    #[inline]
    fn on_upload_progress<F: Fn(&UploadingProgressInfo) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_upload_progress_callback(callback);
        self
    }

    #[inline]
    fn on_response_ok<F: Fn(&mut ResponseParts) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_after_response_ok_callback(callback);
        self
    }

    #[inline]
    fn on_response_error<F: Fn(&ResponseError) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_after_response_error_callback(callback);
        self
    }
}

impl<H: Digest> MultiPartsV2Uploader<H> {
    #[inline]
    pub(crate) fn new_with_callbacks<R: ResumableRecorder<HashAlgorithm = H> + 'static>(
        upload_manager: UploadManager,
        callbacks: Callbacks<'static>,
        resumable_recorder: R,
    ) -> Self {
        Self {
            upload_manager,
            callbacks,
            resumable_recorder: Arc::new(resumable_recorder),
        }
    }
}

impl<H: Digest> MultiPartsUploaderWithCallbacks for MultiPartsV2Uploader<H> {
    #[inline]
    fn on_part_uploaded<F: Fn(&dyn UploadedPart) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_part_uploaded_callback(callback);
        self
    }
}

impl<H: Digest + Send + 'static> MultiPartsUploader for MultiPartsV2Uploader<H> {
    type HashAlgorithm = H;
    type InitializedParts = MultiPartsV2UploaderInitializedObject<Box<dyn DataSource<H>>>;
    type UploadedPart = MultiPartsV2UploaderUploadedPart;

    #[inline]
    fn new<R: ResumableRecorder<HashAlgorithm = Self::HashAlgorithm> + 'static>(
        upload_manager: UploadManager,
        resumable_recorder: R,
    ) -> Self {
        Self {
            upload_manager,
            callbacks: Default::default(),
            resumable_recorder: Arc::new(resumable_recorder),
        }
    }

    fn initialize_parts<D: DataSource<Self::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> ApiResult<Self::InitializedParts> {
        let up_endpoints = self.up_endpoints(&params)?;
        let (upload_id, recovered_records) = self.try_to_recover(&source, &params, &up_endpoints).unwrap_or_default();
        let upload_id = if let Some(upload_id) = upload_id {
            upload_id
        } else {
            let path_params =
                make_init_parts_path_params_from_initialized_params(self.bucket_name()?.to_string(), &params);
            let upload_token_signer = self.make_upload_token_signer(params.object_name().map(|n| n.into()));
            let init_parts = self.storage().resumable_upload_v2_initiate_multipart_upload();
            let initialized_parts = _initialize_parts(
                self,
                init_parts.new_request(&up_endpoints, path_params, upload_token_signer.as_ref()),
            )?;
            initialized_parts.get_upload_id_as_str().to_owned()
        };

        return Ok(Self::InitializedParts {
            source: Box::new(source),
            params,
            upload_id,
            recovered_records,
            up_endpoints,
            progresses: Default::default(),
            initialized_at: SystemTime::now(),
        });

        fn _initialize_parts<'a, H: Digest, E: EndpointsProvider + Clone + 'a>(
            uploader: &'a MultiPartsV2Uploader<H>,
            mut request: SyncInitPartsRequestBuilder<'a, E>,
        ) -> ApiResult<InitPartsResponseBody> {
            uploader.before_request_call(request.parts_mut())?;
            let mut response_result = request.call();
            uploader.after_response_call(&mut response_result)?;
            Ok(response_result?.into_body())
        }
    }

    fn upload_part(
        &self,
        initialized: &Self::InitializedParts,
        data_partitioner_provider: &dyn DataPartitionProvider,
    ) -> ApiResult<Option<Self::UploadedPart>> {
        let data_partitioner_provider = LimitedDataPartitionProvider::new_with_non_zero_threshold(
            data_partitioner_provider,
            MIN_PART_SIZE,
            MAX_PART_SIZE,
        );
        let total_size = initialized.source.total_size()?;
        return if let Some(mut reader) = initialized.source.slice(data_partitioner_provider.part_size())? {
            if let Some(part_size) = NonZeroU64::new(reader.len()?) {
                let progresses_key = initialized.progresses.add_new_part(part_size.into());
                if let Some(uploaded_part) = _could_recover(
                    initialized,
                    &mut reader,
                    part_size,
                    initialized.params.uploaded_part_ttl(),
                ) {
                    self.after_part_uploaded(&progresses_key, total_size, Some(&uploaded_part))?;
                    Ok(Some(uploaded_part))
                } else {
                    let path_params = make_upload_part_path_params_from_initialized_params(
                        self.bucket_name()?.to_string(),
                        &initialized.params,
                        initialized.upload_id.to_owned(),
                        reader.part_number(),
                    );
                    let upload_token_signer =
                        self.make_upload_token_signer(initialized.params.object_name().map(|n| n.into()));
                    let upload_part = self.storage().resumable_upload_v2_upload_part();
                    let uploaded_result = _upload_part(
                        self,
                        upload_part.new_request(&initialized.up_endpoints, path_params, upload_token_signer.as_ref()),
                        reader,
                        part_size,
                        initialized,
                        &progresses_key,
                        &data_partitioner_provider,
                    );

                    match uploaded_result {
                        Ok(uploaded_part) => {
                            self.after_part_uploaded(&progresses_key, total_size, Some(&uploaded_part))?;
                            Ok(Some(uploaded_part))
                        }
                        Err(err) => {
                            self.after_part_uploaded(&progresses_key, total_size, None).ok();
                            Err(err)
                        }
                    }
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        };

        fn _could_recover<H: Digest>(
            initialized: &MultiPartsV2UploaderInitializedObject<Box<dyn DataSource<H>>>,
            data_reader: &mut DataSourceReader,
            part_size: NonZeroU64,
            uploaded_part_ttl: Duration,
        ) -> Option<MultiPartsV2UploaderUploadedPart> {
            let offset = data_reader.offset();
            initialized.recovered_records.take(offset).and_then(|record| {
                if record.size == part_size
                    && record.part_number == data_reader.part_number()
                    && initialized.initialized_at + uploaded_part_ttl > SystemTime::now()
                    && Some(record.base64ed_sha1.as_str()) == sha1_of_sync_reader(data_reader).ok().as_deref()
                {
                    Some(MultiPartsV2UploaderUploadedPart {
                        response_body: record.response_body.to_owned(),
                        uploaded_size: record.size,
                        part_number: record.part_number,
                        resumed: true,
                        offset,
                    })
                } else {
                    None
                }
            })
        }

        fn _upload_part<'a, H: Digest, E: EndpointsProvider + Clone + 'a>(
            uploader: &'a MultiPartsV2Uploader<H>,
            mut request: SyncUploadPartRequestBuilder<'a, E>,
            mut body: DataSourceReader,
            content_length: NonZeroU64,
            initialized: &'a MultiPartsV2UploaderInitializedObject<Box<dyn DataSource<H>>>,
            progresses_key: &'a ProgressesKey,
            data_partitioner_provider: &'a dyn DataPartitionProvider,
        ) -> ApiResult<MultiPartsV2UploaderUploadedPart> {
            let total_size = initialized.source.total_size()?;
            request.on_uploading_progress(move |_, transfer| {
                progresses_key.update_part(transfer.transferred_bytes());
                uploader.callbacks.upload_progress(&UploadingProgressInfo::new(
                    progresses_key.current_uploaded(),
                    total_size,
                ))
            });
            uploader.before_request_call(request.parts_mut())?;
            let body_offset = body.offset();
            let part_number = body.part_number();
            let base64ed_sha1 = sha1_of_sync_reader(&mut body)?;
            let begin_at = Instant::now();
            let mut response_result = request.call(body, content_length.get());
            let elapsed = begin_at.elapsed();
            uploader.after_response_call(&mut response_result)?;
            let mut feedback_builder =
                DataPartitionProviderFeedback::builder(content_length.into(), elapsed, initialized.params.extensions());
            if let Some(err) = response_result.as_ref().err() {
                feedback_builder.error(err);
            }
            data_partitioner_provider.feedback(feedback_builder.build());
            let response_body = response_result?.into_body();
            let record = MultiPartsV2ResumableRecorderRecord {
                response_body,
                offset: body_offset,
                size: content_length,
                base64ed_sha1,
                part_number,
            };
            initialized
                .recovered_records
                .persist(
                    &initialized.upload_id,
                    initialized.initialized_at,
                    &record,
                    &uploader.bucket_name()?,
                    initialized.params.object_name(),
                    &initialized.up_endpoints,
                )
                .ok();
            Ok(MultiPartsV2UploaderUploadedPart::from_record(record, false))
        }
    }

    fn complete_parts(&self, initialized: &Self::InitializedParts, parts: &[Self::UploadedPart]) -> ApiResult<Value> {
        let upload_token_signer = self.make_upload_token_signer(initialized.params.object_name().map(|n| n.into()));
        let path_params = make_complete_parts_path_params_from_initialized_params(
            self.bucket_name()?.to_string(),
            &initialized.params,
            initialized.upload_id.to_owned(),
        );
        let body = make_complete_parts_request_body_from_initialized_params(&initialized.params, parts.to_vec());
        let complete_parts = self.storage().resumable_upload_v2_complete_multipart_upload();
        return _complete_parts(
            self,
            complete_parts.new_request(&initialized.up_endpoints, path_params, upload_token_signer.as_ref()),
            &initialized.source,
            body,
        );

        fn _complete_parts<'a, H: Digest, E: EndpointsProvider + Clone + 'a, D: DataSource<H>>(
            uploader: &'a MultiPartsV2Uploader<H>,
            mut request: SyncCompletePartsRequestBuilder<'a, E>,
            source: &D,
            body: CompletePartsRequestBody,
        ) -> ApiResult<Value> {
            uploader.before_request_call(request.parts_mut())?;
            let mut response_result = request.call(&body);
            uploader.after_response_call(&mut response_result)?;
            let body = response_result?.into_body();
            uploader.try_to_delete_records(&source).ok();
            Ok(body.into())
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    type AsyncInitializedParts = MultiPartsV2UploaderInitializedObject<Box<dyn AsyncDataSource<H>>>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    type AsyncUploadedPart = MultiPartsV2UploaderUploadedPart;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_initialize_parts<D: AsyncDataSource<Self::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Self::AsyncInitializedParts>> {
        return Box::pin(async move {
            let up_endpoints = self.async_up_endpoints(&params).await?;
            let (upload_id, recovered_records) = self
                .try_to_async_recover(&source, &params, &up_endpoints)
                .await
                .unwrap_or_default();
            let upload_id = if let Some(upload_id) = upload_id {
                upload_id
            } else {
                let path_params =
                    make_init_parts_path_params_from_initialized_params(self.bucket_name()?.to_string(), &params);
                let upload_token_signer = self.make_upload_token_signer(params.object_name().map(|n| n.into()));
                let init_parts = self.storage().resumable_upload_v2_initiate_multipart_upload();
                let initialized_parts = _initialize_parts(
                    self,
                    init_parts.new_async_request(&up_endpoints, path_params, upload_token_signer.as_ref()),
                )
                .await?;
                initialized_parts.get_upload_id_as_str().to_owned()
            };

            Ok(Self::AsyncInitializedParts {
                source: Box::new(source),
                params,
                upload_id,
                recovered_records,
                up_endpoints,
                progresses: Default::default(),
                initialized_at: SystemTime::now(),
            })
        });

        async fn _initialize_parts<'a, H: Digest, E: EndpointsProvider + Clone + 'a>(
            uploader: &'a MultiPartsV2Uploader<H>,
            mut request: AsyncInitPartsRequestBuilder<'a, E>,
        ) -> ApiResult<InitPartsResponseBody> {
            uploader.before_request_call(request.parts_mut())?;
            let mut response_result = request.call().await;
            uploader.after_response_call(&mut response_result)?;
            Ok(response_result?.into_body())
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_part<'r>(
        &'r self,
        initialized: &'r Self::AsyncInitializedParts,
        data_partitioner_provider: &'r dyn DataPartitionProvider,
    ) -> BoxFuture<'r, ApiResult<Option<Self::AsyncUploadedPart>>> {
        return Box::pin(async move {
            let data_partitioner_provider = LimitedDataPartitionProvider::new_with_non_zero_threshold(
                data_partitioner_provider,
                MIN_PART_SIZE,
                MAX_PART_SIZE,
            );
            let total_size = initialized.source.total_size().await?;
            if let Some(mut reader) = initialized.source.slice(data_partitioner_provider.part_size()).await? {
                if let Some(part_size) = NonZeroU64::new(reader.len().await?) {
                    let progresses_key = initialized.progresses.add_new_part(part_size.get());
                    if let Some(uploaded_part) = _could_recover(
                        initialized,
                        &mut reader,
                        part_size,
                        initialized.params.uploaded_part_ttl(),
                    )
                    .await
                    {
                        self.after_part_uploaded(&progresses_key, total_size, Some(&uploaded_part))?;
                        Ok(Some(uploaded_part))
                    } else {
                        let path_params = make_upload_part_path_params_from_initialized_params(
                            self.bucket_name()?.to_string(),
                            &initialized.params,
                            initialized.upload_id.to_string(),
                            reader.part_number(),
                        );
                        let upload_token_signer =
                            self.make_upload_token_signer(initialized.params.object_name().map(|n| n.into()));
                        let upload_part = self.storage().resumable_upload_v2_upload_part();
                        let uploaded_result = _upload_part(
                            self,
                            upload_part.new_async_request(
                                &initialized.up_endpoints,
                                path_params,
                                upload_token_signer.as_ref(),
                            ),
                            reader,
                            part_size,
                            initialized,
                            &progresses_key,
                            &data_partitioner_provider,
                        )
                        .await;
                        match uploaded_result {
                            Ok(uploaded_part) => {
                                self.after_part_uploaded(&progresses_key, total_size, Some(&uploaded_part))?;
                                Ok(Some(uploaded_part))
                            }
                            Err(err) => {
                                self.after_part_uploaded(&progresses_key, total_size, None).ok();
                                Err(err)
                            }
                        }
                    }
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        });

        async fn _could_recover<H: Digest>(
            initialized: &MultiPartsV2UploaderInitializedObject<Box<dyn AsyncDataSource<H>>>,
            data_reader: &mut AsyncDataSourceReader,
            part_size: NonZeroU64,
            uploaded_part_ttl: Duration,
        ) -> Option<MultiPartsV2UploaderUploadedPart> {
            let offset = data_reader.offset();
            OptionFuture::from(initialized.recovered_records.take(offset).map(|record| async move {
                if record.size == part_size
                    && record.part_number == data_reader.part_number()
                    && initialized.initialized_at + uploaded_part_ttl > SystemTime::now()
                    && Some(record.base64ed_sha1.as_str()) == sha1_of_async_reader(data_reader).await.ok().as_deref()
                {
                    Some(MultiPartsV2UploaderUploadedPart {
                        response_body: record.response_body.to_owned(),
                        uploaded_size: record.size,
                        part_number: record.part_number,
                        resumed: true,
                        offset,
                    })
                } else {
                    None
                }
            }))
            .await
            .flatten()
        }

        async fn _upload_part<'a, H: Digest, E: EndpointsProvider + Clone + 'a>(
            uploader: &'a MultiPartsV2Uploader<H>,
            mut request: AsyncUploadPartRequestBuilder<'a, E>,
            mut body: AsyncDataSourceReader,
            content_length: NonZeroU64,
            initialized: &'a MultiPartsV2UploaderInitializedObject<Box<dyn AsyncDataSource<H>>>,
            progresses_key: &'a ProgressesKey,
            data_partitioner_provider: &'a dyn DataPartitionProvider,
        ) -> ApiResult<MultiPartsV2UploaderUploadedPart> {
            let total_size = initialized.source.total_size().await?;
            request.on_uploading_progress(move |_, transfer| {
                progresses_key.update_part(transfer.transferred_bytes());
                uploader.callbacks.upload_progress(&UploadingProgressInfo::new(
                    progresses_key.current_uploaded(),
                    total_size,
                ))
            });
            uploader.before_request_call(request.parts_mut())?;
            let body_offset = body.offset();
            let part_number = body.part_number();
            let base64ed_sha1 = sha1_of_async_reader(&mut body).await?;
            let begin_at = Instant::now();
            let mut response_result = request.call(body, content_length.get()).await;
            let elapsed = begin_at.elapsed();
            uploader.after_response_call(&mut response_result)?;
            let mut feedback_builder =
                DataPartitionProviderFeedback::builder(content_length.into(), elapsed, initialized.params.extensions());
            if let Some(err) = response_result.as_ref().err() {
                feedback_builder.error(err);
            }
            data_partitioner_provider.feedback(feedback_builder.build());
            let response_body = response_result?.into_body();
            let record = MultiPartsV2ResumableRecorderRecord {
                response_body,
                offset: body_offset,
                size: content_length,
                base64ed_sha1,
                part_number,
            };
            initialized
                .recovered_records
                .async_persist(
                    &initialized.upload_id,
                    initialized.initialized_at,
                    &record,
                    &uploader.async_bucket_name().await?,
                    initialized.params.object_name(),
                    &initialized.up_endpoints,
                )
                .await
                .ok();
            Ok(MultiPartsV2UploaderUploadedPart::from_record(record, false))
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_complete_parts<'r>(
        &'r self,
        initialized: &'r Self::AsyncInitializedParts,
        parts: &'r [Self::AsyncUploadedPart],
    ) -> BoxFuture<'r, ApiResult<Value>> {
        return Box::pin(async move {
            let upload_token_signer = self.make_upload_token_signer(initialized.params.object_name().map(|n| n.into()));
            let path_params = make_complete_parts_path_params_from_initialized_params(
                self.bucket_name()?.to_string(),
                &initialized.params,
                initialized.upload_id.to_string(),
            );
            let body = make_complete_parts_request_body_from_initialized_params(&initialized.params, parts.to_vec());
            let complete_parts = self.storage().resumable_upload_v2_complete_multipart_upload();
            _complete_parts(
                self,
                complete_parts.new_async_request(&initialized.up_endpoints, path_params, upload_token_signer.as_ref()),
                &initialized.source,
                body,
            )
            .await
        });

        async fn _complete_parts<'a, H: Digest, E: EndpointsProvider + Clone + 'a, D: AsyncDataSource<H>>(
            uploader: &'a MultiPartsV2Uploader<H>,
            mut request: AsyncCompletePartsRequestBuilder<'a, E>,
            source: &D,
            body: CompletePartsRequestBody,
        ) -> ApiResult<Value> {
            uploader.before_request_call(request.parts_mut())?;
            let mut response_result = request.call(&body).await;
            uploader.after_response_call(&mut response_result)?;
            let body = response_result?.into_body();
            uploader.try_to_async_delete_records(&source).await.ok();
            Ok(body.into())
        }
    }
}

fn make_init_parts_path_params_from_initialized_params(
    bucket_name: String,
    params: &ObjectParams,
) -> InitPartsPathParams {
    let mut path_params = InitPartsPathParams::default().set_bucket_name_as_str(bucket_name);
    if let Some(object_name) = params.object_name() {
        path_params = path_params.set_object_name_as_str(object_name.to_owned());
    }
    path_params
}

fn make_upload_part_path_params_from_initialized_params(
    bucket_name: String,
    params: &ObjectParams,
    upload_id: String,
    part_number: NonZeroUsize,
) -> UploadPartPathParams {
    let mut path_params = UploadPartPathParams::default()
        .set_bucket_name_as_str(bucket_name)
        .set_upload_id_as_str(upload_id)
        .set_part_number_as_usize(part_number.get());
    if let Some(object_name) = params.object_name() {
        path_params = path_params.set_object_name_as_str(object_name.to_owned());
    }
    path_params
}

fn make_complete_parts_path_params_from_initialized_params(
    bucket_name: String,
    params: &ObjectParams,
    upload_id: String,
) -> CompletePartsPathParams {
    let mut path_params = CompletePartsPathParams::default()
        .set_bucket_name_as_str(bucket_name)
        .set_upload_id_as_str(upload_id);
    if let Some(object_name) = params.object_name() {
        path_params = path_params.set_object_name_as_str(object_name.to_owned());
    }
    path_params
}

fn make_complete_parts_request_body_from_initialized_params(
    params: &ObjectParams,
    mut parts: Vec<MultiPartsV2UploaderUploadedPart>,
) -> CompletePartsRequestBody {
    parts.sort_by_key(|part| part.part_number);
    let mut body = CompletePartsRequestBody::default();
    body.set_parts(
        parts
            .iter()
            .map(|part| {
                let mut part_info = PartInfo::default();
                part_info.set_etag_as_str(part.response_body.get_etag_as_str().to_owned());
                part_info.set_part_number_as_u64(part.part_number.get() as u64);
                part_info
            })
            .collect::<Vec<_>>()
            .into(),
    );
    if let Some(file_name) = params.file_name() {
        body.set_file_name_as_str(file_name.to_string());
    }
    if let Some(mime) = params.content_type() {
        body.set_mime_type_as_str(mime.to_string());
    }
    body.set_metadata(StringMap::from(
        params
            .metadata()
            .iter()
            .map(|(key, value)| ("x-qn-meta-".to_owned() + key, value.to_owned())),
    ));
    body.set_custom_vars(StringMap::from(
        params
            .custom_vars()
            .iter()
            .map(|(key, value)| ("x:".to_owned() + key, value.to_owned())),
    ));
    body
}

fn sha1_of_sync_reader<R: Read + Reset>(reader: &mut R) -> IoResult<String> {
    Ok(urlsafe_base64(Digestible::<Sha1>::digest(reader)?.as_slice()))
}

#[cfg(feature = "async")]
async fn sha1_of_async_reader<R: AsyncRead + AsyncReset + Unpin + Send>(reader: &mut R) -> IoResult<String> {
    Ok(urlsafe_base64(
        AsyncDigestible::<Sha1>::digest(reader).await?.as_slice(),
    ))
}

impl<H: Digest> MultiPartsV2Uploader<H> {
    fn storage(&self) -> storage::Client {
        self.upload_manager.client().storage()
    }

    fn access_key(&self) -> ApiResult<AccessKey> {
        self.upload_manager.upload_token().access_key()
    }

    fn bucket_name(&self) -> ApiResult<BucketName> {
        self.upload_manager.upload_token().bucket_name()
    }

    #[cfg(feature = "async")]
    async fn async_access_key(&self) -> ApiResult<AccessKey> {
        self.upload_manager.upload_token().async_access_key().await
    }

    #[cfg(feature = "async")]
    async fn async_bucket_name(&self) -> ApiResult<BucketName> {
        self.upload_manager.upload_token().async_bucket_name().await
    }

    fn before_request_call(&self, request: &mut RequestBuilderParts<'_>) -> ApiResult<()> {
        self.callbacks.before_request(request).map_err(make_callback_error)
    }

    fn after_response_call<B>(&self, response: &mut ApiResult<Response<B>>) -> ApiResult<()> {
        self.callbacks.after_response(response).map_err(make_callback_error)
    }

    fn after_part_uploaded(
        &self,
        progresses_key: &ProgressesKey,
        total_size: Option<u64>,
        uploaded_part: Option<&MultiPartsV2UploaderUploadedPart>,
    ) -> ApiResult<()> {
        if let Some(uploaded_part) = uploaded_part {
            progresses_key.complete_part();
            self.callbacks
                .part_uploaded(uploaded_part)
                .map_err(make_callback_error)?;
        } else {
            progresses_key.delete_part();
        }
        self.callbacks
            .upload_progress(&UploadingProgressInfo::new(
                progresses_key.current_uploaded(),
                total_size,
            ))
            .map_err(make_callback_error)
    }
}

impl<H: Digest> MultiPartsV2Uploader<H> {
    fn up_endpoints(&self, params: &ObjectParams) -> ApiResult<UpEndpoints> {
        let up_endpoints = if let Some(region_provider) = params.region_provider() {
            UpEndpoints::from_endpoints_provider(&RegionsProviderEndpoints::new(region_provider))?
        } else {
            UpEndpoints::from_endpoints_provider(&RegionsProviderEndpoints::new(self.get_bucket_region()?))?
        };
        Ok(up_endpoints)
    }

    #[cfg(feature = "async")]
    async fn async_up_endpoints(&self, params: &ObjectParams) -> ApiResult<UpEndpoints> {
        let up_endpoints = if let Some(region_provider) = params.region_provider() {
            UpEndpoints::async_from_endpoints_provider(&RegionsProviderEndpoints::new(region_provider)).await?
        } else {
            UpEndpoints::async_from_endpoints_provider(&RegionsProviderEndpoints::new(
                self.async_get_bucket_region().await?,
            ))
            .await?
        };
        Ok(up_endpoints)
    }

    fn get_bucket_region(&self) -> ApiResult<BucketRegionsProvider> {
        Ok(self
            .upload_manager
            .queryer()
            .query(self.access_key()?, self.bucket_name()?))
    }

    #[cfg(feature = "async")]
    async fn async_get_bucket_region(&self) -> ApiResult<BucketRegionsProvider> {
        Ok(self
            .upload_manager
            .queryer()
            .query(self.async_access_key().await?, self.async_bucket_name().await?))
    }

    fn make_upload_token_signer(&self, object_name: Option<ObjectName>) -> OwnedUploadTokenProviderOrReferenced<'_> {
        self.upload_manager
            .upload_token()
            .make_upload_token_provider(object_name)
    }

    fn try_to_recover<D: DataSource<H>>(
        &self,
        source: &D,
        params: &ObjectParams,
        up_endpoints: &UpEndpoints,
    ) -> ApiResult<(Option<String>, MultiPartsV2ResumableRecorderRecords)> {
        return source
            .source_key()?
            .map(|source_key| {
                _try_to_recover(self, &source_key, params, up_endpoints)
                    .ok()
                    .flatten()
                    .map(|(upload_id, records)| Ok((Some(upload_id), records)))
                    .unwrap_or_else(|| {
                        _new_records(&self.resumable_recorder, &source_key).map(|records| (None, records))
                    })
            })
            .unwrap_or_else(|| Ok(Default::default()));

        fn _try_to_recover<H: Digest>(
            uploader: &MultiPartsV2Uploader<H>,
            source_key: &SourceKey<H>,
            params: &ObjectParams,
            up_endpoints: &UpEndpoints,
        ) -> ApiResult<Option<(String, MultiPartsV2ResumableRecorderRecords)>> {
            let (upload_id, mut records) = {
                let mut medium = uploader.resumable_recorder.open_for_read(source_key)?;
                let mut lines = BufReader::new(&mut medium).lines();
                let upload_id = if let Some(line) = lines.next() {
                    let line = line?;
                    let header: MultiPartsV2ResumableRecorderDeserializableHeader = serde_json::from_str(&line)?;
                    if !header.is_v2()
                        || header.initialized_at() + params.uploaded_part_ttl() <= SystemTime::now()
                        || header.bucket() != uploader.bucket_name()?.as_str()
                        || header.object() != params.object_name()
                        || !up_endpoints.any_intersection(header.up_endpoints())
                    {
                        return Ok(None);
                    }
                    header.upload_id
                } else {
                    return Ok(None);
                };
                let records = lines
                    .map(|line| {
                        let line = line?;
                        let record: MultiPartsV2ResumableRecorderRecord = serde_json::from_str(&line)?;
                        Ok(record)
                    })
                    .collect::<ApiResult<MultiPartsV2ResumableRecorderRecords>>()?;
                (upload_id, records)
            };
            records.set_medium_for_append(uploader.resumable_recorder.open_for_append(source_key)?, true);
            Ok(Some((upload_id, records)))
        }

        fn _new_records<H: Digest>(
            resumable_recorder: &dyn ResumableRecorder<HashAlgorithm = H>,
            source_key: &SourceKey<H>,
        ) -> ApiResult<MultiPartsV2ResumableRecorderRecords> {
            let mut records = MultiPartsV2ResumableRecorderRecords::default();
            records.set_medium_for_append(resumable_recorder.open_for_create_new(source_key)?, false);
            Ok(records)
        }
    }

    #[cfg(feature = "async")]
    async fn try_to_async_recover<D: AsyncDataSource<H>>(
        &self,
        source: &D,
        params: &ObjectParams,
        up_endpoints: &UpEndpoints,
    ) -> ApiResult<(Option<String>, MultiPartsV2ResumableRecorderRecords)> {
        return OptionFuture::from(source.source_key().await?.map(|source_key| async move {
            if let Some((upload_id, records)) = _try_to_recover(self, &source_key, params, up_endpoints)
                .await
                .ok()
                .flatten()
            {
                Ok((Some(upload_id), records))
            } else {
                _new_records(&self.resumable_recorder, &source_key)
                    .await
                    .map(|records| (None, records))
            }
        }))
        .await
        .unwrap_or_else(|| Ok(Default::default()));

        async fn _try_to_recover<H: Digest>(
            uploader: &MultiPartsV2Uploader<H>,
            source_key: &SourceKey<H>,
            params: &ObjectParams,
            up_endpoints: &UpEndpoints,
        ) -> ApiResult<Option<(String, MultiPartsV2ResumableRecorderRecords)>> {
            let (upload_id, mut records) = {
                let mut medium = uploader.resumable_recorder.open_for_async_read(source_key).await?;
                let mut lines = AsyncBufReader::new(&mut medium).lines();
                let upload_id = if let Some(line) = lines.try_next().await? {
                    let header: MultiPartsV2ResumableRecorderDeserializableHeader = serde_json::from_str(&line)?;
                    if !header.is_v2()
                        || header.initialized_at() + params.uploaded_part_ttl() <= SystemTime::now()
                        || header.bucket() != uploader.bucket_name()?.as_str()
                        || header.object() != params.object_name()
                        || !up_endpoints.any_intersection(header.up_endpoints())
                    {
                        return Ok(None);
                    }
                    header.upload_id
                } else {
                    return Ok(None);
                };
                let records = lines
                    .map(|line| {
                        let line = line?;
                        let record: MultiPartsV2ResumableRecorderRecord = serde_json::from_str(&line)?;
                        Ok::<_, ResponseError>(record)
                    })
                    .try_collect::<MultiPartsV2ResumableRecorderRecords>()
                    .await?;
                (upload_id, records)
            };
            records.set_medium_for_async_append(
                uploader.resumable_recorder.open_for_async_append(source_key).await?,
                true,
            );
            Ok(Some((upload_id, records)))
        }

        async fn _new_records<H: Digest>(
            resumable_recorder: &dyn ResumableRecorder<HashAlgorithm = H>,
            source_key: &SourceKey<H>,
        ) -> ApiResult<MultiPartsV2ResumableRecorderRecords> {
            let mut records = MultiPartsV2ResumableRecorderRecords::default();
            records.set_medium_for_async_append(resumable_recorder.open_for_async_create_new(source_key).await?, false);
            Ok(records)
        }
    }

    fn try_to_delete_records<D: DataSource<H>>(&self, source: &D) -> ApiResult<()> {
        if let Some(source_key) = source.source_key()? {
            self.resumable_recorder.delete(&source_key)?;
        }
        Ok(())
    }

    #[cfg(feature = "async")]
    async fn try_to_async_delete_records<D: AsyncDataSource<H>>(&self, source: &D) -> ApiResult<()> {
        if let Some(source_key) = source.source_key().await? {
            self.resumable_recorder.async_delete(&source_key).await?;
        }
        Ok(())
    }
}

impl<H: Digest> Debug for MultiPartsV2Uploader<H> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultiPartsV2Uploader")
            .field("upload_manager", &self.upload_manager)
            .field("callbacks", &self.callbacks)
            .field("resumable_recorder", &self.resumable_recorder)
            .finish()
    }
}

impl<H: Digest> Clone for MultiPartsV2Uploader<H> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            upload_manager: self.upload_manager.clone(),
            callbacks: self.callbacks.clone(),
            resumable_recorder: self.resumable_recorder.clone(),
        }
    }
}

#[allow(unsafe_code)]
const MIN_PART_SIZE: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(1 << 20) };

#[allow(unsafe_code)]
const MAX_PART_SIZE: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(1 << 30) };

#[derive(Debug, Clone, Deserialize)]
struct MultiPartsV2ResumableRecorderDeserializableHeader {
    #[serde(rename = "ver")]
    version: u8,

    #[serde(rename = "uid")]
    upload_id: String,

    #[serde(rename = "init")]
    initialized_timestamp: u64,

    #[serde(rename = "bkt")]
    bucket: BucketName,

    #[serde(rename = "key")]
    object: Option<ObjectName>,

    #[serde(rename = "ups")]
    up_endpoints: Vec<Endpoint>,
}

impl MultiPartsV2ResumableRecorderDeserializableHeader {
    fn is_v2(&self) -> bool {
        self.version == 2
    }

    fn initialized_at(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.initialized_timestamp)
    }

    fn bucket(&self) -> &str {
        &self.bucket
    }

    fn object(&self) -> Option<&str> {
        self.object.as_deref()
    }

    fn up_endpoints(&self) -> &[Endpoint] {
        &self.up_endpoints
    }
}

#[derive(Debug, Clone, Serialize)]
struct MultiPartsV2ResumableRecorderSerializableHeader<'a> {
    #[serde(rename = "ver")]
    version: u8,

    #[serde(rename = "uid")]
    upload_id: &'a str,

    #[serde(rename = "init")]
    initialized_timestamp: u64,

    #[serde(rename = "bkt")]
    bucket: &'a str,

    #[serde(rename = "key")]
    object: Option<&'a str>,

    #[serde(rename = "ups")]
    up_endpoints: &'a [Endpoint],
}

impl<'a> MultiPartsV2ResumableRecorderSerializableHeader<'a> {
    fn v2(
        upload_id: &'a str,
        initialized_at: SystemTime,
        bucket: &'a str,
        object: Option<&'a str>,
        up_endpoints: &'a [Endpoint],
    ) -> Self {
        Self {
            upload_id,
            bucket,
            object,
            up_endpoints,
            version: 2,
            initialized_timestamp: initialized_at.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MultiPartsV2ResumableRecorderRecord {
    #[serde(rename = "off")]
    offset: u64,
    #[serde(rename = "size")]
    size: NonZeroU64,
    #[serde(rename = "body")]
    response_body: UploadPartResponseBody,
    #[serde(rename = "pnum")]
    part_number: NonZeroUsize,
    #[serde(rename = "sha1")]
    base64ed_sha1: String,
}

#[derive(Debug)]
struct AppendOnlyMediumForMultiPartsV2ResumableRecorderRecords {
    medium: Box<dyn AppendOnlyResumableRecorderMedium>,
    header_written: bool,
}

#[cfg(feature = "async")]
#[derive(Debug)]
struct AsyncAppendOnlyMediumForMultiPartsV2ResumableRecorderRecords {
    medium: Box<dyn AppendOnlyAsyncResumableRecorderMedium>,
    header_written: bool,
}

#[derive(Debug, Default)]
struct MultiPartsV2ResumableRecorderRecords {
    map: DashMap<u64, MultiPartsV2ResumableRecorderRecord>,
    append_only_medium: Option<Mutex<AppendOnlyMediumForMultiPartsV2ResumableRecorderRecords>>,

    #[cfg(feature = "async")]
    async_append_only_medium: Option<AsyncMutex<AsyncAppendOnlyMediumForMultiPartsV2ResumableRecorderRecords>>,
}

impl MultiPartsV2ResumableRecorderRecords {
    fn set_medium_for_append(&mut self, medium: Box<dyn AppendOnlyResumableRecorderMedium>, header_written: bool) {
        self.append_only_medium = Some(Mutex::new(AppendOnlyMediumForMultiPartsV2ResumableRecorderRecords {
            medium,
            header_written,
        }));
    }

    #[cfg(feature = "async")]
    fn set_medium_for_async_append(
        &mut self,
        medium: Box<dyn AppendOnlyAsyncResumableRecorderMedium>,
        header_written: bool,
    ) {
        self.async_append_only_medium = Some(AsyncMutex::new(
            AsyncAppendOnlyMediumForMultiPartsV2ResumableRecorderRecords { medium, header_written },
        ));
    }

    fn take(&self, offset: u64) -> Option<MultiPartsV2ResumableRecorderRecord> {
        self.map.remove(&offset).map(|(_, record)| record)
    }

    fn persist(
        &self,
        upload_id: &str,
        initialized_at: SystemTime,
        record: &MultiPartsV2ResumableRecorderRecord,
        bucket_name: &str,
        object_name: Option<&str>,
        up_endpoints: &UpEndpoints,
    ) -> ApiResult<()> {
        if let Some(append_only_medium) = self.append_only_medium.as_ref() {
            let mut buf = Vec::new();
            let mut append_only_medium = append_only_medium.lock().unwrap();
            if !append_only_medium.header_written {
                serde_json::to_writer(
                    &mut buf,
                    &MultiPartsV2ResumableRecorderSerializableHeader::v2(
                        upload_id,
                        initialized_at,
                        bucket_name,
                        object_name,
                        up_endpoints.as_slice(),
                    ),
                )?;
                buf.extend_from_slice(b"\n");
            }
            serde_json::to_writer(&mut buf, &record)?;
            buf.extend_from_slice(b"\n");
            append_only_medium.medium.write_all(&buf)?;
            append_only_medium.medium.flush()?;
            append_only_medium.header_written = true;
        }
        Ok(())
    }

    #[cfg(feature = "async")]
    async fn async_persist(
        &self,
        upload_id: &str,
        initialized_at: SystemTime,
        record: &MultiPartsV2ResumableRecorderRecord,
        bucket_name: &str,
        object_name: Option<&str>,
        up_endpoints: &UpEndpoints,
    ) -> ApiResult<()> {
        if let Some(append_only_medium) = self.async_append_only_medium.as_ref() {
            let mut append_only_medium = append_only_medium.lock().await;
            let mut buf = Vec::new();
            if !append_only_medium.header_written {
                serde_json::to_writer(
                    &mut buf,
                    &MultiPartsV2ResumableRecorderSerializableHeader::v2(
                        upload_id,
                        initialized_at,
                        bucket_name,
                        object_name,
                        up_endpoints.as_slice(),
                    ),
                )?;
                buf.extend_from_slice(b"\n");
            }
            serde_json::to_writer(&mut buf, &record)?;
            buf.extend_from_slice(b"\n");
            append_only_medium.medium.write_all(&buf).await?;
            append_only_medium.medium.flush().await?;
            append_only_medium.header_written = true;
        }
        Ok(())
    }
}

impl FromIterator<MultiPartsV2ResumableRecorderRecord> for MultiPartsV2ResumableRecorderRecords {
    fn from_iter<T: IntoIterator<Item = MultiPartsV2ResumableRecorderRecord>>(iter: T) -> Self {
        Self {
            map: DashMap::from_iter(iter.into_iter().map(|record| (record.offset, record))),
            append_only_medium: None,

            #[cfg(feature = "async")]
            async_append_only_medium: None,
        }
    }
}

impl Extend<MultiPartsV2ResumableRecorderRecord> for MultiPartsV2ResumableRecorderRecords {
    fn extend<T: IntoIterator<Item = MultiPartsV2ResumableRecorderRecord>>(&mut self, iter: T) {
        self.map.extend(iter.into_iter().map(|record| (record.offset, record)))
    }
}

impl MultiPartsV2UploaderUploadedPart {
    fn from_record(record: MultiPartsV2ResumableRecorderRecord, resumed: bool) -> Self {
        Self {
            response_body: record.response_body,
            uploaded_size: record.size,
            offset: record.offset,
            part_number: record.part_number,
            resumed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{data_source::FileDataSource, DummyResumableRecorder, FixedDataPartitionProvider, UploadTokenSigner};
    use anyhow::Result;
    use qiniu_apis::{
        credential::Credential,
        http::{HeaderValue, HttpCaller, StatusCode, SyncRequest, SyncResponse, SyncResponseBody, SyncResponseResult},
        http_client::{DirectChooser, HttpClient, NeverRetrier, Region, NO_BACKOFF},
    };
    use rand::{thread_rng, RngCore};
    use serde_json::{json, to_vec as json_to_vec};
    use sha1::Sha1;
    use std::{
        io::{copy as io_copy, sink as io_sink, Read},
        sync::atomic::{AtomicUsize, Ordering},
        thread::spawn as spawn_thread,
        time::Duration,
    };
    use tempfile::{Builder as TempfileBuilder, TempPath};
    use text_io::scan as scan_text;

    #[cfg(feature = "async")]
    use {
        crate::data_source::AsyncFileDataSource,
        async_std::task::spawn as spawn_task,
        futures::{
            future::join_all,
            io::{copy as async_io_copy, sink as async_io_sink},
        },
        qiniu_apis::http::{AsyncRequest, AsyncResponse, AsyncResponseBody, AsyncResponseResult},
    };

    const FILE_SIZE: u64 = 104885287;
    const BLOCK_SIZE: u64 = 4 << 20;
    const LAST_BLOCK_SIZE: u64 = FILE_SIZE - FILE_SIZE / BLOCK_SIZE * BLOCK_SIZE;
    const BLOCK_COUNT: usize = ((FILE_SIZE + BLOCK_SIZE - 1) / BLOCK_SIZE) as usize;

    #[test]
    fn test_sync_multi_parts_v2_upload() -> Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            init_parts_counts: AtomicUsize,
            upload_part_counts: AtomicUsize,
            complete_parts_counts: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                let resp_body = if request.url().path() == "/buckets/fakebucket/objects/~/uploads" {
                    assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 0);
                    assert_eq!(self.init_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                    assert_eq!(self.complete_parts_counts.load(Ordering::Relaxed), 0);
                    json_to_vec(&json!({
                        "uploadId": "fakeuploadid",
                    }))
                    .unwrap()
                } else if request
                    .url()
                    .path()
                    .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid/")
                {
                    let page_number: usize;
                    scan_text!(request.url().path().bytes() => "/buckets/fakebucket/objects/~/uploads/fakeuploadid/{}", page_number);
                    let body_len = size_of_sync_reader(request.body_mut()).unwrap();
                    assert_eq!(
                        body_len,
                        if page_number == BLOCK_COUNT {
                            LAST_BLOCK_SIZE
                        } else {
                            BLOCK_SIZE
                        }
                    );
                    self.upload_part_counts.fetch_add(1, Ordering::Relaxed);
                    json_to_vec(&json!({
                        "etag": format!("==={}===", page_number),
                        "md5": "fake-md5",
                    }))
                    .unwrap()
                } else if request.url().path() == "/buckets/fakebucket/objects/~/uploads/fakeuploadid" {
                    assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), BLOCK_COUNT);
                    assert_eq!(self.complete_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                    let body: CompletePartsRequestBody = serde_json::from_reader(request.body_mut()).unwrap();
                    body.get_parts()
                        .to_part_info_vec()
                        .into_iter()
                        .fold(None, |last_page_number, part_info| {
                            if let Some(last_page_number) = last_page_number {
                                assert_eq!(part_info.get_part_number_as_u64(), last_page_number + 1);
                                assert_eq!(
                                    part_info.get_etag_as_str(),
                                    &format!("==={}===", part_info.get_part_number_as_u64()),
                                );
                            }
                            Some(part_info.get_part_number_as_u64())
                        });
                    json_to_vec(&json!({
                        "done": 1,
                    }))
                    .unwrap()
                } else {
                    unreachable!()
                };
                Ok(SyncResponse::builder()
                    .status_code(StatusCode::OK)
                    .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                    .body(SyncResponseBody::from_bytes(resp_body))
                    .build())
            }

            #[cfg(feature = "async")]
            fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let uploader = Arc::new(MultiPartsV2Uploader::new(
            get_upload_manager(FakeHttpCaller::default()),
            DummyResumableRecorder::<Sha1>::new(),
        ));
        let file_path = random_file_path(FILE_SIZE)?;
        let file_source = FileDataSource::new(file_path.as_os_str());
        let params = ObjectParams::builder()
            .region_provider(single_up_domain_region())
            .build();
        let initialized_parts = Arc::new(uploader.initialize_parts(file_source, params)?);

        #[allow(clippy::needless_collect)]
        let threads = (0..BLOCK_COUNT)
            .map(|_| {
                let uploader = uploader.to_owned();
                let initialized_parts = initialized_parts.to_owned();
                spawn_thread(move || {
                    uploader.upload_part(&initialized_parts, &new_data_partitioner_provider(BLOCK_SIZE))
                })
            })
            .collect::<Vec<_>>();
        let parts = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<ApiResult<Vec<_>>>()?;
        let parts = parts.into_iter().map(|part| part.unwrap()).collect::<Vec<_>>();
        let body = uploader.complete_parts(&initialized_parts, &parts)?;
        assert_eq!(body.get("done").unwrap().as_u64().unwrap(), 1u64);
        Ok(())
    }

    #[cfg(feature = "async")]
    #[async_std::test]
    async fn test_async_multi_parts_v2_upload() -> Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            init_parts_counts: AtomicUsize,
            upload_part_counts: AtomicUsize,
            complete_parts_counts: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            #[cfg(feature = "async")]
            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    let resp_body = if request.url().path() == "/buckets/fakebucket/objects/~/uploads" {
                        assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 0);
                        assert_eq!(self.init_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                        assert_eq!(self.complete_parts_counts.load(Ordering::Relaxed), 0);
                        json_to_vec(&json!({
                            "uploadId": "fakeuploadid",
                        }))
                        .unwrap()
                    } else if request
                        .url()
                        .path()
                        .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid/")
                    {
                        let page_number: usize;
                        scan_text!(request.url().path().bytes() => "/buckets/fakebucket/objects/~/uploads/fakeuploadid/{}", page_number);
                        let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                        assert_eq!(
                            body_len,
                            if page_number == BLOCK_COUNT {
                                LAST_BLOCK_SIZE
                            } else {
                                BLOCK_SIZE
                            }
                        );
                        self.upload_part_counts.fetch_add(1, Ordering::Relaxed);
                        json_to_vec(&json!({
                            "etag": format!("==={}===", page_number),
                            "md5": "fake-md5",
                        }))
                        .unwrap()
                    } else if request.url().path() == "/buckets/fakebucket/objects/~/uploads/fakeuploadid" {
                        assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), BLOCK_COUNT);
                        assert_eq!(self.complete_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                        let body: CompletePartsRequestBody = {
                            let mut req_body = Vec::new();
                            async_io_copy(request.body_mut(), &mut req_body).await.unwrap();
                            serde_json::from_slice(&req_body).unwrap()
                        };
                        body.get_parts()
                            .to_part_info_vec()
                            .into_iter()
                            .fold(None, |last_page_number, part_info| {
                                if let Some(last_page_number) = last_page_number {
                                    assert_eq!(part_info.get_part_number_as_u64(), last_page_number + 1);
                                    assert_eq!(
                                        part_info.get_etag_as_str(),
                                        &format!("==={}===", part_info.get_part_number_as_u64()),
                                    );
                                }
                                Some(part_info.get_part_number_as_u64())
                            });
                        json_to_vec(&json!({
                            "done": 1,
                        }))
                        .unwrap()
                    } else {
                        unreachable!()
                    };
                    Ok(AsyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(AsyncResponseBody::from_bytes(resp_body))
                        .build())
                })
            }
        }

        let uploader = Arc::new(MultiPartsV2Uploader::new(
            get_upload_manager(FakeHttpCaller::default()),
            DummyResumableRecorder::<Sha1>::new(),
        ));
        let file_path = spawn_task(async { random_file_path(FILE_SIZE) }).await?;
        let file_source = AsyncFileDataSource::new(file_path.as_os_str());
        let params = ObjectParams::builder()
            .region_provider(single_up_domain_region())
            .build();
        let initialized_parts = Arc::new(uploader.async_initialize_parts(file_source, params).await?);

        let tasks = (0..BLOCK_COUNT).map(|_| {
            let uploader = uploader.to_owned();
            let initialized_parts = initialized_parts.to_owned();
            spawn_task(async move {
                uploader
                    .async_upload_part(&initialized_parts, &new_data_partitioner_provider(BLOCK_SIZE))
                    .await
            })
        });
        let parts = join_all(tasks).await.into_iter().collect::<ApiResult<Vec<_>>>()?;
        let parts = parts.into_iter().map(|part| part.unwrap()).collect::<Vec<_>>();
        let body = uploader.async_complete_parts(&initialized_parts, &parts).await?;
        assert_eq!(body.get("done").unwrap().as_u64().unwrap(), 1u64);
        Ok(())
    }

    mod with_recovery {
        use super::*;
        use crate::FileSystemResumableRecorder;
        use std::fs::read_dir;

        const FILE_SIZE: u64 = 11550954;
        const BLOCK_SIZE: u64 = 4 << 20;
        const LAST_BLOCK_SIZE: u64 = FILE_SIZE - FILE_SIZE / BLOCK_SIZE * BLOCK_SIZE;
        const BLOCK_COUNT: usize = ((FILE_SIZE + BLOCK_SIZE - 1) / BLOCK_SIZE) as usize;

        #[test]
        fn test_sync_multi_parts_v2_upload_with_recovery() -> Result<()> {
            env_logger::builder().is_test(true).try_init().ok();

            #[derive(Debug)]
            struct FakeHttpCaller {
                allow_init_parts: bool,
                part_number_assertion: usize,
                init_parts_counts: AtomicUsize,
                upload_part_counts: AtomicUsize,
            }

            impl FakeHttpCaller {
                fn new(allow_init_parts: bool, part_number_assertion: usize) -> Self {
                    Self {
                        allow_init_parts,
                        part_number_assertion,
                        init_parts_counts: Default::default(),
                        upload_part_counts: Default::default(),
                    }
                }
            }

            impl HttpCaller for FakeHttpCaller {
                fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                    let resp_body = if self.allow_init_parts
                        && request.url().path() == "/buckets/fakebucket/objects/~/uploads"
                    {
                        assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 0);
                        assert_eq!(self.init_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                        json_to_vec(&json!({
                            "uploadId": "fakeuploadid",
                        }))
                        .unwrap()
                    } else if request
                        .url()
                        .path()
                        .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid/")
                    {
                        let page_number: usize;
                        scan_text!(request.url().path().bytes() => "/buckets/fakebucket/objects/~/uploads/fakeuploadid/{}", page_number);
                        assert_eq!(page_number, self.part_number_assertion);
                        let body_len = size_of_sync_reader(request.body_mut()).unwrap();
                        assert_eq!(body_len, BLOCK_SIZE);
                        assert_eq!(self.upload_part_counts.fetch_add(1, Ordering::Relaxed), 0);
                        json_to_vec(&json!({
                            "etag": format!("==={}===", page_number),
                            "md5": "fake-md5",
                        }))
                        .unwrap()
                    } else {
                        unreachable!()
                    };
                    Ok(SyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(SyncResponseBody::from_bytes(resp_body))
                        .build())
                }

                #[cfg(feature = "async")]
                fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                    unreachable!()
                }
            }

            let resuming_files_dir = TempfileBuilder::new().tempdir()?;
            let file_path = random_file_path(FILE_SIZE)?;
            {
                let uploader = MultiPartsV2Uploader::new(
                    get_upload_manager(FakeHttpCaller::new(true, 1)),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                );
                let file_source = FileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let initialized_parts = uploader.initialize_parts(file_source, params)?;
                uploader
                    .upload_part(&initialized_parts, &new_data_partitioner_provider(BLOCK_SIZE))?
                    .unwrap();
            }
            {
                let uploader = MultiPartsV2Uploader::new(
                    get_upload_manager(FakeHttpCaller::new(false, 2)),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                );
                let file_source = FileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let initialized_parts = uploader.initialize_parts(file_source, params)?;
                for _ in 0..2 {
                    uploader
                        .upload_part(&initialized_parts, &new_data_partitioner_provider(BLOCK_SIZE))?
                        .unwrap();
                }
            }

            #[derive(Debug, Default)]
            struct FakeHttpCaller2 {
                part_number_assertion: usize,
                upload_part_counts: AtomicUsize,
                complete_parts_counts: AtomicUsize,
            }

            impl FakeHttpCaller2 {
                fn new(part_number_assertion: usize) -> Self {
                    Self {
                        part_number_assertion,
                        upload_part_counts: Default::default(),
                        complete_parts_counts: Default::default(),
                    }
                }
            }

            impl HttpCaller for FakeHttpCaller2 {
                fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                    let resp_body = if request
                        .url()
                        .path()
                        .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid/")
                    {
                        let page_number: usize;
                        scan_text!(request.url().path().bytes() => "/buckets/fakebucket/objects/~/uploads/fakeuploadid/{}", page_number);
                        assert_eq!(page_number, self.part_number_assertion);
                        let body_len = size_of_sync_reader(request.body_mut()).unwrap();
                        assert_eq!(body_len, LAST_BLOCK_SIZE);
                        assert_eq!(self.upload_part_counts.fetch_add(1, Ordering::Relaxed), 0);
                        json_to_vec(&json!({
                            "etag": format!("==={}===", page_number),
                            "md5": "fake-md5",
                        }))
                        .unwrap()
                    } else if request.url().path() == "/buckets/fakebucket/objects/~/uploads/fakeuploadid" {
                        assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 1);
                        assert_eq!(self.complete_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                        let body: CompletePartsRequestBody = serde_json::from_reader(request.body_mut()).unwrap();
                        body.get_parts()
                            .to_part_info_vec()
                            .into_iter()
                            .fold(None, |last_page_number, part_info| {
                                if let Some(last_page_number) = last_page_number {
                                    assert_eq!(part_info.get_part_number_as_u64(), last_page_number + 1);
                                    assert_eq!(
                                        part_info.get_etag_as_str(),
                                        &format!("==={}===", part_info.get_part_number_as_u64()),
                                    );
                                }
                                Some(part_info.get_part_number_as_u64())
                            });
                        json_to_vec(&json!({
                            "done": 1,
                        }))
                        .unwrap()
                    } else {
                        unreachable!()
                    };
                    Ok(SyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                        .body(SyncResponseBody::from_bytes(resp_body))
                        .build())
                }

                #[cfg(feature = "async")]
                fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                    unreachable!()
                }
            }

            {
                let uploader = Arc::new(MultiPartsV2Uploader::new(
                    get_upload_manager(FakeHttpCaller2::new(3)),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                ));
                let file_source = FileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let initialized_parts = Arc::new(uploader.initialize_parts(file_source, params)?);
                #[allow(clippy::needless_collect)]
                let threads = (0..BLOCK_COUNT)
                    .map(|_| {
                        let uploader = uploader.to_owned();
                        let initialized_parts = initialized_parts.to_owned();
                        spawn_thread(move || {
                            uploader.upload_part(&initialized_parts, &new_data_partitioner_provider(BLOCK_SIZE))
                        })
                    })
                    .collect::<Vec<_>>();
                let parts = threads
                    .into_iter()
                    .map(|thread| thread.join().unwrap())
                    .collect::<ApiResult<Vec<_>>>()?;
                let parts = parts.into_iter().map(|part| part.unwrap()).collect::<Vec<_>>();
                let body = uploader.complete_parts(&initialized_parts, &parts)?;
                assert_eq!(body.get("done").unwrap().as_u64().unwrap(), 1u64);
            }

            assert_eq!(read_dir(resuming_files_dir.path())?.count(), 0);
            Ok(())
        }

        #[cfg(feature = "async")]
        #[async_std::test]
        async fn test_async_multi_parts_v1_upload_with_recovery() -> Result<()> {
            env_logger::builder().is_test(true).try_init().ok();

            #[derive(Debug)]
            struct FakeHttpCaller {
                allow_init_parts: bool,
                part_number_assertion: usize,
                init_parts_counts: AtomicUsize,
                upload_part_counts: AtomicUsize,
            }

            impl FakeHttpCaller {
                fn new(allow_init_parts: bool, part_number_assertion: usize) -> Self {
                    Self {
                        allow_init_parts,
                        part_number_assertion,
                        init_parts_counts: Default::default(),
                        upload_part_counts: Default::default(),
                    }
                }
            }

            impl HttpCaller for FakeHttpCaller {
                fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                    unreachable!()
                }

                #[cfg(feature = "async")]
                fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                    Box::pin(async move {
                        let resp_body = if self.allow_init_parts
                            && request.url().path() == "/buckets/fakebucket/objects/~/uploads"
                        {
                            assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 0);
                            assert_eq!(self.init_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                            json_to_vec(&json!({
                                "uploadId": "fakeuploadid",
                            }))
                            .unwrap()
                        } else if request
                            .url()
                            .path()
                            .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid/")
                        {
                            let page_number: usize;
                            scan_text!(request.url().path().bytes() => "/buckets/fakebucket/objects/~/uploads/fakeuploadid/{}", page_number);
                            assert_eq!(page_number, self.part_number_assertion);
                            let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                            assert_eq!(body_len, BLOCK_SIZE);
                            assert_eq!(self.upload_part_counts.fetch_add(1, Ordering::Relaxed), 0);
                            json_to_vec(&json!({
                                "etag": format!("==={}===", page_number),
                                "md5": "fake-md5",
                            }))
                            .unwrap()
                        } else {
                            unreachable!()
                        };
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    })
                }
            }

            let resuming_files_dir = TempfileBuilder::new().tempdir()?;
            let file_path = spawn_task(async { random_file_path(FILE_SIZE) }).await?;
            {
                let uploader = MultiPartsV2Uploader::new(
                    get_upload_manager(FakeHttpCaller::new(true, 1)),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                );
                let file_source = AsyncFileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let initialized_parts = uploader.async_initialize_parts(file_source, params).await?;
                uploader
                    .async_upload_part(&initialized_parts, &new_data_partitioner_provider(BLOCK_SIZE))
                    .await?
                    .unwrap();
            }
            {
                let uploader = MultiPartsV2Uploader::new(
                    get_upload_manager(FakeHttpCaller::new(false, 2)),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                );
                let file_source = AsyncFileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let initialized_parts = uploader.async_initialize_parts(file_source, params).await?;
                for _ in 0..2 {
                    uploader
                        .async_upload_part(&initialized_parts, &new_data_partitioner_provider(BLOCK_SIZE))
                        .await?
                        .unwrap();
                }
            }

            #[derive(Debug, Default)]
            struct FakeHttpCaller2 {
                part_number_assertion: usize,
                upload_part_counts: AtomicUsize,
                complete_parts_counts: AtomicUsize,
            }

            impl FakeHttpCaller2 {
                fn new(part_number_assertion: usize) -> Self {
                    Self {
                        part_number_assertion,
                        upload_part_counts: Default::default(),
                        complete_parts_counts: Default::default(),
                    }
                }
            }

            impl HttpCaller for FakeHttpCaller2 {
                fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                    unreachable!()
                }

                #[cfg(feature = "async")]
                fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                    Box::pin(async move {
                        let resp_body = if request
                            .url()
                            .path()
                            .starts_with("/buckets/fakebucket/objects/~/uploads/fakeuploadid/")
                        {
                            let page_number: usize;
                            scan_text!(request.url().path().bytes() => "/buckets/fakebucket/objects/~/uploads/fakeuploadid/{}", page_number);
                            assert_eq!(page_number, self.part_number_assertion);
                            let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                            assert_eq!(body_len, LAST_BLOCK_SIZE);
                            assert_eq!(self.upload_part_counts.fetch_add(1, Ordering::Relaxed), 0);
                            json_to_vec(&json!({
                                "etag": format!("==={}===", page_number),
                                "md5": "fake-md5",
                            }))
                            .unwrap()
                        } else if request.url().path() == "/buckets/fakebucket/objects/~/uploads/fakeuploadid" {
                            assert_eq!(self.upload_part_counts.load(Ordering::Relaxed), 1);
                            assert_eq!(self.complete_parts_counts.fetch_add(1, Ordering::Relaxed), 0);
                            let body: CompletePartsRequestBody = {
                                let mut req_body = Vec::new();
                                async_io_copy(request.body_mut(), &mut req_body).await.unwrap();
                                serde_json::from_slice(&req_body).unwrap()
                            };
                            body.get_parts().to_part_info_vec().into_iter().fold(
                                None,
                                |last_page_number, part_info| {
                                    if let Some(last_page_number) = last_page_number {
                                        assert_eq!(part_info.get_part_number_as_u64(), last_page_number + 1);
                                        assert_eq!(
                                            part_info.get_etag_as_str(),
                                            &format!("==={}===", part_info.get_part_number_as_u64()),
                                        );
                                    }
                                    Some(part_info.get_part_number_as_u64())
                                },
                            );
                            json_to_vec(&json!({
                                "done": 1,
                            }))
                            .unwrap()
                        } else {
                            unreachable!()
                        };
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    })
                }
            }

            {
                let uploader = Arc::new(MultiPartsV2Uploader::new(
                    get_upload_manager(FakeHttpCaller2::new(3)),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                ));
                let file_source = AsyncFileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let initialized_parts = Arc::new(uploader.async_initialize_parts(file_source, params).await?);
                let tasks = (0..BLOCK_COUNT).map(|_| {
                    let uploader = uploader.to_owned();
                    let initialized_parts = initialized_parts.to_owned();
                    spawn_task(async move {
                        uploader
                            .async_upload_part(&initialized_parts, &new_data_partitioner_provider(BLOCK_SIZE))
                            .await
                    })
                });
                let parts = join_all(tasks).await.into_iter().collect::<ApiResult<Vec<_>>>()?;
                let parts = parts.into_iter().map(|part| part.unwrap()).collect::<Vec<_>>();
                let body = uploader.async_complete_parts(&initialized_parts, &parts).await?;
                assert_eq!(body.get("done").unwrap().as_u64().unwrap(), 1u64);
            }

            assert!(async_std::fs::read_dir(resuming_files_dir.path())
                .await?
                .next()
                .await
                .is_none());
            Ok(())
        }
    }

    fn get_upload_manager(caller: impl HttpCaller + 'static) -> UploadManager {
        UploadManager::builder(UploadTokenSigner::new_credential_provider(
            get_credential(),
            "fakebucket",
            Duration::from_secs(100),
        ))
        .http_client(
            HttpClient::builder(caller)
                .chooser(DirectChooser)
                .request_retrier(NeverRetrier)
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

    fn random_file_path(size: u64) -> Result<TempPath> {
        let mut tempfile = TempfileBuilder::new().tempfile()?;
        let rng = Box::new(thread_rng()) as Box<dyn RngCore>;
        io_copy(&mut rng.take(size), &mut tempfile)?;
        Ok(tempfile.into_temp_path())
    }

    fn new_data_partitioner_provider(block_size: u64) -> FixedDataPartitionProvider {
        FixedDataPartitionProvider::new(block_size).unwrap()
    }

    fn size_of_sync_reader<R: Read + Reset>(mut reader: &mut R) -> IoResult<u64> {
        let size = io_copy(&mut reader, &mut io_sink())?;
        reader.reset()?;
        Ok(size)
    }

    #[cfg(feature = "async")]
    async fn size_of_async_reader<R: AsyncRead + AsyncReset + Unpin>(mut reader: &mut R) -> IoResult<u64> {
        let size = async_io_copy(&mut reader, &mut async_io_sink()).await?;
        reader.reset().await?;
        Ok(size)
    }
}
