use super::{
    super::{
        callbacks::{Callbacks, UploadingProgressInfo},
        data_source::SourceKey,
        upload_token::OwnedUploadTokenProviderOrReferenced,
        DataPartitionProvider, DataPartitionProviderFeedback, DataSourceReader,
        MultiplyDataPartitionProvider,
    },
    progress::{Progresses, ProgressesKey},
    DataSource, MultiPartsUploader, ObjectParams, ResumableRecorder, UploadManager,
    UploaderWithCallbacks,
};
use dashmap::DashMap;
use digest::Digest;
use qiniu_apis::{
    credential::AccessKey,
    http::{Reset, ResponseErrorKind as HttpResponseErrorKind, ResponseParts},
    http_client::{
        ApiResult, BucketRegionsProvider, CallbackResult, EndpointsProvider,
        RegionProviderEndpoints, RequestBuilderParts, Response, ResponseError,
    },
    storage::{
        self,
        resumable_upload_v1_make_block::{
            PathParams as MkBlkPathParams, ResponseBody as MkBlkResponseBody,
            SyncRequestBuilder as SyncMkBlkRequestBuilder,
        },
        resumable_upload_v1_make_file::{
            PathParams as MkFilePathParams, SyncRequestBuilder as SyncMkFileRequestBuilder,
        },
    },
};
use qiniu_upload_token::{BucketName, ObjectName};
use qiniu_utils::base64::urlsafe as urlsafe_base64;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::Sha1;
use std::{
    fmt::Debug,
    io::{copy as io_copy, BufRead, BufReader, Cursor, Read, Result as IoResult, Write},
    iter::FromIterator,
    num::NonZeroU64,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "async")]
use {
    super::super::AsyncDataSourceReader,
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

#[derive(Debug)]
pub struct MultiPartsV1Uploader<R: ?Sized> {
    upload_manager: UploadManager,
    uploaded_part_ttl: Duration,
    callbacks: Callbacks<'static>,
    resumable_recorder: R,
}

#[derive(Debug)]
pub struct MultiPartsV1UploaderInitializedObject<R: ResumableRecorder + ?Sized> {
    source: Arc<dyn DataSource<<R as ResumableRecorder>::HashAlgorithm>>,
    params: ObjectParams,
    progresses: Progresses,
    recovered_records: MultiPartsV1ResumableRecorderRecords<R>,
}

#[derive(Debug)]
pub struct MultiPartsV1UploaderUploadedPart {
    response_body: MkBlkResponseBody,
    uploaded_size: u64,
    offset: u64,
    uploaded_at: SystemTime,
}

impl<R> UploaderWithCallbacks for MultiPartsV1Uploader<R> {
    #[inline]
    fn on_before_request<
        F: Fn(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'static,
    >(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_before_request_callback(callback);
        self
    }

    #[inline]
    fn on_upload_progress<
        F: Fn(&UploadingProgressInfo) -> CallbackResult + Send + Sync + 'static,
    >(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_upload_progress_callback(callback);
        self
    }

    #[inline]
    fn on_response_ok<F: Fn(&mut ResponseParts) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_after_response_ok_callback(callback);
        self
    }

    #[inline]
    fn on_response_error<F: Fn(&ResponseError) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks
            .insert_after_response_error_callback(callback);
        self
    }
}

impl<R: ResumableRecorder> MultiPartsUploader for MultiPartsV1Uploader<R> {
    type ResumableRecorder = R;
    type InitializedParts = MultiPartsV1UploaderInitializedObject<R>;
    type UploadedPart = MultiPartsV1UploaderUploadedPart;

    #[inline]
    fn new(upload_manager: UploadManager, resumable_recorder: Self::ResumableRecorder) -> Self {
        Self {
            upload_manager,
            resumable_recorder,
            callbacks: Default::default(),
            uploaded_part_ttl: Duration::from_secs(3 * 86400),
        }
    }

    #[inline]
    fn uploaded_part_ttl(&self) -> Duration {
        self.uploaded_part_ttl
    }

    #[inline]
    fn uploaded_part_lifetime_mut(&mut self) -> &mut Duration {
        &mut self.uploaded_part_ttl
    }

    fn initialize_parts<
        D: DataSource<<Self::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static,
    >(
        &self,
        source: D,
        params: ObjectParams,
    ) -> ApiResult<Self::InitializedParts> {
        let recovered_records = self.try_to_recover(&source).unwrap_or_default();
        Ok(Self::InitializedParts {
            source: Arc::new(source),
            params,
            recovered_records,
            progresses: Default::default(),
        })
    }

