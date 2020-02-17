use super::{
    io_status_manager::{IOStatusManager, Result as IOStatusResult},
    upload_recorder::{FileUploadRecordMedium, FileUploadRecordMediumBlockItem, FileUploadRecordMediumMetadata},
    upload_response_callback, BucketUploader, TokenizedUploadLogger, UpType, UploadLoggerRecordBuilder, UploadResponse,
};
use crate::{
    http::{Client, Error as HTTPError, ErrorKind as HTTPErrorKind, Result as HTTPResult, RetryKind},
    utils::{base64, ron::Ron, seek_adapter},
};
use mime::Mime;
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    borrow::Cow,
    boxed::Box,
    cell::Cell,
    collections::HashMap,
    convert::TryInto,
    fs::File,
    io::{Read, Result as IOResult, Seek, SeekFrom},
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering::Relaxed},
        Mutex,
    },
    time::{Duration, Instant},
};
use tap::{TapOptionOps, TapResultOps};

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct InitPartsResult {
    upload_id: Box<str>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct UploadPartResult {
    etag: Box<str>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Part {
    etag: Box<str>,
    part_number: usize,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct CompletedParts<'f> {
    parts: Vec<Part>,
    fname: Option<Cow<'f, str>>,
    mime_type: Option<Box<str>>,
    metadata: Option<HashMap<Cow<'f, str>, Cow<'f, str>>>,
    custom_vars: Option<HashMap<Cow<'f, str>, Cow<'f, str>>>,
}

struct FromResuming {
    upload_id: Box<str>,
    up_urls: Box<[Box<str>]>,
    recorder: FileUploadRecordMedium,
}

struct UploadingProgressCallback<'u> {
    callback: &'u (dyn Fn(u64, Option<u64>) + Send + Sync),
    completed_size: AtomicU64,
    total_size: Option<u64>,
}

pub(super) struct ResumableUploaderBuilder<'u> {
    bucket_uploader: &'u BucketUploader,
    upload_token: Cow<'u, str>,
    key: Option<Cow<'u, str>>,
    metadata: Option<HashMap<Cow<'u, str>, Cow<'u, str>>>,
    custom_vars: Option<HashMap<Cow<'u, str>, Cow<'u, str>>>,
    on_uploading_progress: Option<&'u (dyn Fn(u64, Option<u64>) + Send + Sync)>,
    thread_pool: Option<Ron<'u, ThreadPool>>,
    max_concurrency: usize,
    upload_logger: Option<TokenizedUploadLogger>,
}

pub(super) struct ResumableUploader<'u, R: Read + Seek + Send + 'u> {
    bucket_uploader: &'u BucketUploader,
    upload_token: Cow<'u, str>,
    key: Option<Cow<'u, str>>,
    completed_parts: Mutex<CompletedParts<'u>>,
    checksum_enabled: bool,
    is_seekable: bool,
    block_size: u32,
    io_size: Option<u64>,
    io: R,
    uploaded_size: AtomicU64,
    file_path: Option<Cow<'u, Path>>,
    from_resuming: Option<FromResuming>,
    uploading_progress_callback: Option<UploadingProgressCallback<'u>>,
    thread_pool: Ron<'u, ThreadPool>,
    max_concurrency: usize,
    upload_logger: Option<TokenizedUploadLogger>,
}

