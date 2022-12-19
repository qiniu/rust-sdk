use super::{
    super::{
        callbacks::{Callbacks, UploadingProgressInfo},
        data_source::{Digestible, SourceKey},
        AppendOnlyResumableRecorderMedium, DataPartitionProvider, DataPartitionProviderFeedback, DataSourceReader,
        LimitedDataPartitionProvider, UploaderWithCallbacks,
    },
    progress::{Progresses, ProgressesKey},
    DataSource, InitializedParts, MultiPartsUploader, MultiPartsUploaderExt, MultiPartsUploaderWithCallbacks,
    ObjectParams, PartsExpiredError, ReinitializeOptions, ResumableRecorder, UploadManager, UploadedPart,
};
use anyhow::{Error as AnyError, Result as AnyResult};
use dashmap::DashMap;
use digest::Digest;
use qiniu_apis::{
    http::{Reset, ResponseErrorKind as HttpResponseErrorKind, ResponseParts},
    http_client::{
        ApiResult, Endpoints, EndpointsProvider, RequestBuilderParts, Response, ResponseError, ResponseErrorKind,
    },
    storage::{
        resumable_upload_v1_make_block::{
            PathParams as MkBlkPathParams, ResponseBody as MkBlkResponseBody,
            SyncRequestBuilder as SyncMkBlkRequestBuilder,
        },
        resumable_upload_v1_make_file::{
            PathParams as MkFilePathParams, SyncRequestBuilder as SyncMkFileRequestBuilder,
        },
    },
};
use qiniu_upload_token::BucketName;
use qiniu_utils::base64::urlsafe as urlsafe_base64;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::Sha1;
use std::{
    fmt::{self, Debug},
    io::{BufRead, BufReader, Cursor, Read, Result as IoResult, Write},
    num::NonZeroU64,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "async")]
use {
    super::super::{
        data_source::{AsyncDataSource, AsyncDigestible},
        AppendOnlyAsyncResumableRecorderMedium, AsyncDataSourceReader,
    },
    async_std::io::Cursor as AsyncCursor,
    futures::{
        future::{BoxFuture, OptionFuture},
        io::{AsyncRead, BufReader as AsyncBufReader},
        lock::Mutex as AsyncMutex,
        AsyncBufReadExt, AsyncWriteExt, StreamExt, TryStreamExt,
    },
    qiniu_apis::{
        http::AsyncReset,
        storage::{
            resumable_upload_v1_make_block::AsyncRequestBuilder as AsyncMkBlkRequestBuilder,
            resumable_upload_v1_make_file::AsyncRequestBuilder as AsyncMkFileRequestBuilder,
        },
    },
};

/// 分片上传器 V1
///
/// 不推荐直接使用这个上传器，而是可以借助 [`crate::MultiPartsUploaderScheduler`] 来方便地实现分片上传。
pub struct MultiPartsV1Uploader<H: Digest = Sha1> {
    upload_manager: UploadManager,
    callbacks: Callbacks<'static>,
    resumable_recorder: Arc<dyn ResumableRecorder<HashAlgorithm = H>>,
}

/// 被 分片上传器 V1 初始化的分片信息
#[derive(Clone)]
pub struct MultiPartsV1UploaderInitializedObject<S> {
    source: S,
    params: ObjectParams,
    progresses: Progresses,
    resumed_records: MultiPartsV1ResumableRecorderRecords,
}

impl<S: Clone + Debug + Send + Sync> InitializedParts for MultiPartsV1UploaderInitializedObject<S> {
    #[inline]
    fn params(&self) -> &ObjectParams {
        &self.params
    }

    #[inline]
    fn up_endpoints(&self) -> &Endpoints {
        &self.resumed_records.up_endpoints
    }
}

impl<S> super::__private::Sealed for MultiPartsV1UploaderInitializedObject<S> {}

impl<S: Debug> Debug for MultiPartsV1UploaderInitializedObject<S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultiPartsV1UploaderInitializedObject")
            .field("source", &self.source)
            .field("params", &self.params)
            .field("progresses", &self.progresses)
            .field("resumed_records", &self.resumed_records)
            .finish()
    }
}

/// 已经通过 分片上传器 V1 上传的分片信息
#[derive(Debug, Clone)]
pub struct MultiPartsV1UploaderUploadedPart {
    response_body: MkBlkResponseBody,
    uploaded_size: NonZeroU64,
    offset: u64,
    resumed: bool,
}

impl MultiPartsV1UploaderUploadedPart {
    /// 获取响应体
    #[inline]
    pub fn response_body(&self) -> &MkBlkResponseBody {
        &self.response_body
    }
}

impl UploadedPart for MultiPartsV1UploaderUploadedPart {
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

impl super::__private::Sealed for MultiPartsV1UploaderUploadedPart {}

impl<H: Digest> UploaderWithCallbacks for MultiPartsV1Uploader<H> {
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
    fn on_response_error<F: Fn(&mut ResponseError) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_after_response_error_callback(callback);
        self
    }
}

impl<H: Digest> MultiPartsUploaderWithCallbacks for MultiPartsV1Uploader<H> {
    #[inline]
    fn on_part_uploaded<F: Fn(&dyn UploadedPart) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_part_uploaded_callback(callback);
        self
    }
}

impl<H: Digest> MultiPartsV1Uploader<H> {
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

impl<H: Digest + Send + 'static> MultiPartsUploader for MultiPartsV1Uploader<H> {
    type HashAlgorithm = H;
    type InitializedParts = MultiPartsV1UploaderInitializedObject<Box<dyn DataSource<H>>>;
    type UploadedPart = MultiPartsV1UploaderUploadedPart;

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

    #[inline]
    fn upload_manager(&self) -> &UploadManager {
        &self.upload_manager
    }

    fn initialize_parts<D: DataSource<Self::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> ApiResult<Self::InitializedParts> {
        let up_endpoints = self.get_up_endpoints(&params)?;
        let resumed_records = self.resume_or_create_records(&source, up_endpoints)?;
        Ok(Self::InitializedParts {
            source: Box::new(source),
            params,
            resumed_records,
            progresses: Default::default(),
        })
    }