    fn upload_part(
        &self,
        initialized: &Self::InitializedParts,
        data_partitioner_provider: &dyn DataPartitionProvider,
    ) -> ApiResult<Option<Self::UploadedPart>> {
        let data_partitioner_provider = MultiplyDataPartitionProvider::new_with_non_zero_multiply(
            data_partitioner_provider,
            PART_SIZE,
        );
        return if let Some(mut reader) = initialized
            .source
            .slice(data_partitioner_provider.part_size())?
        {
            let part_size = reader.len()?;
            assert!(part_size > 0);
            if let Some(uploaded_part) =
                _could_recover(initialized, &mut reader, part_size, self.uploaded_part_ttl)
            {
                return Ok(Some(uploaded_part));
            }
            let params = MkBlkPathParams::default().set_block_size_as_u64(part_size);
            let upload_token_signer =
                self.make_upload_token_signer(initialized.params.object_name().map(|n| n.into()));
            let mkblk = self.storage().resumable_upload_v1_make_block();
            let progresses_key = initialized.progresses.add_new_part(part_size);
            if let Some(region_provider) = initialized.params.region_provider() {
                _upload_part(
                    self,
                    mkblk.new_request(
                        RegionProviderEndpoints::new(region_provider),
                        params,
                        upload_token_signer.as_ref(),
                    ),
                    reader,
                    part_size,
                    progresses_key,
                    &initialized.recovered_records,
                    &data_partitioner_provider,
                )
                .map(Some)
            } else {
                _upload_part(
                    self,
                    mkblk.new_request(
                        RegionProviderEndpoints::new(self.get_bucket_region()?),
                        params,
                        upload_token_signer.as_ref(),
                    ),
                    reader,
                    part_size,
                    progresses_key,
                    &initialized.recovered_records,
                    &data_partitioner_provider,
                )
                .map(Some)
            }
        } else {
            Ok(None)
        };

        fn _could_recover<R: ResumableRecorder>(
            initialized: &MultiPartsV1UploaderInitializedObject<R>,
            data_reader: &mut DataSourceReader,
            part_size: u64,
            uploaded_part_ttl: Duration,
        ) -> Option<MultiPartsV1UploaderUploadedPart> {
            let offset = data_reader.offset();
            initialized
                .recovered_records
                .take(offset)
                .and_then(|record| {
                    if record.size == part_size
                        && UNIX_EPOCH
                            + Duration::from_secs(record.uploaded_timestamp)
                            + uploaded_part_ttl
                            > SystemTime::now()
                        && Some(record.response_body.get_checksum_as_str())
                            == sha1_of_sync_reader(data_reader).ok().as_deref()
                    {
                        Some(MultiPartsV1UploaderUploadedPart {
                            response_body: record.response_body.to_owned(),
                            uploaded_size: record.size,
                            uploaded_at: UNIX_EPOCH
                                + Duration::from_secs(record.uploaded_timestamp),
                            offset,
                        })
                    } else {
                        None
                    }
                })
        }

        fn _upload_part<
            'a,
            R: ResumableRecorder + Send + Sync,
            E: EndpointsProvider + Clone + 'a,
        >(
            uploader: &'a MultiPartsV1Uploader<R>,
            mut request: SyncMkBlkRequestBuilder<'a, E>,
            body: DataSourceReader,
            content_length: u64,
            progresses_key: ProgressesKey,
            recovered_records: &MultiPartsV1ResumableRecorderRecords<R>,
            data_partitioner_provider: &dyn DataPartitionProvider,
        ) -> ApiResult<MultiPartsV1UploaderUploadedPart> {
            request.on_uploading_progress(move |_, transfer| {
                progresses_key.update_part(transfer.transferred_bytes());
                uploader
                    .callbacks
                    .upload_progress(&UploadingProgressInfo::new(
                        progresses_key.current_uploaded(),
                        None,
                        transfer.body(),
                    ))
            });
            uploader.before_request_call(request.parts_mut())?;
            let body_offset = body.offset();
            let begin_at = Instant::now();
            let mut response_result = request.call(body, content_length);
            let elapsed = begin_at.elapsed();
            uploader.after_response_call(&mut response_result)?;
            data_partitioner_provider.feedback(DataPartitionProviderFeedback::new(
                NonZeroU64::new(content_length).unwrap(),
                elapsed,
                response_result.as_ref().err(),
            ));
            let response_body = response_result?.into_body();
            let record = MultiPartsV1ResumableRecorderRecord {
                response_body,
                offset: body_offset,
                size: content_length,
                uploaded_timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs()),
            };
            recovered_records.persist(&record).ok();
            Ok(record.into())
        }
    }

    fn complete_parts(
        &self,
        mut initialized: Self::InitializedParts,
        mut parts: Vec<Self::UploadedPart>,
    ) -> ApiResult<Value> {
        parts.sort_by_key(|part| part.offset);
        let file_size = get_file_size_from_uploaded_parts(&parts);
        let upload_token_signer =
            self.make_upload_token_signer(initialized.params.object_name().map(|n| n.into()));
        let params =
            make_mkfile_path_params_from_initialized_parts(&mut initialized.params, file_size);
        let mkfile = self.storage().resumable_upload_v1_make_file();
        let body = make_mkfile_request_body_from_uploaded_parts(&parts);
        return if let Some(region_provider) = initialized.params.region_provider() {
            _complete_parts(
                self,
                mkfile.new_request(
                    RegionProviderEndpoints::new(region_provider),
                    params,
                    upload_token_signer.as_ref(),
                ),
                &initialized.source,
                body,
            )
        } else {
            _complete_parts(
                self,
                mkfile.new_request(
                    RegionProviderEndpoints::new(self.get_bucket_region()?),
                    params,
                    upload_token_signer.as_ref(),
                ),
                &initialized.source,
                body,
            )
        };

        fn _complete_parts<
            'a,
            R: ResumableRecorder + Send + Sync,
            E: EndpointsProvider + Clone + 'a,
            D: DataSource<<<MultiPartsV1Uploader<R> as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm>,
        >(
            uploader: &'a MultiPartsV1Uploader<R>,
            mut request: SyncMkFileRequestBuilder<'a, E>,
            source: &D,
            body: String,
        ) -> ApiResult<Value>{
            uploader.before_request_call(request.parts_mut())?;
            let content_length = body.len() as u64;
            let mut response_result = request.call(Cursor::new(body), content_length);
            uploader.after_response_call(&mut response_result)?;
            let body = response_result?.into_body();
            uploader.try_to_delete_records(&source).ok();
            Ok(body.into())
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_initialize_parts<
        D: DataSource<<Self::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static,
    >(
        &self,
        source: D,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Self::InitializedParts>> {
        Box::pin(async move {
            let recovered_records = self.try_to_async_recover(&source).await.unwrap_or_default();
            Ok(Self::InitializedParts {
                source: Arc::new(source),
                params,
                recovered_records,
                progresses: Default::default(),
            })
        })
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_part<'r>(
        &'r self,
        initialized: &'r Self::InitializedParts,
        data_partitioner_provider: &'r dyn DataPartitionProvider,
    ) -> BoxFuture<'r, ApiResult<Option<Self::UploadedPart>>> {
        return Box::pin(async move {
            let data_partitioner_provider =
                MultiplyDataPartitionProvider::new_with_non_zero_multiply(
                    data_partitioner_provider,
                    PART_SIZE,
                );
            if let Some(mut reader) = initialized
                .source
                .async_slice(data_partitioner_provider.part_size())
                .await?
            {
                let part_size = reader.len().await?;
                assert!(part_size > 0);
                if let Some(uploaded_part) =
                    _could_recover(initialized, &mut reader, part_size, self.uploaded_part_ttl)
                        .await
                {
                    Ok(Some(uploaded_part))
                } else {
                    let params = MkBlkPathParams::default().set_block_size_as_u64(part_size);
                    let upload_token_signer = self.make_upload_token_signer(
                        initialized.params.object_name().map(|n| n.into()),
                    );
                    let mkblk = self.storage().resumable_upload_v1_make_block();
                    let progresses_key = initialized.progresses.add_new_part(part_size);
                    if let Some(region_provider) = initialized.params.region_provider() {
                        _upload_part(
                            self,
                            mkblk.new_async_request(
                                RegionProviderEndpoints::new(region_provider),
                                params,
                                upload_token_signer.as_ref(),
                            ),
                            reader,
                            part_size,
                            progresses_key,
                            &initialized.recovered_records,
                            &data_partitioner_provider,
                        )
                        .await
                        .map(Some)
                    } else {
                        _upload_part(
                            self,
                            mkblk.new_async_request(
                                RegionProviderEndpoints::new(self.async_get_bucket_region().await?),
                                params,
                                upload_token_signer.as_ref(),
                            ),
                            reader,
                            part_size,
                            progresses_key,
                            &initialized.recovered_records,
                            &data_partitioner_provider,
                        )
                        .await
                        .map(Some)
                    }
                }
            } else {
                Ok(None)
            }
        });

        async fn _could_recover<R: ResumableRecorder>(
            initialized: &MultiPartsV1UploaderInitializedObject<R>,
            data_reader: &mut AsyncDataSourceReader,
            part_size: u64,
            uploaded_part_ttl: Duration,
        ) -> Option<MultiPartsV1UploaderUploadedPart> {
            let offset = data_reader.offset();
            OptionFuture::from(initialized.recovered_records.take(offset).map(
                |record| async move {
                    if record.size == part_size
                        && UNIX_EPOCH
                            + Duration::from_secs(record.uploaded_timestamp)
                            + uploaded_part_ttl
                            > SystemTime::now()
                        && Some(record.response_body.get_checksum_as_str())
                            == sha1_of_async_reader(data_reader).await.ok().as_deref()
                    {
                        Some(MultiPartsV1UploaderUploadedPart {
                            response_body: record.response_body.to_owned(),
                            uploaded_size: record.size,
                            uploaded_at: UNIX_EPOCH
                                + Duration::from_secs(record.uploaded_timestamp),
                            offset,
                        })
                    } else {
                        None
                    }
                },
            ))
            .await
            .flatten()
        }

        async fn _upload_part<
            'a,
            R: ResumableRecorder + Send + Sync,
            E: EndpointsProvider + Clone + 'a,
        >(
            uploader: &'a MultiPartsV1Uploader<R>,
            mut request: AsyncMkBlkRequestBuilder<'a, E>,
            body: AsyncDataSourceReader,
            content_length: u64,
            progresses_key: ProgressesKey,
            recovered_records: &MultiPartsV1ResumableRecorderRecords<R>,
            data_partitioner_provider: &dyn DataPartitionProvider,
        ) -> ApiResult<MultiPartsV1UploaderUploadedPart> {
            request.on_uploading_progress(move |_, transfer| {
                progresses_key.update_part(transfer.transferred_bytes());
                uploader
                    .callbacks
                    .upload_progress(&UploadingProgressInfo::new(
                        progresses_key.current_uploaded(),
                        None,
                        transfer.body(),
                    ))
            });
            uploader.before_request_call(request.parts_mut())?;
            let body_offset = body.offset();
            let begin_at = Instant::now();
            let mut response_result = request.call(body, content_length).await;
            let elapsed = begin_at.elapsed();
            uploader.after_response_call(&mut response_result)?;
            data_partitioner_provider.feedback(DataPartitionProviderFeedback::new(
                NonZeroU64::new(content_length).unwrap(),
                elapsed,
                response_result.as_ref().err(),
            ));
            let response_body = response_result?.into_body();
            let record = MultiPartsV1ResumableRecorderRecord {
                response_body,
                offset: body_offset,
                size: content_length,
                uploaded_timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs()),
            };
            recovered_records.async_persist(&record).await.ok();
            Ok(record.into())
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_complete_parts(
        &self,
        mut initialized: Self::InitializedParts,
        mut parts: Vec<Self::UploadedPart>,
    ) -> BoxFuture<'_, ApiResult<Value>> {
        return Box::pin(async move {
            parts.sort_by_key(|part| part.offset);
            let file_size = get_file_size_from_uploaded_parts(&parts);
            let upload_token_signer =
                self.make_upload_token_signer(initialized.params.object_name().map(|n| n.into()));
            let params =
                make_mkfile_path_params_from_initialized_parts(&mut initialized.params, file_size);
            let mkfile = self.storage().resumable_upload_v1_make_file();
            let body = make_mkfile_request_body_from_uploaded_parts(&parts);
            if let Some(region_provider) = initialized.params.region_provider() {
                _complete_parts(
                    self,
                    mkfile.new_async_request(
                        RegionProviderEndpoints::new(region_provider),
                        params,
                        upload_token_signer.as_ref(),
                    ),
                    &initialized.source,
                    body,
                )
                .await
            } else {
                _complete_parts(
                    self,
                    mkfile.new_async_request(
                        RegionProviderEndpoints::new(self.async_get_bucket_region().await?),
                        params,
                        upload_token_signer.as_ref(),
                    ),
                    &initialized.source,
                    body,
                )
                .await
            }
        });

        async fn _complete_parts<
            'a,
            R: ResumableRecorder + Send + Sync,
            E: EndpointsProvider + Clone + 'a,
            D: DataSource<<<MultiPartsV1Uploader<R> as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm>,
        >(
            uploader: &'a MultiPartsV1Uploader<R>,
            mut request: AsyncMkFileRequestBuilder<'a, E>,
            source: &D,
            body: String,
        ) -> ApiResult<Value>{
            uploader.before_request_call(request.parts_mut())?;
            let content_length = body.len() as u64;
            let mut response_result = request.call(AsyncCursor::new(body), content_length).await;
            uploader.after_response_call(&mut response_result)?;
            let body = response_result?.into_body();
            uploader.try_to_async_delete_records(source).await.ok();
            Ok(body.into())
        }
    }
}

fn make_mkfile_path_params_from_initialized_parts(
    object_params: &mut ObjectParams,
    file_size: u64,
) -> MkFilePathParams {
    let mut params = MkFilePathParams::default().set_size_as_u64(file_size);
    if let Some(object_name) = object_params.take_object_name() {
        params = params.set_object_name_as_str(object_name.to_string());
    }
    if let Some(file_name) = object_params.take_file_name() {
        params = params.set_file_name_as_str(file_name.to_string());
    }
    if let Some(mime) = object_params.take_content_type() {
        params = params.set_mime_type_as_str(mime.to_string());
    }
    for (metadata_name, metadata_value) in object_params.take_metadata() {
        params = params
            .append_custom_data_as_str("x-qn-meta-".to_owned() + &metadata_name, metadata_value);
    }
    for (var_name, var_value) in object_params.take_custom_vars() {
        params = params.append_custom_data_as_str("x:".to_owned() + &var_name, var_value);
    }
    params
}

fn get_file_size_from_uploaded_parts(parts: &[MultiPartsV1UploaderUploadedPart]) -> u64 {
    parts
        .iter()
        .map(|uploaded_part| uploaded_part.uploaded_size)
        .sum()
}

fn make_mkfile_request_body_from_uploaded_parts(
    parts: &[MultiPartsV1UploaderUploadedPart],
) -> String {
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

fn sha1_of_sync_reader<R: Read + Reset>(mut reader: &mut R) -> IoResult<String> {
    let mut hasher = Sha1::new();
    io_copy(&mut reader, &mut hasher)?;
    reader.reset()?;
    Ok(urlsafe_base64(hasher.finalize().as_slice()))
}

impl<R> MultiPartsV1Uploader<R> {
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
        if self.callbacks.before_request(request).is_cancelled() {
            Err(make_user_cancelled_error(
                "Cancelled by on_before_request() callback",
            ))
        } else {
            Ok(())
        }
    }

    fn after_response_call<B>(&self, response: &mut ApiResult<Response<B>>) -> ApiResult<()> {
        if self.callbacks.after_response(response).is_cancelled() {
            Err(make_user_cancelled_error(
                "Cancelled by on_after_response() callback",
            ))
        } else {
            Ok(())
        }
    }
}

impl<R: ResumableRecorder> MultiPartsV1Uploader<R> {
    fn get_bucket_region(&self) -> ApiResult<BucketRegionsProvider> {
        Ok(self
            .upload_manager
            .queryer()
            .query(self.access_key()?, self.bucket_name()?))
    }

    #[cfg(feature = "async")]
    async fn async_get_bucket_region(&self) -> ApiResult<BucketRegionsProvider> {
        Ok(self.upload_manager.queryer().query(
            self.async_access_key().await?,
            self.async_bucket_name().await?,
        ))
    }

    fn make_upload_token_signer(
        &self,
        object_name: Option<ObjectName>,
    ) -> OwnedUploadTokenProviderOrReferenced<'_> {
        self.upload_manager
            .upload_token()
            .make_upload_token_provider(object_name)
    }

    fn try_to_recover<
        D: DataSource<<<Self as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static,
    >(
        &self,
        source: &D,
    ) -> ApiResult<MultiPartsV1ResumableRecorderRecords<R>>{
        return source
            .source_key()?
            .map(|source_key| {
                _try_to_recover(&self.resumable_recorder, &source_key)
                    .ok()
                    .flatten()
                    .map(Ok)
                    .unwrap_or_else(|| _new_records(&self.resumable_recorder, &source_key))
            })
            .unwrap_or_else(|| Ok(Default::default()));

        fn _try_to_recover<R: ResumableRecorder>(
            resumable_recorder: &R,
            source_key: &SourceKey<R::HashAlgorithm>,
        ) -> ApiResult<Option<MultiPartsV1ResumableRecorderRecords<R>>> {
            let mut medium = resumable_recorder.open_for_read(source_key)?;
            let mut lines = BufReader::new(&mut medium).lines();
            if let Some(line) = lines.next() {
                let line = line?;
                let header: MultiPartsV1ResumableRecorderHeader = serde_json::from_str(&line)?;
                if !header.is_v1() {
                    return Ok(None);
                }
            }
            let mut records: MultiPartsV1ResumableRecorderRecords<R> = lines
                .map(|line| {
                    let line = line?;
                    let record: MultiPartsV1ResumableRecorderRecord = serde_json::from_str(&line)?;
                    Ok(record)
                })
                .collect::<ApiResult<_>>()?;
            records.set_medium_for_append(resumable_recorder.open_for_append(source_key)?, true);
            Ok(Some(records))
        }

        fn _new_records<R: ResumableRecorder>(
            resumable_recorder: &R,
            source_key: &SourceKey<R::HashAlgorithm>,
        ) -> ApiResult<MultiPartsV1ResumableRecorderRecords<R>> {
            let mut records = MultiPartsV1ResumableRecorderRecords::default();
            records
                .set_medium_for_append(resumable_recorder.open_for_create_new(source_key)?, false);
            Ok(records)
        }
    }

    #[cfg(feature = "async")]
    async fn try_to_async_recover<
        D: DataSource<<<Self as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static,
    >(
        &self,
        source: &D,
    ) -> ApiResult<MultiPartsV1ResumableRecorderRecords<R>>{
        return OptionFuture::from(
            source
                .async_source_key()
                .await?
                .map(|source_key| async move {
                    if let Some(records) = _try_to_recover(&self.resumable_recorder, &source_key)
                        .await
                        .ok()
                        .flatten()
                    {
                        Ok(records)
                    } else {
                        _new_records(&self.resumable_recorder, &source_key).await
                    }
                }),
        )
        .await
        .unwrap_or_else(|| Ok(Default::default()));

        async fn _try_to_recover<R: ResumableRecorder>(
            resumable_recorder: &R,
            source_key: &SourceKey<R::HashAlgorithm>,
        ) -> ApiResult<Option<MultiPartsV1ResumableRecorderRecords<R>>> {
            let mut medium = resumable_recorder.open_for_async_read(source_key).await?;
            let mut lines = AsyncBufReader::new(&mut medium).lines();
            if let Some(line) = lines.try_next().await? {
                let header: MultiPartsV1ResumableRecorderHeader = serde_json::from_str(&line)?;
                if !header.is_v1() {
                    return Ok(None);
                }
            }
            let mut records: MultiPartsV1ResumableRecorderRecords<R> = lines
                .map(|line| {
                    let line = line?;
                    let record: MultiPartsV1ResumableRecorderRecord = serde_json::from_str(&line)?;
                    Ok::<_, ResponseError>(record)
                })
                .try_collect()
                .await?;
            records.set_medium_for_async_append(
                resumable_recorder.open_for_async_append(source_key).await?,
                true,
            );
            Ok(Some(records))
        }

        async fn _new_records<R: ResumableRecorder>(
            resumable_recorder: &R,
            source_key: &SourceKey<R::HashAlgorithm>,
        ) -> ApiResult<MultiPartsV1ResumableRecorderRecords<R>> {
            let mut records = MultiPartsV1ResumableRecorderRecords::default();
            records.set_medium_for_async_append(
                resumable_recorder
                    .open_for_async_create_new(source_key)
                    .await?,
                false,
            );
            Ok(records)
        }
    }

    fn try_to_delete_records<
        D: DataSource<
            <<Self as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm,
        >,
    >(
        &self,
        source: &D,
    ) -> ApiResult<()> {
        if let Some(source_key) = source.source_key()? {
            self.resumable_recorder.delete(&source_key)?;
        }
        Ok(())
    }

    #[cfg(feature = "async")]
    async fn try_to_async_delete_records<
        D: DataSource<
            <<Self as MultiPartsUploader>::ResumableRecorder as ResumableRecorder>::HashAlgorithm,
        >,
    >(
        &self,
        source: &D,
    ) -> ApiResult<()> {
        if let Some(source_key) = source.async_source_key().await? {
            self.resumable_recorder.async_delete(&source_key).await?;
        }
        Ok(())
    }
}