impl<'u> ResumableUploaderBuilder<'u> {
    pub(super) fn new(bucket_uploader: &'u BucketUploader, upload_token: Cow<'u, str>) -> ResumableUploaderBuilder<'u> {
        ResumableUploaderBuilder {
            bucket_uploader,
            upload_token: upload_token.clone(),
            key: None,
            metadata: None,
            custom_vars: None,
            on_uploading_progress: None,
            thread_pool: None,
            upload_logger: bucket_uploader.upload_logger().map(|upload_logger| {
                upload_logger.tokenize(
                    upload_token.into_owned().into(),
                    bucket_uploader.http_client().to_owned(),
                )
            }),
            max_concurrency: 0,
        }
    }

    pub(super) fn thread_pool(mut self, thread_pool: Ron<'u, ThreadPool>) -> ResumableUploaderBuilder<'u> {
        self.thread_pool = Some(thread_pool);
        self
    }

    pub(super) fn max_concurrency(mut self, concurrency: usize) -> ResumableUploaderBuilder<'u> {
        self.max_concurrency = concurrency;
        self
    }

    pub(super) fn key(mut self, key: Cow<'u, str>) -> ResumableUploaderBuilder<'u> {
        self.key = Some(key);
        self
    }

    pub(super) fn metadata(mut self, metadata: HashMap<Cow<'u, str>, Cow<'u, str>>) -> ResumableUploaderBuilder<'u> {
        self.metadata = Some(metadata);
        self
    }

    pub(super) fn vars(mut self, vars: HashMap<Cow<'u, str>, Cow<'u, str>>) -> ResumableUploaderBuilder<'u> {
        let mut hashmap = HashMap::new();
        for (k, v) in vars.into_iter() {
            hashmap.insert(Cow::Owned("x:".to_owned() + &k), v);
        }
        self.custom_vars = Some(hashmap);
        self
    }

    pub(super) fn on_uploading_progress(
        mut self,
        callback: &'u (dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> ResumableUploaderBuilder<'u> {
        self.on_uploading_progress = Some(callback);
        self
    }

    pub(super) fn file<'n: 'u>(
        self,
        file: File,
        file_path: Cow<'n, Path>,
        file_name: Option<Cow<'n, str>>,
        file_size: u64,
        mime_type: Option<Mime>,
        checksum_enabled: bool,
    ) -> IOResult<ResumableUploader<'u, File>> {
        let bucket_uploader = self.bucket_uploader;
        let block_size = bucket_uploader.http_client().config().upload_block_size();
        Ok(ResumableUploader {
            bucket_uploader,
            upload_token: self.upload_token,
            key: self.key,
            file_path: Some(file_path),
            io: file,
            io_size: Some(file_size),
            uploaded_size: AtomicU64::new(0),
            checksum_enabled,
            is_seekable: true,
            block_size,
            completed_parts: Mutex::new(CompletedParts {
                parts: Vec::with_capacity({
                    let block_size: u64 = block_size.into();
                    ((file_size + block_size - 1) / (block_size))
                        .try_into()
                        .unwrap_or(usize::max_value())
                }),
                fname: file_name,
                mime_type: mime_type.map(|m| m.as_ref().into()),
                metadata: self.metadata,
                custom_vars: self.custom_vars,
            }),
            from_resuming: None,
            uploading_progress_callback: self.on_uploading_progress.map(|callback| UploadingProgressCallback {
                callback,
                completed_size: AtomicU64::new(0),
                total_size: Some(file_size),
            }),
            thread_pool: self
                .thread_pool
                .or_else(|| bucket_uploader.thread_pool().map(|pool| Ron::Referenced(pool)))
                .unwrap_or_else(|| {
                    Ron::Owned(
                        ThreadPoolBuilder::new()
                            .thread_name(|index| format!("resumable_uploader_thread_{}", index))
                            .build()
                            .unwrap(),
                    )
                }),
            max_concurrency: self.max_concurrency,
            upload_logger: self.upload_logger,
        })
    }

    pub(super) fn stream<'n: 'u, R: Read + Send + 'u>(
        self,
        stream: R,
        mime_type: Option<Mime>,
        file_name: Option<Cow<'n, str>>,
        checksum_enabled: bool,
    ) -> IOResult<ResumableUploader<'u, seek_adapter::SeekAdapter<R>>> {
        let bucket_uploader = self.bucket_uploader;
        Ok(ResumableUploader {
            bucket_uploader,
            upload_token: self.upload_token,
            key: self.key,
            file_path: None,
            io: seek_adapter::SeekAdapter(stream),
            io_size: None,
            uploaded_size: AtomicU64::new(0),
            checksum_enabled,
            is_seekable: false,
            block_size: bucket_uploader.http_client().config().upload_block_size(),
            completed_parts: Mutex::new(CompletedParts {
                parts: Vec::new(),
                fname: file_name,
                mime_type: mime_type.map(|m| m.as_ref().into()),
                metadata: self.metadata,
                custom_vars: self.custom_vars,
            }),
            from_resuming: None,
            uploading_progress_callback: self.on_uploading_progress.map(|callback| UploadingProgressCallback {
                callback,
                completed_size: AtomicU64::new(0),
                total_size: None,
            }),
            thread_pool: self
                .thread_pool
                .or_else(|| bucket_uploader.thread_pool().map(|pool| Ron::Referenced(pool)))
                .unwrap_or_else(|| {
                    Ron::Owned(
                        ThreadPoolBuilder::new()
                            .thread_name(|index| format!("resumable_uploader_thread_{}", index))
                            .build()
                            .unwrap(),
                    )
                }),
            max_concurrency: self.max_concurrency,
            upload_logger: self.upload_logger,
        })
    }
}