    fn try_to_resume_parts<D: DataSource<Self::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> Option<Self::InitializedParts> {
        source
            .source_key()
            .ok()
            .flatten()
            .and_then(|source_key| self.try_to_resume_records(&source_key).ok().flatten())
            .map(|resumed_records| Self::InitializedParts {
                source: Box::new(source),
                params,
                resumed_records,
                progresses: Default::default(),
            })
    }

    fn reinitialize_parts(
        &self,
        initialized: &mut Self::InitializedParts,
        options: ReinitializeOptions,
    ) -> ApiResult<()> {
        initialized.source.reset()?;
        let up_endpoints = options.get_up_endpoints(self, initialized)?;
        initialized.resumed_records = self.create_new_records(&initialized.source, up_endpoints)?;
        initialized.progresses = Default::default();
        Ok(())
    }

    fn upload_part(
        &self,
        initialized: &Self::InitializedParts,
        data_partitioner_provider: &dyn DataPartitionProvider,
    ) -> ApiResult<Option<Self::UploadedPart>> {
        let data_partitioner_provider = normalize_data_partitioner_provider(data_partitioner_provider);
        let total_size = initialized.source.total_size()?;
        return if let Some(mut reader) = initialized.source.slice(data_partitioner_provider.part_size())? {
            if let Some(part_size) = NonZeroU64::new(reader.len()?) {
                let progresses_key = initialized.progresses.add_new_part(part_size.into());
                if let Some(uploaded_part) = _could_resume(initialized, &mut reader, part_size) {
                    self.after_part_uploaded(&progresses_key, total_size, Some(&uploaded_part))?;
                    Ok(Some(uploaded_part))
                } else {
                    let params = MkBlkPathParams::default().set_block_size_as_u64(part_size.into());
                    let upload_token_signer =
                        self.make_upload_token_signer(initialized.params.object_name().map(|n| n.into()));
                    let mkblk = self.storage().resumable_upload_v1_make_block();
                    let uploaded_result = _upload_part(
                        self,
                        mkblk.new_request(initialized.up_endpoints(), params, upload_token_signer.as_ref()),
                        reader,
                        part_size,
                        &progresses_key,
                        initialized,
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

        fn _could_resume<H: Digest>(
            initialized: &MultiPartsV1UploaderInitializedObject<Box<dyn DataSource<H>>>,
            data_reader: &mut DataSourceReader,
            part_size: NonZeroU64,
        ) -> Option<MultiPartsV1UploaderUploadedPart> {
            let offset = data_reader.offset();
            initialized.resumed_records.take(offset).and_then(|record| {
                if record.size == part_size
                    && record.expired_at() > SystemTime::now()
                    && Some(record.base64ed_sha1.as_str()) == sha1_of_sync_reader(data_reader).ok().as_deref()
                {
                    Some(MultiPartsV1UploaderUploadedPart {
                        response_body: record.response_body.to_owned(),
                        uploaded_size: record.size,
                        resumed: true,
                        offset,
                    })
                } else {
                    None
                }
            })
        }

        #[allow(clippy::too_many_arguments)]
        fn _upload_part<'a, H: Digest + Send + 'static, E: EndpointsProvider + Clone + 'a>(
            uploader: &'a MultiPartsV1Uploader<H>,
            mut request: SyncMkBlkRequestBuilder<'a, E>,
            mut body: DataSourceReader,
            content_length: NonZeroU64,
            progresses_key: &'a ProgressesKey,
            initialized: &'a MultiPartsV1UploaderInitializedObject<Box<dyn DataSource<H>>>,
            data_partitioner_provider: &'a dyn DataPartitionProvider,
        ) -> ApiResult<MultiPartsV1UploaderUploadedPart> {
            let total_size = initialized.source.total_size()?;
            request.on_uploading_progress(move |_, transfer| {
                progresses_key.update_part(transfer.transferred_bytes());
                uploader.callbacks.upload_progress(&UploadingProgressInfo::new(
                    progresses_key.current_uploaded(),
                    total_size,
                ))
            });
            uploader.before_request_call(request.parts_mut())?;
            let base64ed_sha1 = sha1_of_sync_reader(&mut body)?;
            let body_offset = body.offset();
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
            if let Err(err) = &mut response_result {
                may_set_extensions_in_err(err);
            }
            let response_body = response_result?.into_body();
            let record = MultiPartsV1ResumableRecorderRecord {
                expired_timestamp: response_body.get_expired_at_as_u64(),
                offset: body_offset,
                size: content_length,
                base64ed_sha1,
                response_body,
            };
            initialized
                .resumed_records
                .persist(&record, &uploader.bucket_name()?, initialized.up_endpoints())
                .ok();
            Ok(MultiPartsV1UploaderUploadedPart::from_record(record, false))
        }
    }

    fn complete_parts(&self, initialized: &Self::InitializedParts, parts: &[Self::UploadedPart]) -> ApiResult<Value> {
        let file_size = get_file_size_from_uploaded_parts(parts);
        let upload_token_signer = self.make_upload_token_signer(initialized.params.object_name().map(|n| n.into()));
        let params = make_mkfile_path_params_from_initialized_parts(&initialized.params, file_size);
        let mkfile = self.storage().resumable_upload_v1_make_file();
        let body = make_mkfile_request_body_from_uploaded_parts(parts.to_vec());
        return _complete_parts(
            self,
            mkfile.new_request(initialized.up_endpoints(), params, upload_token_signer.as_ref()),
            &initialized.source,
            body,
        );

        fn _complete_parts<'a, H: Digest + Send + 'static, E: EndpointsProvider + Clone + 'a, D: DataSource<H>>(
            uploader: &'a MultiPartsV1Uploader<H>,
            mut request: SyncMkFileRequestBuilder<'a, E>,
            source: &D,
            body: String,
        ) -> ApiResult<Value> {
            uploader.before_request_call(request.parts_mut())?;
            let content_length = body.len() as u64;
            let mut response_result = request.call(Cursor::new(body), content_length);
            uploader.after_response_call(&mut response_result)?;
            if let Err(err) = &mut response_result {
                may_set_extensions_in_err(err);
            }
            let body = response_result?.into_body();
            uploader.try_to_delete_records(&source).ok();
            Ok(body.into())
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    type AsyncInitializedParts = MultiPartsV1UploaderInitializedObject<Box<dyn AsyncDataSource<H>>>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    type AsyncUploadedPart = MultiPartsV1UploaderUploadedPart;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_initialize_parts<D: AsyncDataSource<Self::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Self::AsyncInitializedParts>> {
        Box::pin(async move {
            let up_endpoints = self.async_get_up_endpoints(&params).await?;
            let resumed_records = self.async_resume_or_create_records(&source, up_endpoints).await?;
            Ok(Self::AsyncInitializedParts {
                source: Box::new(source),
                params,
                resumed_records,
                progresses: Default::default(),
            })
        })
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn try_to_async_resume_parts<D: AsyncDataSource<Self::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> BoxFuture<Option<Self::AsyncInitializedParts>> {
        Box::pin(async move {
            if let Some(source_key) = source.source_key().await.ok().flatten() {
                self.try_to_async_resume_records(&source_key)
                    .await
                    .ok()
                    .flatten()
                    .map(|resumed_records| Self::AsyncInitializedParts {
                        source: Box::new(source),
                        params,
                        resumed_records,
                        progresses: Default::default(),
                    })
            } else {
                None
            }
        })
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_reinitialize_parts<'r>(
        &'r self,
        initialized: &'r mut Self::AsyncInitializedParts,
        options: ReinitializeOptions,
    ) -> BoxFuture<'r, ApiResult<()>> {
        Box::pin(async move {
            initialized.source.reset().await?;
            let up_endpoints = options.async_get_up_endpoints(self, initialized).await?;
            initialized.resumed_records = self.async_create_new_records(&initialized.source, up_endpoints).await?;
            initialized.progresses = Default::default();
            Ok(())
        })
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_part<'r>(
        &'r self,
        initialized: &'r Self::AsyncInitializedParts,
        data_partitioner_provider: &'r dyn DataPartitionProvider,
    ) -> BoxFuture<'r, ApiResult<Option<Self::AsyncUploadedPart>>> {
        return Box::pin(async move {
            let data_partitioner_provider = normalize_data_partitioner_provider(data_partitioner_provider);
            let total_size = initialized.source.total_size().await?;
            if let Some(mut reader) = initialized.source.slice(data_partitioner_provider.part_size()).await? {
                if let Some(part_size) = NonZeroU64::new(reader.len().await?) {
                    let progresses_key = initialized.progresses.add_new_part(part_size.into());
                    if let Some(uploaded_part) = _could_resume(initialized, &mut reader, part_size).await {
                        self.after_part_uploaded(&progresses_key, total_size, Some(&uploaded_part))?;
                        Ok(Some(uploaded_part))
                    } else {
                        let params = MkBlkPathParams::default().set_block_size_as_u64(part_size.into());
                        let upload_token_signer =
                            self.make_upload_token_signer(initialized.params.object_name().map(|n| n.into()));
                        let mkblk = self.storage().resumable_upload_v1_make_block();
                        let uploaded_result = _upload_part(
                            self,
                            mkblk.new_async_request(initialized.up_endpoints(), params, upload_token_signer.as_ref()),
                            reader,
                            part_size,
                            &progresses_key,
                            initialized,
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

        async fn _could_resume<H: Digest>(
            initialized: &MultiPartsV1UploaderInitializedObject<Box<dyn AsyncDataSource<H>>>,
            data_reader: &mut AsyncDataSourceReader,
            part_size: NonZeroU64,
        ) -> Option<MultiPartsV1UploaderUploadedPart> {
            let offset = data_reader.offset();
            OptionFuture::from(initialized.resumed_records.take(offset).map(|record| async move {
                if record.size == part_size
                    && record.expired_at() > SystemTime::now()
                    && Some(record.base64ed_sha1.as_str()) == sha1_of_async_reader(data_reader).await.ok().as_deref()
                {
                    Some(MultiPartsV1UploaderUploadedPart {
                        response_body: record.response_body.to_owned(),
                        uploaded_size: record.size,
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

        #[allow(clippy::too_many_arguments)]
        async fn _upload_part<'a, H: Digest + Send + 'static, E: EndpointsProvider + Clone + 'a>(
            uploader: &'a MultiPartsV1Uploader<H>,
            mut request: AsyncMkBlkRequestBuilder<'a, E>,
            mut body: AsyncDataSourceReader,
            content_length: NonZeroU64,
            progresses_key: &'a ProgressesKey,
            initialized: &'a MultiPartsV1UploaderInitializedObject<Box<dyn AsyncDataSource<H>>>,
            data_partitioner_provider: &'a dyn DataPartitionProvider,
        ) -> ApiResult<MultiPartsV1UploaderUploadedPart> {
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
            if let Err(err) = &mut response_result {
                may_set_extensions_in_err(err);
            }
            let response_body = response_result?.into_body();
            let record = MultiPartsV1ResumableRecorderRecord {
                expired_timestamp: response_body.get_expired_at_as_u64(),
                offset: body_offset,
                size: content_length,
                base64ed_sha1,
                response_body,
            };
            initialized
                .resumed_records
                .async_persist(
                    &record,
                    &uploader.async_bucket_name().await?,
                    initialized.up_endpoints(),
                )
                .await
                .ok();
            Ok(MultiPartsV1UploaderUploadedPart::from_record(record, false))
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
            let file_size = get_file_size_from_uploaded_parts(parts);
            let upload_token_signer = self.make_upload_token_signer(initialized.params.object_name().map(|n| n.into()));
            let params = make_mkfile_path_params_from_initialized_parts(&initialized.params, file_size);
            let mkfile = self.storage().resumable_upload_v1_make_file();
            let body = make_mkfile_request_body_from_uploaded_parts(parts.to_vec());
            _complete_parts(
                self,
                mkfile.new_async_request(initialized.up_endpoints(), params, upload_token_signer.as_ref()),
                &initialized.source,
                body,
            )
            .await
        });

        async fn _complete_parts<
            'a,
            H: Digest + Send + 'static,
            E: EndpointsProvider + Clone + 'a,
            D: AsyncDataSource<H>,
        >(
            uploader: &'a MultiPartsV1Uploader<H>,
            mut request: AsyncMkFileRequestBuilder<'a, E>,
            source: &D,
            body: String,
        ) -> ApiResult<Value> {
            uploader.before_request_call(request.parts_mut())?;
            let content_length = body.len() as u64;
            let mut response_result = request.call(AsyncCursor::new(body), content_length).await;
            uploader.after_response_call(&mut response_result)?;
            if let Err(err) = &mut response_result {
                may_set_extensions_in_err(err);
            }
            let body = response_result?.into_body();
            uploader.try_to_async_delete_records(source).await.ok();
            Ok(body.into())
        }
    }
}

impl<H: Digest> super::__private::Sealed for MultiPartsV1Uploader<H> {}

fn make_mkfile_path_params_from_initialized_parts(object_params: &ObjectParams, file_size: u64) -> MkFilePathParams {
    let mut params = MkFilePathParams::default().set_size_as_u64(file_size);
    if let Some(object_name) = object_params.object_name() {
        params = params.set_object_name_as_str(object_name.to_string());
    }
    if let Some(file_name) = object_params.file_name() {
        params = params.set_file_name_as_str(file_name.to_string());
    }
    if let Some(mime) = object_params.content_type() {
        params = params.set_mime_type_as_str(mime.to_string());
    }
    for (metadata_name, metadata_value) in object_params.metadata() {
        params = params.append_custom_data_as_str("x-qn-meta-".to_owned() + metadata_name, metadata_value.to_owned());
    }
    for (var_name, var_value) in object_params.custom_vars() {
        params = params.append_custom_data_as_str("x:".to_owned() + var_name, var_value.to_owned());
    }
    params
}

fn get_file_size_from_uploaded_parts(parts: &[MultiPartsV1UploaderUploadedPart]) -> u64 {
    parts
        .iter()
        .map(|uploaded_part| uploaded_part.uploaded_size.get())
        .sum()
}

fn make_mkfile_request_body_from_uploaded_parts(mut parts: Vec<MultiPartsV1UploaderUploadedPart>) -> String {
    parts.sort_by_key(|part| part.offset);
    parts
        .iter()
        .map(|part| part.response_body.get_ctx_as_str())
        .enumerate()
        .fold(String::new(), |mut joined, (i, ctx)| {
            if i > 0 {
                joined += ",";
            }
            joined += ctx;
            joined
        })
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

fn may_set_extensions_in_err(err: &mut ResponseError) {
    match err.kind() {
        ResponseErrorKind::StatusCodeError(status_code) if status_code.as_u16() == 701 => {
            err.extensions_mut().insert(PartsExpiredError);
        }
        _ => {}
    }
}

fn normalize_data_partitioner_provider<P: DataPartitionProvider>(base: P) -> LimitedDataPartitionProvider<P> {
    LimitedDataPartitionProvider::new_with_non_zero_threshold(base, PART_SIZE, PART_SIZE)
}

impl<H: Digest + Send + 'static> MultiPartsV1Uploader<H> {
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
        uploaded_part: Option<&MultiPartsV1UploaderUploadedPart>,
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

    fn resume_or_create_records<D: DataSource<H>>(
        &self,
        source: &D,
        up_endpoints: Endpoints,
    ) -> IoResult<MultiPartsV1ResumableRecorderRecords> {
        let records = if let Some(source_key) = source.source_key()? {
            self.try_to_resume_records(&source_key)
                .ok()
                .flatten()
                .unwrap_or_else(|| _create_new_records(&self.resumable_recorder, &source_key, up_endpoints))
        } else {
            MultiPartsV1ResumableRecorderRecords::new(up_endpoints)
        };
        Ok(records)
    }

    fn try_to_resume_records(
        &self,
        source_key: &SourceKey<H>,
    ) -> ApiResult<Option<MultiPartsV1ResumableRecorderRecords>> {
        let mut medium = self.resumable_recorder.open_for_read(source_key)?;
        let mut lines = BufReader::new(&mut medium).lines();
        let header = if let Some(line) = lines.next() {
            let line = line?;
            let header: MultiPartsV1ResumableRecorderDeserializableHeader = serde_json::from_str(&line)?;
            if !header.is_v1() || header.bucket() != self.bucket_name()?.as_str() {
                return Ok(None);
            }
            header
        } else {
            return Ok(None);
        };
        let mut records = lines
            .map(|line| {
                let line = line?;
                let record: MultiPartsV1ResumableRecorderRecord = serde_json::from_str(&line)?;
                Ok(record)
            })
            .collect::<ApiResult<MultiPartsV1ResumableRecorderRecords>>()?;
        records.up_endpoints = header.up_endpoints;
        records.set_medium_for_append(self.resumable_recorder.open_for_append(source_key)?, true);
        Ok(Some(records))
    }

    fn create_new_records<D: DataSource<H>>(
        &self,
        source: &D,
        up_endpoints: Endpoints,
    ) -> IoResult<MultiPartsV1ResumableRecorderRecords> {
        let records = if let Some(source_key) = source.source_key()? {
            _create_new_records(&self.resumable_recorder, &source_key, up_endpoints)
        } else {
            MultiPartsV1ResumableRecorderRecords::new(up_endpoints)
        };
        Ok(records)
    }

    #[cfg(feature = "async")]
    async fn async_resume_or_create_records<D: AsyncDataSource<H>>(
        &self,
        source: &D,
        up_endpoints: Endpoints,
    ) -> IoResult<MultiPartsV1ResumableRecorderRecords> {
        let records = if let Some(source_key) = source.source_key().await? {
            if let Some(records) = self.try_to_async_resume_records(&source_key).await.ok().flatten() {
                records
            } else {
                _async_create_new_records(&self.resumable_recorder, &source_key, up_endpoints).await
            }
        } else {
            MultiPartsV1ResumableRecorderRecords::new(up_endpoints)
        };
        Ok(records)
    }

    #[cfg(feature = "async")]
    async fn try_to_async_resume_records(
        &self,
        source_key: &SourceKey<H>,
    ) -> ApiResult<Option<MultiPartsV1ResumableRecorderRecords>> {
        let mut medium = self.resumable_recorder.open_for_async_read(source_key).await?;
        let mut lines = AsyncBufReader::new(&mut medium).lines();
        let header = if let Some(line) = lines.try_next().await? {
            let header: MultiPartsV1ResumableRecorderDeserializableHeader = serde_json::from_str(&line)?;
            if !header.is_v1() || header.bucket() != self.async_bucket_name().await?.as_str() {
                return Ok(None);
            }
            header
        } else {
            return Ok(None);
        };
        let mut records = lines
            .map(|line| {
                let line = line?;
                let record: MultiPartsV1ResumableRecorderRecord = serde_json::from_str(&line)?;
                Ok::<_, ResponseError>(record)
            })
            .try_collect::<MultiPartsV1ResumableRecorderRecords>()
            .await?;
        records.up_endpoints = header.up_endpoints;
        records.set_medium_for_async_append(self.resumable_recorder.open_for_async_append(source_key).await?, true);
        Ok(Some(records))
    }

    #[cfg(feature = "async")]
    async fn async_create_new_records<D: AsyncDataSource<H>>(
        &self,
        source: &D,
        up_endpoints: Endpoints,
    ) -> IoResult<MultiPartsV1ResumableRecorderRecords> {
        let records = if let Some(source_key) = source.source_key().await? {
            _async_create_new_records(&self.resumable_recorder, &source_key, up_endpoints).await
        } else {
            MultiPartsV1ResumableRecorderRecords::new(up_endpoints)
        };
        Ok(records)
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

fn _create_new_records<D: Digest>(
    resumable_recorder: &dyn ResumableRecorder<HashAlgorithm = D>,
    source_key: &SourceKey<D>,
    up_endpoints: Endpoints,
) -> MultiPartsV1ResumableRecorderRecords {
    let mut records = MultiPartsV1ResumableRecorderRecords::new(up_endpoints);
    if let Ok(medium) = resumable_recorder.open_for_create_new(source_key) {
        records.set_medium_for_append(medium, false);
    }
    records
}

#[cfg(feature = "async")]
async fn _async_create_new_records<D: Digest>(
    resumable_recorder: &dyn ResumableRecorder<HashAlgorithm = D>,
    source_key: &SourceKey<D>,
    up_endpoints: Endpoints,
) -> MultiPartsV1ResumableRecorderRecords {
    let mut records = MultiPartsV1ResumableRecorderRecords::new(up_endpoints);
    if let Ok(medium) = resumable_recorder.open_for_async_create_new(source_key).await {
        records.set_medium_for_async_append(medium, false);
    }
    records
}

impl<H: Digest> Debug for MultiPartsV1Uploader<H> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultiPartsV1Uploader")
            .field("upload_manager", &self.upload_manager)
            .field("callbacks", &self.callbacks)
            .field("resumable_recorder", &self.resumable_recorder)
            .finish()
    }
}

impl<H: Digest> Clone for MultiPartsV1Uploader<H> {
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
const PART_SIZE: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(1 << 22) };

pub(super) fn make_callback_error(err: AnyError) -> ResponseError {
    ResponseError::new(HttpResponseErrorKind::CallbackError.into(), err)
}

#[derive(Debug, Clone, Serialize)]
struct MultiPartsV1ResumableRecorderSerializableHeader<'a> {
    #[serde(rename = "apiver")]
    api_version: u8,

    #[serde(rename = "fmtver")]
    format_version: u8,

    #[serde(rename = "bkt")]
    bucket: &'a str,

    #[serde(rename = "ups")]
    up_endpoints: &'a Endpoints,
}

impl<'a> MultiPartsV1ResumableRecorderSerializableHeader<'a> {
    fn v1(bucket: &'a str, up_endpoints: &'a Endpoints) -> Self {
        MultiPartsV1ResumableRecorderSerializableHeader {
            api_version: 1,
            format_version: 2,
            bucket,
            up_endpoints,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct MultiPartsV1ResumableRecorderDeserializableHeader {
    #[serde(rename = "apiver")]
    api_version: u8,

    #[serde(rename = "fmtver")]
    format_version: u8,

    #[serde(rename = "bkt")]
    bucket: BucketName,

    #[serde(rename = "ups")]
    up_endpoints: Endpoints,
}

impl MultiPartsV1ResumableRecorderDeserializableHeader {
    fn is_v1(&self) -> bool {
        self.api_version == 1 && self.format_version == 2
    }

    fn bucket(&self) -> &str {
        &self.bucket
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MultiPartsV1ResumableRecorderRecord {
    #[serde(rename = "off")]
    offset: u64,
    #[serde(rename = "size")]
    size: NonZeroU64,
    #[serde(rename = "body")]
    response_body: MkBlkResponseBody,
    #[serde(rename = "exat")]
    expired_timestamp: u64,
    #[serde(rename = "sha1")]
    base64ed_sha1: String,
}

impl MultiPartsV1ResumableRecorderRecord {
    fn expired_at(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.expired_timestamp)
    }
}

#[derive(Debug)]
struct AppendOnlyMediumForMultiPartsV1ResumableRecorderRecords {
    medium: Box<dyn AppendOnlyResumableRecorderMedium>,
    header_written: bool,
}

#[cfg(feature = "async")]
#[derive(Debug)]
struct AsyncAppendOnlyMediumForMultiPartsV1ResumableRecorderRecords {
    medium: Box<dyn AppendOnlyAsyncResumableRecorderMedium>,
    header_written: bool,
}

#[derive(Debug, Default, Clone)]
struct MultiPartsV1ResumableRecorderRecords {
    map: Arc<DashMap<u64, MultiPartsV1ResumableRecorderRecord>>,
    up_endpoints: Endpoints,
    append_only_medium: Option<Arc<Mutex<AppendOnlyMediumForMultiPartsV1ResumableRecorderRecords>>>,

    #[cfg(feature = "async")]
    async_append_only_medium: Option<Arc<AsyncMutex<AsyncAppendOnlyMediumForMultiPartsV1ResumableRecorderRecords>>>,
}

impl MultiPartsV1ResumableRecorderRecords {
    fn new(up_endpoints: Endpoints) -> Self {
        Self {
            up_endpoints,
            map: Default::default(),
            append_only_medium: None,
            #[cfg(feature = "async")]
            async_append_only_medium: None,
        }
    }
    fn set_medium_for_append(&mut self, medium: Box<dyn AppendOnlyResumableRecorderMedium>, header_written: bool) {
        self.append_only_medium = Some(Arc::new(Mutex::new(
            AppendOnlyMediumForMultiPartsV1ResumableRecorderRecords { medium, header_written },
        )));
    }

    #[cfg(feature = "async")]
    fn set_medium_for_async_append(
        &mut self,
        medium: Box<dyn AppendOnlyAsyncResumableRecorderMedium>,
        header_written: bool,
    ) {
        self.async_append_only_medium = Some(Arc::new(AsyncMutex::new(
            AsyncAppendOnlyMediumForMultiPartsV1ResumableRecorderRecords { medium, header_written },
        )));
    }

    fn take(&self, offset: u64) -> Option<MultiPartsV1ResumableRecorderRecord> {
        self.map.remove(&offset).map(|(_, record)| record)
    }

    fn persist(
        &self,
        record: &MultiPartsV1ResumableRecorderRecord,
        bucket_name: &str,
        up_endpoints: &Endpoints,
    ) -> ApiResult<()> {
        if let Some(append_only_medium) = self.append_only_medium.as_ref() {
            let mut buf = Vec::new();
            let mut append_only_medium = append_only_medium.lock().unwrap();
            if !append_only_medium.header_written {
                serde_json::to_writer(
                    &mut buf,
                    &MultiPartsV1ResumableRecorderSerializableHeader::v1(bucket_name, up_endpoints),
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
        record: &MultiPartsV1ResumableRecorderRecord,
        bucket_name: &str,
        up_endpoints: &Endpoints,
    ) -> ApiResult<()> {
        if let Some(append_only_medium) = self.async_append_only_medium.as_ref() {
            let mut append_only_medium = append_only_medium.lock().await;
            let mut buf = Vec::new();
            if !append_only_medium.header_written {
                serde_json::to_writer(
                    &mut buf,
                    &MultiPartsV1ResumableRecorderSerializableHeader::v1(bucket_name, up_endpoints),
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

impl FromIterator<MultiPartsV1ResumableRecorderRecord> for MultiPartsV1ResumableRecorderRecords {
    fn from_iter<T: IntoIterator<Item = MultiPartsV1ResumableRecorderRecord>>(iter: T) -> Self {
        Self {
            map: Arc::new(DashMap::from_iter(
                iter.into_iter().map(|record| (record.offset, record)),
            )),
            up_endpoints: Default::default(),
            append_only_medium: None,

            #[cfg(feature = "async")]
            async_append_only_medium: None,
        }
    }
}

impl Extend<MultiPartsV1ResumableRecorderRecord> for MultiPartsV1ResumableRecorderRecords {
    fn extend<T: IntoIterator<Item = MultiPartsV1ResumableRecorderRecord>>(&mut self, iter: T) {
        for record in iter {
            self.map.insert(record.offset, record);
        }
    }
}

impl MultiPartsV1UploaderUploadedPart {
    fn from_record(record: MultiPartsV1ResumableRecorderRecord, resumed: bool) -> Self {
        Self {
            response_body: record.response_body,
            uploaded_size: record.size,
            offset: record.offset,
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
    use std::{
        io::{copy as io_copy, sink as io_sink, Read},
        sync::atomic::{AtomicUsize, Ordering},
        thread::spawn as spawn_thread,
        time::{Duration, SystemTime},
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
    fn test_sync_multi_parts_v1_upload() -> Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            mkblk_counts: AtomicUsize,
            mkfile_counts: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                let resp_body = if request.url().path().starts_with("/mkblk/") {
                    let blk_size: u64;
                    scan_text!(request.url().path().bytes() => "/mkblk/{}", blk_size);

                    let mkblk_counts = match blk_size {
                        BLOCK_SIZE => self.mkblk_counts.fetch_add(1, Ordering::Relaxed),
                        LAST_BLOCK_SIZE => BLOCK_COUNT - 1,
                        _ => unreachable!(),
                    };
                    let body_len = size_of_sync_reader(request.body_mut()).unwrap();
                    assert_eq!(body_len, blk_size);
                    json_to_vec(&json!({
                        "ctx": format!("==={mkblk_counts}==="),
                        "checksum": sha1_of_sync_reader(request.body_mut()).unwrap(),
                        "offset": blk_size,
                        "host": "http://fakeexample.com",
                        "expired_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    }))
                    .unwrap()
                } else if request.url().path().starts_with("/mkfile/") {
                    assert_eq!(self.mkblk_counts.load(Ordering::Relaxed), BLOCK_COUNT - 1);
                    assert_eq!(self.mkfile_counts.fetch_add(1, Ordering::Relaxed), 0);
                    assert_eq!(request.url().path(), &format!("/mkfile/{FILE_SIZE}"));
                    let mut req_body = Vec::new();
                    io_copy(request.body_mut(), &mut req_body).unwrap();
                    let req_body = String::from_utf8(req_body).unwrap();
                    let contexts: Vec<_> = req_body.split(',').collect();
                    assert_eq!(contexts.len(), BLOCK_COUNT);
                    assert_eq!(*contexts.last().unwrap(), &format!("==={}===", BLOCK_COUNT - 1));
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

        let uploader = Arc::new(MultiPartsV1Uploader::new(
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
    async fn test_async_multi_parts_v1_upload() -> Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller {
            mkblk_counts: AtomicUsize,
            mkfile_counts: AtomicUsize,
        }

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                unreachable!()
            }

            fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                Box::pin(async move {
                    let resp_body = if request.url().path().starts_with("/mkblk/") {
                        let blk_size: u64;
                        scan_text!(request.url().path().bytes() => "/mkblk/{}", blk_size);

                        let mkblk_counts = match blk_size {
                            BLOCK_SIZE => self.mkblk_counts.fetch_add(1, Ordering::Relaxed),
                            LAST_BLOCK_SIZE => BLOCK_COUNT - 1,
                            _ => unreachable!(),
                        };
                        let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                        assert_eq!(body_len, blk_size);
                        json_to_vec(&json!({
                            "ctx": format!("==={mkblk_counts}==="),
                            "checksum": sha1_of_async_reader(request.body_mut()).await.unwrap(),
                            "offset": blk_size,
                            "host": "http://fakeexample.com",
                            "expired_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        }))
                        .unwrap()
                    } else if request.url().path().starts_with("/mkfile/") {
                        assert_eq!(self.mkblk_counts.load(Ordering::Relaxed), BLOCK_COUNT - 1);
                        assert_eq!(self.mkfile_counts.fetch_add(1, Ordering::Relaxed), 0);
                        assert_eq!(request.url().path(), &format!("/mkfile/{FILE_SIZE}"));
                        let mut req_body = Vec::new();
                        async_io_copy(request.body_mut(), &mut req_body).await.unwrap();
                        let req_body = String::from_utf8(req_body).unwrap();
                        let contexts: Vec<_> = req_body.split(',').collect();
                        assert_eq!(contexts.len(), BLOCK_COUNT);
                        assert_eq!(*contexts.last().unwrap(), &format!("==={}===", BLOCK_COUNT - 1));
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

        let uploader = Arc::new(MultiPartsV1Uploader::new(
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
        fn test_sync_multi_parts_v1_upload_with_recovery() -> Result<()> {
            env_logger::builder().is_test(true).try_init().ok();

            #[derive(Debug)]
            struct FakeHttpCaller {
                mkblk_counts: AtomicUsize,
                blk_num: usize,
            }

            impl FakeHttpCaller {
                fn new(blk_num: usize) -> Self {
                    Self {
                        blk_num,
                        mkblk_counts: Default::default(),
                    }
                }
            }

            impl HttpCaller for FakeHttpCaller {
                fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                    let resp_body = if request.url().path().starts_with("/mkblk/") {
                        let blk_size: u64;
                        scan_text!(request.url().path().bytes() => "/mkblk/{}", blk_size);

                        match blk_size {
                            BLOCK_SIZE => {
                                assert_eq!(self.mkblk_counts.fetch_add(1, Ordering::Relaxed), 0);
                            }
                            _ => unreachable!(),
                        }
                        let body_len = size_of_sync_reader(request.body_mut()).unwrap();
                        assert_eq!(body_len, blk_size);
                        json_to_vec(&json!({
                            "ctx": format!("==={}===", self.blk_num),
                            "checksum": sha1_of_sync_reader(request.body_mut()).unwrap(),
                            "offset": blk_size,
                            "host": "http://fakeexample.com",
                            "expired_at": (SystemTime::now() + Duration::from_secs(3600)).duration_since(UNIX_EPOCH).unwrap().as_secs(),
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
                let uploader = MultiPartsV1Uploader::new(
                    get_upload_manager(FakeHttpCaller::new(0)),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                );
                let file_source = FileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                assert!(uploader
                    .try_to_resume_parts(file_source.to_owned(), params.to_owned())
                    .is_none());
                let initialized_parts = uploader.initialize_parts(file_source, params)?;
                uploader
                    .upload_part(&initialized_parts, &new_data_partitioner_provider(BLOCK_SIZE))?
                    .unwrap();
            }
            {
                let uploader = MultiPartsV1Uploader::new(
                    get_upload_manager(FakeHttpCaller::new(1)),
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

            #[derive(Debug)]
            struct FakeHttpCaller2 {
                mkblk_counts: AtomicUsize,
                mkfile_counts: AtomicUsize,
                blk_num: u64,
            }

            impl FakeHttpCaller2 {
                fn new(blk_num: u64) -> Self {
                    Self {
                        blk_num,
                        mkblk_counts: Default::default(),
                        mkfile_counts: Default::default(),
                    }
                }
            }

            impl HttpCaller for FakeHttpCaller2 {
                fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                    let resp_body = if request.url().path().starts_with("/mkblk/") {
                        let blk_size: u64;
                        scan_text!(request.url().path().bytes() => "/mkblk/{}", blk_size);

                        match blk_size {
                            LAST_BLOCK_SIZE => {
                                assert_eq!(self.mkblk_counts.fetch_add(1, Ordering::Relaxed), 0)
                            }
                            _ => unreachable!(),
                        }
                        let body_len = size_of_sync_reader(request.body_mut()).unwrap();
                        assert_eq!(body_len, blk_size);
                        json_to_vec(&json!({
                            "ctx": format!("==={}===", self.blk_num),
                            "checksum": sha1_of_sync_reader(request.body_mut()).unwrap(),
                            "offset": blk_size,
                            "host": "http://fakeexample.com",
                            "expired_at": (SystemTime::now() + Duration::from_secs(3600)).duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        }))
                        .unwrap()
                    } else if request.url().path().starts_with("/mkfile/") {
                        assert_eq!(self.mkblk_counts.load(Ordering::Relaxed), 1);
                        assert_eq!(self.mkfile_counts.fetch_add(1, Ordering::Relaxed), 0);
                        assert_eq!(request.url().path(), &format!("/mkfile/{FILE_SIZE}"));
                        let mut req_body = Vec::new();
                        io_copy(request.body_mut(), &mut req_body).unwrap();
                        let req_body = String::from_utf8(req_body).unwrap();
                        let contexts: Vec<_> = req_body.split(',').collect();
                        assert_eq!(contexts.len(), BLOCK_COUNT);
                        assert_eq!(*contexts.last().unwrap(), &format!("==={}===", self.blk_num));
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
                let uploader = Arc::new(MultiPartsV1Uploader::new(
                    get_upload_manager(FakeHttpCaller2::new(2)),
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
            use async_std::fs::read_dir as async_read_dir;

            env_logger::builder().is_test(true).try_init().ok();

            #[derive(Debug)]
            struct FakeHttpCaller {
                mkblk_counts: AtomicUsize,
                blk_num: usize,
            }

            impl FakeHttpCaller {
                fn new(blk_num: usize) -> Self {
                    Self {
                        blk_num,
                        mkblk_counts: Default::default(),
                    }
                }
            }

            impl HttpCaller for FakeHttpCaller {
                fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                    unreachable!()
                }

                fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                    Box::pin(async move {
                        let resp_body = if request.url().path().starts_with("/mkblk/") {
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
                            json_to_vec(&json!({
                                "ctx": format!("==={}===", self.blk_num),
                                "checksum": sha1_of_async_reader(request.body_mut()).await.unwrap(),
                                "offset": blk_size,
                                "host": "http://fakeexample.com",
                                "expired_at": (SystemTime::now() + Duration::from_secs(3600)).duration_since(UNIX_EPOCH).unwrap().as_secs(),
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
                let uploader = MultiPartsV1Uploader::new(
                    get_upload_manager(FakeHttpCaller::new(0)),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                );
                let file_source = AsyncFileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                assert!(uploader
                    .try_to_async_resume_parts(file_source.to_owned(), params.to_owned())
                    .await
                    .is_none());
                let initialized_parts = uploader.async_initialize_parts(file_source, params).await?;
                uploader
                    .async_upload_part(&initialized_parts, &new_data_partitioner_provider(BLOCK_SIZE))
                    .await?
                    .unwrap();
            }
            {
                let uploader = MultiPartsV1Uploader::new(
                    get_upload_manager(FakeHttpCaller::new(1)),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                );
                let file_source = AsyncFileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let initialized_parts = uploader.try_to_async_resume_parts(file_source, params).await.unwrap();
                for _ in 0..2 {
                    uploader
                        .async_upload_part(&initialized_parts, &new_data_partitioner_provider(BLOCK_SIZE))
                        .await?
                        .unwrap();
                }
            }

            #[derive(Debug)]
            struct FakeHttpCaller2 {
                mkblk_counts: AtomicUsize,
                mkfile_counts: AtomicUsize,
                blk_num: usize,
            }

            impl FakeHttpCaller2 {
                fn new(blk_num: usize) -> Self {
                    Self {
                        blk_num,
                        mkblk_counts: Default::default(),
                        mkfile_counts: Default::default(),
                    }
                }
            }

            impl HttpCaller for FakeHttpCaller2 {
                fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                    unreachable!()
                }

                fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                    Box::pin(async move {
                        let resp_body = if request.url().path().starts_with("/mkblk/") {
                            let blk_size: u64;
                            scan_text!(request.url().path().bytes() => "/mkblk/{}", blk_size);

                            match blk_size {
                                LAST_BLOCK_SIZE => {
                                    assert_eq!(self.mkblk_counts.fetch_add(1, Ordering::Relaxed), 0);
                                }
                                _ => unreachable!(),
                            }
                            let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                            assert_eq!(body_len, blk_size);
                            json_to_vec(&json!({
                                "ctx": format!("==={}===", self.blk_num),
                                "checksum": sha1_of_async_reader(request.body_mut()).await.unwrap(),
                                "offset": blk_size,
                                "host": "http://fakeexample.com",
                                "expired_at": (SystemTime::now() + Duration::from_secs(3600)).duration_since(UNIX_EPOCH).unwrap().as_secs(),
                            }))
                            .unwrap()
                        } else if request.url().path().starts_with("/mkfile/") {
                            assert_eq!(self.mkblk_counts.load(Ordering::Relaxed), 1);
                            assert_eq!(self.mkfile_counts.fetch_add(1, Ordering::Relaxed), 0);
                            assert_eq!(request.url().path(), &format!("/mkfile/{FILE_SIZE}"));
                            let mut req_body = Vec::new();
                            async_io_copy(request.body_mut(), &mut req_body).await.unwrap();
                            let req_body = String::from_utf8(req_body).unwrap();
                            let contexts: Vec<_> = req_body.split(',').collect();
                            assert_eq!(contexts.len(), BLOCK_COUNT);
                            assert_eq!(*contexts.last().unwrap(), &format!("==={}===", self.blk_num));
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

            let mut initialized_parts = {
                let uploader = Arc::new(MultiPartsV1Uploader::new(
                    get_upload_manager(FakeHttpCaller2::new(2)),
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
                Arc::try_unwrap(initialized_parts).unwrap()
            };

            assert!(async_read_dir(resuming_files_dir.path()).await?.next().await.is_none());

            #[derive(Debug)]
            struct FakeHttpCaller3 {
                mkblk_counts: AtomicUsize,
                mkfile_counts: AtomicUsize,
            }

            impl FakeHttpCaller3 {
                fn new() -> Self {
                    Self {
                        mkblk_counts: Default::default(),
                        mkfile_counts: Default::default(),
                    }
                }
            }

            impl HttpCaller for FakeHttpCaller3 {
                fn call(&self, _request: &mut SyncRequest<'_>) -> SyncResponseResult {
                    unreachable!()
                }

                fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
                    Box::pin(async move {
                        let resp_body = if request.url().path().starts_with("/mkblk/") {
                            let blk_size: u64;
                            scan_text!(request.url().path().bytes() => "/mkblk/{}", blk_size);

                            match blk_size {
                                BLOCK_SIZE | LAST_BLOCK_SIZE => {
                                    self.mkblk_counts.fetch_add(1, Ordering::Relaxed);
                                }
                                _ => unreachable!(),
                            }
                            let body_len = size_of_async_reader(request.body_mut()).await.unwrap();
                            assert_eq!(body_len, blk_size);
                            json_to_vec(&json!({
                                "ctx": format!("==={}===", self.mkblk_counts.load(Ordering::Relaxed)),
                                "checksum": sha1_of_async_reader(request.body_mut()).await.unwrap(),
                                "offset": blk_size,
                                "host": "http://fakeexample.com",
                                "expired_at": (SystemTime::now() + Duration::from_secs(3600)).duration_since(UNIX_EPOCH).unwrap().as_secs(),
                            }))
                            .unwrap()
                        } else if request.url().path().starts_with("/mkfile/") {
                            assert_eq!(self.mkblk_counts.load(Ordering::Relaxed), 3);
                            assert_eq!(self.mkfile_counts.fetch_add(1, Ordering::Relaxed), 0);
                            assert_eq!(request.url().path(), &format!("/mkfile/{FILE_SIZE}"));
                            let mut req_body = Vec::new();
                            async_io_copy(request.body_mut(), &mut req_body).await.unwrap();
                            assert_eq!(String::from_utf8(req_body).unwrap().split(',').count(), BLOCK_COUNT);
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
                let uploader = Arc::new(MultiPartsV1Uploader::new(
                    get_upload_manager(FakeHttpCaller3::new()),
                    FileSystemResumableRecorder::<Sha1>::new(resuming_files_dir.path()),
                ));
                uploader
                    .async_reinitialize_parts(&mut initialized_parts, Default::default())
                    .await?;
                let initialized_parts = Arc::new(initialized_parts);
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