#[cfg(feature = "async")]
async fn sha1_of_async_reader<R: AsyncRead + AsyncReset + Unpin>(
    reader: &mut R,
) -> IoResult<String> {
    use futures::{
        io::{copy as async_io_copy, sink as async_sink},
        ready,
    };
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };

    struct ReadHasher<'r, R> {
        reader: &'r mut R,
        sha1: Sha1,
    }

    impl<R: AsyncRead + Unpin> AsyncRead for ReadHasher<'_, R> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<IoResult<usize>> {
            let size = ready!(Pin::new(&mut self.reader).poll_read(cx, buf))?;
            self.sha1.update(buf);
            Poll::Ready(Ok(size))
        }
    }

    let mut hasher = ReadHasher {
        reader,
        sha1: Sha1::new(),
    };
    async_io_copy(&mut hasher, &mut async_sink()).await?;
    hasher.reader.reset().await?;
    Ok(urlsafe_base64(hasher.sha1.finalize().as_slice()))
}

#[allow(unsafe_code)]
const PART_SIZE: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(1 << 22) };

fn make_user_cancelled_error(message: &str) -> ResponseError {
    ResponseError::new(HttpResponseErrorKind::UserCanceled.into(), message)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MultiPartsV1ResumableRecorderHeader {
    #[serde(rename = "ver")]
    version: u8,
}

impl MultiPartsV1ResumableRecorderHeader {
    fn v1() -> Self {
        Self { version: 1 }
    }

    fn is_v1(&self) -> bool {
        self.version == 1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MultiPartsV1ResumableRecorderRecord {
    #[serde(rename = "off")]
    offset: u64,
    #[serde(rename = "size")]
    size: u64,
    #[serde(rename = "body")]
    response_body: MkBlkResponseBody,
    #[serde(rename = "upat")]
    uploaded_timestamp: u64,
}

#[derive(Debug)]
struct AppendOnlyMediumForMultiPartsV1ResumableRecorderRecords<R: ResumableRecorder + ?Sized> {
    medium: <R as ResumableRecorder>::AppendOnlyMedium,
    header_written: bool,
}

#[cfg(feature = "async")]
#[derive(Debug)]
struct AsyncAppendOnlyMediumForMultiPartsV1ResumableRecorderRecords<R: ResumableRecorder + ?Sized> {
    medium: <R as ResumableRecorder>::AsyncAppendOnlyMedium,
    header_written: bool,
}

#[derive(Debug)]
struct MultiPartsV1ResumableRecorderRecords<R: ResumableRecorder + ?Sized> {
    map: DashMap<u64, MultiPartsV1ResumableRecorderRecord>,
    append_only_medium: Option<Mutex<AppendOnlyMediumForMultiPartsV1ResumableRecorderRecords<R>>>,

    #[cfg(feature = "async")]
    async_append_only_medium:
        Option<AsyncMutex<AsyncAppendOnlyMediumForMultiPartsV1ResumableRecorderRecords<R>>>,
}

impl<R: ResumableRecorder + ?Sized> Default for MultiPartsV1ResumableRecorderRecords<R> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            append_only_medium: None,
            #[cfg(feature = "async")]
            async_append_only_medium: None,
        }
    }
}