impl<'u, R: Read + Seek + Send> ResumableUploader<'u, R> {
    pub(super) fn send(&mut self) -> HTTPResult<UploadResponse> {
        let base_path = self.make_base_path();
        let authorization = self.make_authorization();
        if let Ok(Some(result)) = self.try_to_resume(&base_path, &authorization) {
            return Ok(result);
        }
        let mut prev_err: Option<HTTPError> = None;
        for up_urls in self.bucket_uploader.up_urls_list().iter() {
            match self.try_to_init_and_upload_with_log(
                &up_urls.iter().map(|url| url.as_ref()).collect::<Box<[&str]>>(),
                &base_path,
                &authorization,
            ) {
                Ok(result) => {
                    return Ok(result);
                }
                Err(err) => match err.retry_kind() {
                    RetryKind::RetryableError | RetryKind::HostUnretryableError | RetryKind::ZoneUnretryableError => {
                        if self.is_seekable {
                            prev_err = Some(err);
                            continue;
                        } else {
                            return Err(err);
                        }
                    }
                    _ => {
                        return Err(err);
                    }
                },
            }
        }

        Err(prev_err.expect("ResumableUploader::send() should try at lease once, but not"))
    }

    fn try_to_init_and_upload_with_log(
        &mut self,
        up_urls: &[&str],
        base_path: &str,
        authorization: &str,
    ) -> HTTPResult<UploadResponse> {
        if self.is_seekable {
            self.io
                .seek(SeekFrom::Start(0))
                .map_err(|err| HTTPError::new_unretryable_error_from_parts(HTTPErrorKind::IOError(err), None, None))?;
        }
        self.uploaded_size.store(0, Relaxed);
        if let Some(uploading_progress_callback) = &self.uploading_progress_callback {
            uploading_progress_callback.completed_size.store(0, Relaxed);
        }
        self.completed_parts.lock().unwrap().parts.clear();
        let timer = Instant::now();
        let result = self.try_to_init_and_upload(up_urls, base_path, authorization);
        if let Some(upload_logger) = &self.upload_logger {
            let uploaded_size = self.uploaded_size.load(Relaxed);
            match &result {
                Ok(_) => {
                    let _ = upload_logger.log(
                        UploadLoggerRecordBuilder::default()
                            .duration(timer.elapsed())
                            .up_type(UpType::Chunkedv2)
                            .sent(uploaded_size)
                            .total_size(uploaded_size)
                            .build(),
                    );
                }
                Err(err) => {
                    let mut record_builder = UploadLoggerRecordBuilder::default()
                        .duration(timer.elapsed())
                        .up_type(UpType::Chunkedv2)
                        .sent(uploaded_size)
                        .http_error(err);
                    if let Some(total_size) = self.io_size {
                        record_builder = record_builder.total_size(u64::max(uploaded_size, total_size));
                    }
                    let _ = upload_logger.log(record_builder.build());
                }
            }
        }
        result
    }

    fn try_to_init_and_upload(
        &mut self,
        up_urls: &[&str],
        base_path: &str,
        authorization: &str,
    ) -> HTTPResult<UploadResponse> {
        let upload_id = self.init_parts(&base_path, up_urls, &authorization)?;
        let recorder = self.file_path.as_ref().and_then(|file_path| {
            self.bucket_uploader
                .recorder()
                .open_and_write_metadata(
                    file_path,
                    self.key.as_ref().map(|key| key.as_ref()),
                    &upload_id,
                    up_urls,
                    self.block_size,
                )
                .ok()
        });
        self.start_uploading_blocks(
            up_urls,
            &(base_path.to_owned() + "/" + &upload_id),
            authorization,
            recorder,
        )
    }

    fn start_uploading_blocks(
        &mut self,
        up_urls: &[&str],
        base_path: &str,
        authorization: &str,
        upload_recorder: Option<FileUploadRecordMedium>,
    ) -> HTTPResult<UploadResponse> {
        let io_status_manager = IOStatusManager::new(
            &mut self.io,
            self.block_size,
            &self
                .completed_parts
                .lock()
                .unwrap()
                .parts
                .iter()
                .map(|part| part.part_number)
                .collect::<Vec<_>>(),
        );
        let http_client = self.bucket_uploader.http_client();
        let block_size = self.block_size;
        let completed_parts = &self.completed_parts;
        let uploaded_size = &self.uploaded_size;
        let uploading_progress_callback = self.uploading_progress_callback.as_ref();
        let checksum_enabled = self.checksum_enabled;
        let upload_logger = self.upload_logger.as_ref();
        let concurrency = {
            let mut c = self.thread_pool.current_num_threads();
            if (1..c).contains(&self.max_concurrency) {
                c = self.max_concurrency;
            }
            c
        };

        self.thread_pool.scope(|s| {
            for _ in 0..concurrency {
                s.spawn(|_| {
                    let mut md5 = OptionalMd5::new(checksum_enabled);
                    loop {
                        match io_status_manager.read() {
                            Some(part_data) => {
                                let last_block_uploaded = Cell::new(0);
                                match Self::upload_part(
                                    http_client,
                                    &(base_path.to_owned() + "/" + &part_data.part_number.to_string()),
                                    up_urls,
                                    authorization,
                                    &part_data.data,
                                    part_data.part_number,
                                    &mut md5,
                                    |block_uploaded, _| {
                                        if let Some(progress) = uploading_progress_callback {
                                            let added_size =
                                                block_uploaded - last_block_uploaded.replace(block_uploaded);
                                            (progress.callback)(
                                                progress.completed_size.fetch_add(added_size, Relaxed) + added_size,
                                                progress.total_size,
                                            );
                                        }
                                    },
                                    |_, _, _| {
                                        if let Some(progress) = uploading_progress_callback {
                                            progress
                                                .completed_size
                                                .fetch_sub(last_block_uploaded.replace(0), Relaxed);
                                        }
                                    },
                                    upload_logger,
                                    upload_recorder.as_ref(),
                                ) {
                                    Ok(etag) => {
                                        completed_parts.lock().unwrap().parts.push(Part {
                                            etag,
                                            part_number: part_data.part_number,
                                        });
                                        uploaded_size.fetch_add(block_size.into(), Relaxed);
                                    }
                                    Err(err) => {
                                        io_status_manager.error(err);
                                        return;
                                    }
                                };
                            }
                            None => {
                                return;
                            }
                        }
                    }
                });
            }
        });

        match io_status_manager.result() {
            IOStatusResult::Success => self.complete_parts(base_path, up_urls, authorization).tap_ok(|_| {
                self.file_path.as_ref().tap_some(|file_path| {
                    let _ = self
                        .bucket_uploader
                        .recorder()
                        .drop(file_path, self.key.as_ref().map(|key| key.as_ref()));
                })
            }),
            IOStatusResult::IOError(err) => Err(HTTPError::new_unretryable_error_from_parts(
                HTTPErrorKind::IOError(err),
                None,
                None,
            )),
            IOStatusResult::HTTPError(err) => Err(err),
        }
    }

    pub(super) fn prepare_for_resuming(
        &mut self,
        file_record: FileUploadRecordMediumMetadata,
        block_records: Box<[FileUploadRecordMediumBlockItem]>,
        recorder: FileUploadRecordMedium,
    ) {
        let mut io_offset = 0u64;
        {
            let block_records: Vec<FileUploadRecordMediumBlockItem> = block_records.into();
            let mut completed_parts = self.completed_parts.lock().unwrap();
            for block_record in block_records {
                completed_parts.parts.push(Part {
                    etag: block_record.etag,
                    part_number: block_record.part_number,
                });
                io_offset += u64::from(file_record.block_size);
            }
        }
        self.from_resuming = Some(FromResuming {
            upload_id: file_record.upload_id,
            up_urls: file_record.up_urls,
            recorder,
        });
        self.block_size = file_record.block_size;
        self.uploaded_size = AtomicU64::new(io_offset);
    }

    fn init_parts(&self, base_path: &str, up_urls: &[&str], authorization: &str) -> HTTPResult<Box<str>> {
        let result: InitPartsResult = self
            .bucket_uploader
            .http_client()
            .post(base_path, up_urls)
            .header("Authorization", authorization)
            .idempotent()
            .on_response(&|response, duration| {
                let result = upload_response_callback(response);
                if result.is_ok() {
                    if let Some(upload_logger) = &self.upload_logger {
                        let _ = upload_logger.log(
                            UploadLoggerRecordBuilder::default()
                                .response(response)
                                .duration(duration)
                                .up_type(UpType::InitParts)
                                .build(),
                        );
                    }
                }
                result
            })
            .on_error(&|host_url, err, duration| {
                if let Some(upload_logger) = &self.upload_logger {
                    let _ = upload_logger.log({
                        let mut builder = UploadLoggerRecordBuilder::default()
                            .duration(duration)
                            .up_type(UpType::InitParts)
                            .http_error(err);
                        if let Some(host_url) = host_url {
                            builder = builder.host(host_url);
                        }
                        builder.build()
                    });
                }
            })
            .accept_json()
            .no_body()
            .send()?
            .parse_json()?;
        Ok(result.upload_id)
    }