impl<R: ResumableRecorder + ?Sized> MultiPartsV1ResumableRecorderRecords<R> {
    fn set_medium_for_append(
        &mut self,
        medium: <R as ResumableRecorder>::AppendOnlyMedium,
        header_written: bool,
    ) {
        self.append_only_medium = Some(Mutex::new(
            AppendOnlyMediumForMultiPartsV1ResumableRecorderRecords {
                medium,
                header_written,
            },
        ));
    }

    #[cfg(feature = "async")]
    fn set_medium_for_async_append(
        &mut self,
        medium: <R as ResumableRecorder>::AsyncAppendOnlyMedium,
        header_written: bool,
    ) {
        self.async_append_only_medium = Some(AsyncMutex::new(
            AsyncAppendOnlyMediumForMultiPartsV1ResumableRecorderRecords {
                medium,
                header_written,
            },
        ));
    }

    fn take(&self, offset: u64) -> Option<MultiPartsV1ResumableRecorderRecord> {
        self.map.remove(&offset).map(|(_, record)| record)
    }

    fn persist(&self, record: &MultiPartsV1ResumableRecorderRecord) -> ApiResult<()> {
        if let Some(append_only_medium) = self.append_only_medium.as_ref() {
            let mut buf = Vec::new();
            let mut append_only_medium = append_only_medium.lock().unwrap();
            if !append_only_medium.header_written {
                serde_json::to_writer(&mut buf, &MultiPartsV1ResumableRecorderHeader::v1())?;
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
    async fn async_persist(&self, record: &MultiPartsV1ResumableRecorderRecord) -> ApiResult<()> {
        if let Some(append_only_medium) = self.async_append_only_medium.as_ref() {
            let mut append_only_medium = append_only_medium.lock().await;
            let mut buf = Vec::new();
            if !append_only_medium.header_written {
                serde_json::to_writer(&mut buf, &MultiPartsV1ResumableRecorderHeader::v1())?;
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

impl<R: ResumableRecorder + ?Sized> FromIterator<MultiPartsV1ResumableRecorderRecord>
    for MultiPartsV1ResumableRecorderRecords<R>
{
    fn from_iter<T: IntoIterator<Item = MultiPartsV1ResumableRecorderRecord>>(iter: T) -> Self {
        Self {
            map: DashMap::from_iter(iter.into_iter().map(|record| (record.offset, record))),
            append_only_medium: None,

            #[cfg(feature = "async")]
            async_append_only_medium: None,
        }
    }
}

impl<R: ResumableRecorder + ?Sized> Extend<MultiPartsV1ResumableRecorderRecord>
    for MultiPartsV1ResumableRecorderRecords<R>
{
    fn extend<T: IntoIterator<Item = MultiPartsV1ResumableRecorderRecord>>(&mut self, iter: T) {
        self.map
            .extend(iter.into_iter().map(|record| (record.offset, record)))
    }
}

impl From<MultiPartsV1ResumableRecorderRecord> for MultiPartsV1UploaderUploadedPart {
    fn from(record: MultiPartsV1ResumableRecorderRecord) -> Self {
        Self {
            response_body: record.response_body,
            uploaded_size: record.size,
            offset: record.offset,
            uploaded_at: UNIX_EPOCH + Duration::from_secs(record.uploaded_timestamp),
        }
    }
}

impl From<MultiPartsV1UploaderUploadedPart> for MultiPartsV1ResumableRecorderRecord {
    fn from(record: MultiPartsV1UploaderUploadedPart) -> Self {
        Self {
            response_body: record.response_body,
            size: record.uploaded_size,
            offset: record.offset,
            uploaded_timestamp: record
                .uploaded_at
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DummyResumableRecorder, FileDataSource, FixedDataPartitionProvider, UploadTokenSigner,
    };
    use anyhow::Result;
    use qiniu_apis::{
        credential::Credential,
        http::{
            HeaderName, HeaderValue, HttpCaller, StatusCode, SyncRequest, SyncResponse,
            SyncResponseBody, SyncResponseResult,
        },
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
                        "ctx": format!("==={}===", mkblk_counts),
                        "checksum": sha1_of_sync_reader(request.body_mut()).unwrap(),
                        "offset": blk_size,
                        "host": "http://fakeexample.com",
                        "expired_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    })).unwrap()
                } else if request.url().path().starts_with("/mkfile/") {
                    assert_eq!(self.mkblk_counts.load(Ordering::Relaxed), BLOCK_COUNT - 1);
                    assert_eq!(self.mkfile_counts.fetch_add(1, Ordering::Relaxed), 0);
                    assert_eq!(request.url().path(), &format!("/mkfile/{}", FILE_SIZE));
                    let mut req_body = Vec::new();
                    io_copy(request.body_mut(), &mut req_body).unwrap();
                    let req_body = String::from_utf8(req_body).unwrap();
                    let contexts: Vec<_> = req_body.split(',').collect();
                    assert_eq!(contexts.len(), BLOCK_COUNT);
                    assert_eq!(
                        *contexts.last().unwrap(),
                        &format!("==={}===", BLOCK_COUNT - 1)
                    );
                    json_to_vec(&json!({
                        "done": 1,
                    }))
                    .unwrap()
                } else {
                    unreachable!()
                };
                Ok(SyncResponse::builder()
                    .status_code(StatusCode::OK)
                    .header(
                        HeaderName::from_static("x-reqid"),
                        HeaderValue::from_static("FakeReqid"),
                    )
                    .body(SyncResponseBody::from_bytes(resp_body))
                    .build())
            }

            #[cfg(feature = "async")]
            fn async_call(
                &self,
                _request: &mut AsyncRequest<'_>,
            ) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let uploader = Arc::new(MultiPartsV1Uploader::new(
            get_upload_manager(FakeHttpCaller::default()),
            DummyResumableRecorder::new(),
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
                    uploader.upload_part(
                        &initialized_parts,
                        &new_data_partitioner_provider(BLOCK_SIZE),
                    )
                })
            })
            .collect::<Vec<_>>();
        let parts = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<ApiResult<Vec<_>>>()?;
        let parts = parts
            .into_iter()
            .map(|part| part.unwrap())
            .collect::<Vec<_>>();
        let body = uploader.complete_parts(Arc::try_unwrap(initialized_parts).unwrap(), parts)?;
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

            fn async_call<'a>(
                &'a self,
                request: &'a mut AsyncRequest<'_>,
            ) -> BoxFuture<'a, AsyncResponseResult> {
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
                            "ctx": format!("==={}===", mkblk_counts),
                            "checksum": sha1_of_async_reader(request.body_mut()).await.unwrap(),
                            "offset": blk_size,
                            "host": "http://fakeexample.com",
                            "expired_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        })).unwrap()
                    } else if request.url().path().starts_with("/mkfile/") {
                        assert_eq!(self.mkblk_counts.load(Ordering::Relaxed), BLOCK_COUNT - 1);
                        assert_eq!(self.mkfile_counts.fetch_add(1, Ordering::Relaxed), 0);
                        assert_eq!(request.url().path(), &format!("/mkfile/{}", FILE_SIZE));
                        let mut req_body = Vec::new();
                        async_io_copy(request.body_mut(), &mut req_body)
                            .await
                            .unwrap();
                        let req_body = String::from_utf8(req_body).unwrap();
                        let contexts: Vec<_> = req_body.split(',').collect();
                        assert_eq!(contexts.len(), BLOCK_COUNT);
                        assert_eq!(
                            *contexts.last().unwrap(),
                            &format!("==={}===", BLOCK_COUNT - 1)
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
                        .header(
                            HeaderName::from_static("x-reqid"),
                            HeaderValue::from_static("FakeReqid"),
                        )
                        .body(AsyncResponseBody::from_bytes(resp_body))
                        .build())
                })
            }
        }