    #[allow(clippy::too_many_arguments)]
    fn upload_part(
        http_client: &Client,
        path: &str,
        up_urls: &[&str],
        authorization: &str,
        part: &[u8],
        part_number: usize,
        md5_hasher: &mut OptionalMd5,
        on_progress: impl Fn(u64, u64),
        on_error: impl Fn(Option<&str>, &HTTPError, Duration),
        upload_logger: Option<&TokenizedUploadLogger>,
        upload_recorder: Option<&FileUploadRecordMedium>,
    ) -> HTTPResult<Box<str>> {
        let mut builder = http_client
            .put(path, up_urls)
            .header("Authorization", authorization)
            .on_uploading_progress(&on_progress);
        if let Some(md5) = md5_hasher.hash(part) {
            builder = builder.header("Content-MD5", md5);
        }
        let result: UploadPartResult = builder
            .idempotent()
            .on_response(&|response, duration| {
                let result = upload_response_callback(response);
                if result.is_ok() {
                    if let Some(upload_logger) = upload_logger {
                        let _ = upload_logger.log(
                            UploadLoggerRecordBuilder::default()
                                .response(response)
                                .duration(duration)
                                .up_type(UpType::UploadPart)
                                .sent(part.len().try_into().unwrap_or(u64::max_value()))
                                .total_size(part.len().try_into().unwrap_or(u64::max_value()))
                                .build(),
                        );
                    }
                }
                result
            })
            .on_error(&|host_url, err, duration| {
                (on_error)(host_url, err, duration);
                if let Some(upload_logger) = upload_logger {
                    let _ = upload_logger.log({
                        let mut builder = UploadLoggerRecordBuilder::default()
                            .duration(duration)
                            .up_type(UpType::UploadPart)
                            .http_error(err)
                            .total_size(part.len().try_into().unwrap_or(u64::max_value()));
                        if let Some(host_url) = host_url {
                            builder = builder.host(host_url);
                        }
                        builder.build()
                    });
                }
            })
            .accept_json()
            .raw_body("application/octet-stream", part.as_ref())
            .send()?
            .parse_json()?;
        if let Some(upload_recorder) = upload_recorder {
            upload_recorder
                .append(&result.etag, part_number)
                .map_err(|err| HTTPError::new_unretryable_error_from_parts(HTTPErrorKind::IOError(err), None, None))?;
        }
        Ok(result.etag)
    }

    fn complete_parts(&self, path: &str, up_urls: &[&str], authorization: &str) -> HTTPResult<UploadResponse> {
        let mut completed_parts = self.completed_parts.lock().unwrap();
        completed_parts.parts.sort_unstable_by_key(|part| part.part_number);
        let value: Value = self
            .bucket_uploader
            .http_client()
            .post(path, up_urls)
            .header("Authorization", authorization)
            .idempotent()
            .on_response(&|response, duration| {
                let result = upload_response_callback(response);
                if result.is_ok() {
                    if let Some(upload_logger) = &self.upload_logger {
                        let _ = upload_logger.log(
                            UploadLoggerRecordBuilder::default()
                                .response(response)
                                .duration(duration)
                                .up_type(UpType::CompleteParts)
                                .build(),
                        );
                    }
                }
                result
            })
            .on_error(&|host_url, err, duration| {
                if let Some(upload_logger) = &self.upload_logger {
                    let _ = upload_logger.log({
                        let mut builder = UploadLoggerRecordBuilder::default()
                            .duration(duration)
                            .up_type(UpType::CompleteParts)
                            .http_error(err);
                        if let Some(host_url) = host_url {
                            builder = builder.host(host_url);
                        }
                        builder.build()
                    });
                }
            })
            .accept_json()
            .json_body(&*completed_parts)
            .unwrap()
            .send()?
            .parse_json()?;
        Ok(value.into())
    }

    fn try_to_resume(&mut self, base_path: &str, authorization: &str) -> HTTPResult<Option<UploadResponse>> {
        if let Some(from_resuming) = self.from_resuming.take() {
            let init_uploaded_size = self.uploaded_size.load(Relaxed);
            if let Some(uploading_progress_callback) = &self.uploading_progress_callback {
                uploading_progress_callback
                    .completed_size
                    .store(init_uploaded_size, Relaxed);
            }
            let timer = Instant::now();
            self.start_uploading_blocks(
                &from_resuming
                    .up_urls
                    .iter()
                    .map(|url| url.as_ref())
                    .collect::<Box<[_]>>(),
                &(base_path.to_owned() + "/" + &from_resuming.upload_id),
                authorization,
                Some(from_resuming.recorder),
            )
            .map(|response| {
                if let Some(upload_logger) = &self.upload_logger {
                    let uploaded_size = self.uploaded_size.load(Relaxed);
                    let _ = upload_logger.log(
                        UploadLoggerRecordBuilder::default()
                            .duration(timer.elapsed())
                            .up_type(UpType::Chunkedv2)
                            .sent(uploaded_size - init_uploaded_size)
                            .total_size(uploaded_size - init_uploaded_size)
                            .build(),
                    );
                }
                response
            })
            .map_err(|err| {
                if let Some(upload_logger) = &self.upload_logger {
                    let uploaded_size = self.uploaded_size.load(Relaxed);
                    let mut record_builder = UploadLoggerRecordBuilder::default()
                        .duration(timer.elapsed())
                        .up_type(UpType::Chunkedv2)
                        .sent(uploaded_size - init_uploaded_size)
                        .http_error(&err);
                    if let Some(total_size) = self.io_size {
                        record_builder =
                            record_builder.total_size(u64::max(uploaded_size, total_size) - init_uploaded_size);
                    }
                    let _ = upload_logger.log(record_builder.build());
                }
                err
            })
            .map(Some)
        } else {
            Ok(None)
        }
    }

    fn make_base_path(&self) -> String {
        "/buckets/".to_owned()
            + self.bucket_uploader.bucket_name().as_ref()
            + "/objects/"
            + encode_key(self.key.as_ref().map(|key| key.as_ref())).as_ref()
            + "/uploads"
    }

    fn make_authorization(&self) -> Box<str> {
        ("UpToken ".to_owned() + self.upload_token.as_ref()).into()
    }
}

fn encode_key(key: Option<&str>) -> Cow<'static, str> {
    key.map_or_else(|| "~".into(), |key| base64::urlsafe(key.as_bytes()).into())
}

struct OptionalMd5 {
    enabled: bool,
}

impl OptionalMd5 {
    fn new(checksum_enabled: bool) -> OptionalMd5 {
        OptionalMd5 {
            enabled: checksum_enabled,
        }
    }