        let uploader = Arc::new(MultiPartsV1Uploader::new(
            get_upload_manager(FakeHttpCaller::default()),
            DummyResumableRecorder::new(),
        ));
        let file_path = spawn_task(async { random_file_path(FILE_SIZE) }).await?;
        let file_source = FileDataSource::new(file_path.as_os_str());
        let params = ObjectParams::builder()
            .region_provider(single_up_domain_region())
            .build();
        let initialized_parts =
            Arc::new(uploader.async_initialize_parts(file_source, params).await?);

        let tasks = (0..BLOCK_COUNT).map(|_| {
            let uploader = uploader.to_owned();
            let initialized_parts = initialized_parts.to_owned();
            spawn_task(async move {
                uploader
                    .async_upload_part(
                        &initialized_parts,
                        &new_data_partitioner_provider(BLOCK_SIZE),
                    )
                    .await
            })
        });
        let parts = join_all(tasks)
            .await
            .into_iter()
            .collect::<ApiResult<Vec<_>>>()?;
        let parts = parts
            .into_iter()
            .map(|part| part.unwrap())
            .collect::<Vec<_>>();
        let body = uploader
            .async_complete_parts(Arc::try_unwrap(initialized_parts).unwrap(), parts)
            .await?;
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
                            "expired_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        })).unwrap()
                    } else {
                        unreachable!()
                    };
                    Ok(SyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header(
                            HeaderName::from_static("x-reqid"),
                            HeaderValue::from_static("FakeReqid"),
                        )
                        .body(SyncResponseBody::from_bytes(resp_body))
                        .build())
                }

                #[cfg(feature = "async")]
                fn async_call(
                    &self,
                    _request: &mut AsyncRequest<'_>,
                ) -> BoxFuture<AsyncResponseResult> {
                    unreachable!()
                }
            }

            let resuming_files_dir = TempfileBuilder::new().tempdir()?;
            let file_path = random_file_path(FILE_SIZE)?;
            {
                let uploader = MultiPartsV1Uploader::new(
                    get_upload_manager(FakeHttpCaller::new(0)),
                    FileSystemResumableRecorder::new(resuming_files_dir.path()),
                );
                let file_source = FileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let initialized_parts = uploader.initialize_parts(file_source, params)?;
                uploader
                    .upload_part(
                        &initialized_parts,
                        &new_data_partitioner_provider(BLOCK_SIZE),
                    )?
                    .unwrap();
            }
            {
                let uploader = MultiPartsV1Uploader::new(
                    get_upload_manager(FakeHttpCaller::new(1)),
                    FileSystemResumableRecorder::new(resuming_files_dir.path()),
                );
                let file_source = FileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let initialized_parts = uploader.initialize_parts(file_source, params)?;
                for _ in 0..2 {
                    uploader
                        .upload_part(
                            &initialized_parts,
                            &new_data_partitioner_provider(BLOCK_SIZE),
                        )?
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
                            "expired_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        })).unwrap()
                    } else if request.url().path().starts_with("/mkfile/") {
                        assert_eq!(self.mkblk_counts.load(Ordering::Relaxed), 1);
                        assert_eq!(self.mkfile_counts.fetch_add(1, Ordering::Relaxed), 0);
                        assert_eq!(request.url().path(), &format!("/mkfile/{}", FILE_SIZE));
                        let mut req_body = Vec::new();
                        io_copy(request.body_mut(), &mut req_body).unwrap();
                        let req_body = String::from_utf8(req_body).unwrap();
                        let contexts: Vec<_> = req_body.split(',').collect();
                        assert_eq!(contexts.len(), BLOCK_COUNT);
                        assert_eq!(
                            *contexts.last().unwrap(),
                            &format!("==={}===", self.blk_num)
                        );
                        json_to_vec(&json!({
                            "done": 1,
                        }))
                        .unwrap()
                    } else {
                        unreachable!()
                    };
                    Ok(SyncResponse::builder()
                        .status_code(StatusCode::OK)
                        .header(
                            HeaderName::from_static("x-reqid"),
                            HeaderValue::from_static("FakeReqid"),
                        )
                        .body(SyncResponseBody::from_bytes(resp_body))
                        .build())
                }

                #[cfg(feature = "async")]
                fn async_call(
                    &self,
                    _request: &mut AsyncRequest<'_>,
                ) -> BoxFuture<AsyncResponseResult> {
                    unreachable!()
                }
            }

            {
                let uploader = Arc::new(MultiPartsV1Uploader::new(
                    get_upload_manager(FakeHttpCaller2::new(2)),
                    FileSystemResumableRecorder::new(resuming_files_dir.path()),
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
                            uploader.upload_part(
                                &initialized_parts,
                                &new_data_partitioner_provider(BLOCK_SIZE),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                let parts = threads
                    .into_iter()
                    .map(|thread| thread.join().unwrap())
                    .collect::<ApiResult<Vec<_>>>()?;
                let parts = parts
                    .into_iter()
                    .map(|part| part.unwrap())
                    .collect::<Vec<_>>();
                let body =
                    uploader.complete_parts(Arc::try_unwrap(initialized_parts).unwrap(), parts)?;
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

                fn async_call<'a>(
                    &'a self,
                    request: &'a mut AsyncRequest<'_>,
                ) -> BoxFuture<'a, AsyncResponseResult> {
                    Box::pin(async move {
                        let resp_body = if request.url().path().starts_with("/mkblk/") {
                            let blk_size: u64;
                            scan_text!(request.url().path().bytes() => "/mkblk/{}", blk_size);

                            match blk_size {
                                BLOCK_SIZE => {
                                    assert_eq!(
                                        self.mkblk_counts.fetch_add(1, Ordering::Relaxed),
                                        0
                                    );
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
                                "expired_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                            })).unwrap()
                        } else {
                            unreachable!()
                        };
                        Ok(AsyncResponse::builder()
                            .status_code(StatusCode::OK)
                            .header(
                                HeaderName::from_static("x-reqid"),
                                HeaderValue::from_static("FakeReqid"),
                            )
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
                    FileSystemResumableRecorder::new(resuming_files_dir.path()),
                );
                let file_source = FileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let initialized_parts =
                    uploader.async_initialize_parts(file_source, params).await?;
                uploader
                    .async_upload_part(
                        &initialized_parts,
                        &new_data_partitioner_provider(BLOCK_SIZE),
                    )
                    .await?
                    .unwrap();
            }
            {
                let uploader = MultiPartsV1Uploader::new(
                    get_upload_manager(FakeHttpCaller::new(1)),
                    FileSystemResumableRecorder::new(resuming_files_dir.path()),
                );
                let file_source = FileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let initialized_parts =
                    uploader.async_initialize_parts(file_source, params).await?;
                for _ in 0..2 {
                    uploader
                        .async_upload_part(
                            &initialized_parts,
                            &new_data_partitioner_provider(BLOCK_SIZE),
                        )
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

                fn async_call<'a>(
                    &'a self,
                    request: &'a mut AsyncRequest<'_>,
                ) -> BoxFuture<'a, AsyncResponseResult> {
                    Box::pin(async move {
                        let resp_body = if request.url().path().starts_with("/mkblk/") {
                            let blk_size: u64;
                            scan_text!(request.url().path().bytes() => "/mkblk/{}", blk_size);

                            match blk_size {
                                LAST_BLOCK_SIZE => {
                                    assert_eq!(
                                        self.mkblk_counts.fetch_add(1, Ordering::Relaxed),
                                        0
                                    );
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
                                "expired_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                            })).unwrap()
                        } else if request.url().path().starts_with("/mkfile/") {
                            assert_eq!(self.mkblk_counts.load(Ordering::Relaxed), 1);
                            assert_eq!(self.mkfile_counts.fetch_add(1, Ordering::Relaxed), 0);
                            assert_eq!(request.url().path(), &format!("/mkfile/{}", FILE_SIZE));
                            let mut req_body = Vec::new();
                            async_io_copy(request.body_mut(), &mut req_body)
                                .await
                                .unwrap();
                            let req_body = String::from_utf8(req_body).unwrap();
                            let contexts: Vec<_> = req_body.split(',').collect();
                            assert_eq!(contexts.len(), BLOCK_COUNT);
                            assert_eq!(
                                *contexts.last().unwrap(),
                                &format!("==={}===", self.blk_num)
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
                            .header(
                                HeaderName::from_static("x-reqid"),
                                HeaderValue::from_static("FakeReqid"),
                            )
                            .body(AsyncResponseBody::from_bytes(resp_body))
                            .build())
                    })
                }
            }

            {
                let uploader = Arc::new(MultiPartsV1Uploader::new(
                    get_upload_manager(FakeHttpCaller2::new(2)),
                    FileSystemResumableRecorder::new(resuming_files_dir.path()),
                ));
                let file_source = FileDataSource::new(file_path.as_os_str());
                let params = ObjectParams::builder()
                    .region_provider(single_up_domain_region())
                    .build();
                let initialized_parts =
                    Arc::new(uploader.async_initialize_parts(file_source, params).await?);
                let tasks = (0..BLOCK_COUNT).map(|_| {
                    let uploader = uploader.to_owned();
                    let initialized_parts = initialized_parts.to_owned();
                    spawn_task(async move {
                        uploader
                            .async_upload_part(
                                &initialized_parts,
                                &new_data_partitioner_provider(BLOCK_SIZE),
                            )
                            .await
                    })
                });
                let parts = join_all(tasks)
                    .await
                    .into_iter()
                    .collect::<ApiResult<Vec<_>>>()?;
                let parts = parts
                    .into_iter()
                    .map(|part| part.unwrap())
                    .collect::<Vec<_>>();
                let body = uploader
                    .async_complete_parts(Arc::try_unwrap(initialized_parts).unwrap(), parts)
                    .await?;
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
            .push_up_preferred_endpoint(("fakeup.example.com".to_owned(), 8080).into())
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
    async fn size_of_async_reader<R: AsyncRead + AsyncReset + Unpin>(
        mut reader: &mut R,
    ) -> IoResult<u64> {
        let size = async_io_copy(&mut reader, &mut async_io_sink()).await?;
        reader.reset().await?;
        Ok(size)
    }
}