    fn hash(&mut self, buf: &[u8]) -> Option<String> {
        if self.enabled {
            Some(format!("{:x}", md5::compute(buf)))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{
            super::uploader::{UploadPolicyBuilder, UploadToken},
            BucketUploaderBuilder,
        },
        *,
    };
    use crate::{
        config::ConfigBuilder,
        credential::Credential,
        http::{DomainsManagerBuilder, Error as HTTPError, ErrorKind as HTTPErrorKind, Headers, Method},
    };
    use qiniu_http::ResponseBuilder;
    use qiniu_test_utils::{
        http_call_mock::{fake_req_id, CallHandlers, UploadingProgressErrorMock},
        temp_file::create_temp_file,
    };
    use serde_json::json;
    use std::{error::Error, result::Result};

    #[test]
    fn test_storage_uploader_resumable_uploader_upload_file() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_handler(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"uploadId":"test_upload_id"}).to_string())
                            .build())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id/"),
                        )
                        + "\\d"
                        + "$",
                    |request, called| {
                        if called >= 4 {
                            panic!("Unexpected call `PUT {}` for {} times", request.url(), called);
                        }
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "etag": format!("etag_{}", called) }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"hash": "abcdef", "key": "test-key"}).to_string())
                            .build())
                    },
                ),
            )
            .upload_logger(None)
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        let result = BucketUploaderBuilder::new(
            "test_bucket".into(),
            vec![vec![Box::from("http://z1h1.com")].into()].into(),
            config,
        )
        .build()
        .upload_token(UploadToken::new(policy, get_credential()))
        .key("test-key")
        .upload_file(&temp_path, "", None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_resumable_uploader_upload_file_with_recovering() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(5 * (1 << 22))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_handler(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |request, _| {
                        panic!("Unexpected call `POST {}`", request.url());
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id/2"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "etag": "etag_3" }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id/4"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "etag": "etag_4" }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"hash": "abcdef", "key": "test-key"}).to_string())
                            .build())
                    },
                ),
            )
            .upload_logger(None)
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        let bucket_uploader = BucketUploaderBuilder::new(
            "test_bucket".into(),
            vec![vec![Box::from("http://z1h1.com")].into()].into(),
            config,
        )
        .build();
        {
            let medium = bucket_uploader.recorder().open_and_write_metadata(
                &temp_path,
                Some("test-key"),
                "test_upload_id",
                &["http://z1h1.com"],
                1 << 22,
            )?;
            medium.append("etag_1", 1)?;
            medium.append("etag_3", 3)?;
            medium.append("etag_5", 5)?;
        }
        let result = bucket_uploader
            .upload_token(UploadToken::new(policy, get_credential()))
            .key("test-key")
            .upload_file(&temp_path, "", None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_resumable_uploader_upload_file_with_many_retryable_errors() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_handler(UploadingProgressErrorMock::new(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"uploadId":"test_upload_id"}).to_string())
                            .build())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id/"),
                        )
                        + "\\d"
                        + "$",
                    |request, called| {
                        if called >= 4 {
                            panic!("Unexpected call `PUT {}` for {} times", request.url(), called);
                        }
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "etag": format!("etag_{}", called) }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"hash": "abcdef", "key": "test-key"}).to_string())
                            .build())
                    },
                ),
                16384,
                0.5f64,
            ))
            .http_request_retries(100)
            .upload_logger(None)
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        let result = BucketUploaderBuilder::new(
            "test_bucket".into(),
            vec![vec![Box::from("http://z1h1.com")].into()].into(),
            config,
        )
        .build()
        .upload_token(UploadToken::new(policy, get_credential()))
        .key("test-key")
        .upload_file(&temp_path, "", None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_resumable_uploader_upload_file_with_1_host_failure() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_handler(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"uploadId":"test_upload_id"}).to_string())
                            .build())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id/"),
                        )
                        + "\\d"
                        + "$",
                    |_, called| {
                        if called >= 3 {
                            return Err(HTTPError::new_retryable_error_from_parts(
                                HTTPErrorKind::MaliciousResponse,
                                true,
                                None,
                                None,
                            ));
                        }
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "etag": format!("etag_{}", called) }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h2.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id/"),
                        )
                        + "\\d"
                        + "$",
                    |request, called| {
                        if called >= 2 {
                            panic!("Unexpected call `PUT {}` for {} times", request.url(), called);
                        }
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "etag": format!("etag_{}", called) }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h2.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"hash": "abcdef", "key": "test-key"}).to_string())
                            .build())
                    },
                ),
            )
            .upload_logger(None)
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        let result = BucketUploaderBuilder::new(
            "test_bucket".into(),
            vec![vec![Box::from("http://z1h1.com"), Box::from("http://z1h2.com")].into()].into(),
            config,
        )
        .build()
        .upload_token(UploadToken::new(policy, get_credential()))
        .key("test-key")
        .upload_file(&temp_path, "", None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_resumable_uploader_upload_file_with_1_zone_failure() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_handler(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        Err(HTTPError::new_zone_unretryable_error_from_parts(
                            HTTPErrorKind::MaliciousResponse,
                            true,
                            None,
                            None,
                        ))
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z2h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"uploadId":"test_upload_id"}).to_string())
                            .build())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z2h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id/"),
                        )
                        + "\\d"
                        + "$",
                    |request, called| {
                        if called >= 4 {
                            panic!("Unexpected call `PUT {}` for {} times", request.url(), called);
                        }
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "etag": format!("etag_{}", called) }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z2h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"hash": "abcdef", "key": "test-key"}).to_string())
                            .build())
                    },
                ),
            )
            .upload_logger(None)
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        let result = BucketUploaderBuilder::new(
            "test_bucket".into(),
            vec![
                vec![Box::from("http://z1h1.com"), Box::from("http://z1h2.com")].into(),
                vec![Box::from("http://z2h1.com"), Box::from("http://z2h2.com")].into(),
            ]
            .into(),
            config,
        )
        .build()
        .upload_token(UploadToken::new(policy, get_credential()))
        .key("test-key")
        .upload_file(&temp_path, "", None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_resumable_uploader_upload_file_with_1_continuous_zone_failure(
    ) -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_handler(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"uploadId":"test_upload_id_1"}).to_string())
                            .build())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z2h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"uploadId":"test_upload_id_2"}).to_string())
                            .build())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id_1/"),
                        )
                        + "\\d"
                        + "$",
                    |_, called| {
                        if called >= 3 {
                            return Err(HTTPError::new_retryable_error_from_parts(
                                HTTPErrorKind::MaliciousResponse,
                                true,
                                None,
                                None,
                            ));
                        }
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "etag": format!("etag_{}", called) }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z2h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id_2/"),
                        )
                        + "\\d"
                        + "$",
                    |request, called| {
                        if called >= 4 {
                            panic!("Unexpected call `PUT {}` for {} times", request.url(), called);
                        }
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "etag": format!("etag_{}", called) }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z2h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id_2"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"hash": "abcdef", "key": "test-key"}).to_string())
                            .build())
                    },
                ),
            )
            .upload_logger(None)
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        let result = BucketUploaderBuilder::new(
            "test_bucket".into(),
            vec![
                vec![Box::from("http://z1h1.com")].into(),
                vec![Box::from("http://z2h1.com")].into(),
            ]
            .into(),
            config,
        )
        .build()
        .upload_token(UploadToken::new(policy, get_credential()))
        .key("test-key")
        .upload_file(&temp_path, "", None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_resumable_uploader_upload_file_with_1_unretryable_failure() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_handler(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"uploadId": "test_upload_id"}).to_string())
                            .build())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id/"),
                        )
                        + "\\d"
                        + "$",
                    |request, called| {
                        if called == 3 {
                            return Err(HTTPError::new_unretryable_error_from_parts(
                                HTTPErrorKind::MaliciousResponse,
                                None,
                                None,
                            ));
                        } else if called >= 5 {
                            panic!("Unexpected call `PUT {}` for {} times", request.url(), called);
                        }
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "etag": format!("etag_{}", called) }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"hash": "abcdef", "key": "test-key"}).to_string())
                            .build())
                    },
                ),
            )
            .upload_logger(None)
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let upload_token = UploadToken::new(
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build(),
            get_credential(),
        );

        BucketUploaderBuilder::new(
            "test_bucket".into(),
            vec![vec![Box::from("http://z1h1.com")].into()].into(),
            config.clone(),
        )
        .build()
        .upload_token(upload_token.clone())
        .key("test-key")
        .upload_file(&temp_path, "", None)
        .unwrap_err();

        let result = BucketUploaderBuilder::new(
            "test_bucket".into(),
            vec![vec![Box::from("http://z1h1.com")].into()].into(),
            config,
        )
        .build()
        .upload_token(upload_token)
        .key("test-key")
        .upload_file(&temp_path, "", None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_resumable_uploader_upload_file_with_1_unretryable_failure_on_upload_id(
    ) -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_handler(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, called| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "uploadId": format!("test_upload_id_{}", called) }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id_1/"),
                        )
                        + "\\d"
                        + "$",
                    |_, called| {
                        if called >= 3 {
                            return Err(HTTPError::new_unretryable_error_from_parts(
                                HTTPErrorKind::MaliciousResponse,
                                None,
                                None,
                            ));
                        }
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "etag": format!("etag_{}", called) }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id_2/"),
                        )
                        + "\\d"
                        + "$",
                    |request, called| {
                        if called >= 4 {
                            panic!("Unexpected call `PUT {}` for {} times", request.url(), called);
                        }
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({ "etag": format!("etag_{}", called) }).to_string())
                            .build())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("http://z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id_2"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), "application/json".into());
                        headers.insert("X-Reqid".into(), fake_req_id().into());
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .bytes_as_body(json!({"hash": "abcdef", "key": "test-key"}).to_string())
                            .build())
                    },
                ),
            )
            .upload_logger(None)
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();

        let upload_token = UploadToken::new(
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build(),
            get_credential(),
        );

        BucketUploaderBuilder::new(
            "test_bucket".into(),
            vec![vec![Box::from("http://z1h1.com")].into()].into(),
            config.clone(),
        )
        .build()
        .upload_token(upload_token.clone())
        .key("test-key")
        .upload_file(&temp_path, "", None)
        .unwrap_err();

        let result = BucketUploaderBuilder::new(
            "test_bucket".into(),
            vec![vec![Box::from("http://z1h1.com")].into()].into(),
            config,
        )
        .build()
        .upload_token(upload_token)
        .key("test-key")
        .upload_file(&temp_path, "", None)?;

        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
