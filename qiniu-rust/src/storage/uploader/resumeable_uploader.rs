use super::{
    super::{
        recorder,
        upload_token::{Result as UploadTokenParseResult, UploadToken},
    },
    upload_recorder, BucketUploader, UploadResponseCallback, UploadResult,
};
use crate::utils::{base64, seek_adapter};
use crypto::{digest::Digest, md5::Md5};
use mime::Mime;
use qiniu_http::{Error as HTTPError, ErrorKind as HTTPErrorKind, Result as HTTPResult, RetryKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    borrow::Cow,
    boxed::Box,
    collections::HashMap,
    convert::TryInto,
    fs::File,
    io::{ErrorKind as IOErrorKind, Read, Result as IOResult, Seek, SeekFrom},
    path::Path,
};

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

struct FromResuming<REC: recorder::Recorder> {
    upload_id: Box<str>,
    up_urls: Box<[Box<str>]>,
    recorder: upload_recorder::FileUploadRecorder<REC::Medium>,
    io_offset: u64,
}

pub(super) struct ResumeableUploaderBuilder<'u, REC: recorder::Recorder> {
    bucket_uploader: &'u BucketUploader<'u, REC>,
    upload_token: Cow<'u, str>,
    key: Option<Cow<'u, str>>,
    metadata: Option<HashMap<Cow<'u, str>, Cow<'u, str>>>,
    custom_vars: Option<HashMap<Cow<'u, str>, Cow<'u, str>>>,
}

pub(super) struct ResumeableUploader<'u, R: Read + Seek + 'u, REC: recorder::Recorder> {
    bucket_uploader: &'u BucketUploader<'u, REC>,
    upload_token: Cow<'u, str>,
    key: Option<Cow<'u, str>>,
    completed_parts: CompletedParts<'u>,
    checksum_enabled: bool,
    is_seekable: bool,
    block_size: usize,
    io: R,
    file_path: Option<Cow<'u, Path>>,
    from_resuming: Option<FromResuming<REC>>,
}

impl<'u, REC: recorder::Recorder> ResumeableUploaderBuilder<'u, REC> {
    pub(super) fn new(
        bucket_uploader: &'u BucketUploader<'u, REC>,
        upload_token: &'u UploadToken<'u>,
    ) -> UploadTokenParseResult<ResumeableUploaderBuilder<'u, REC>> {
        Ok(ResumeableUploaderBuilder {
            bucket_uploader: bucket_uploader,
            upload_token: upload_token.token(),
            key: None,
            metadata: None,
            custom_vars: None,
        })
    }

    pub(super) fn key<K: Into<Cow<'u, str>>>(mut self, key: K) -> ResumeableUploaderBuilder<'u, REC> {
        self.key = Some(key.into());
        self
    }

    pub(super) fn metadata(
        mut self,
        metadata: HashMap<Cow<'u, str>, Cow<'u, str>>,
    ) -> ResumeableUploaderBuilder<'u, REC> {
        self.metadata = Some(metadata);
        self
    }

    pub(super) fn vars(mut self, vars: HashMap<Cow<'u, str>, Cow<'u, str>>) -> ResumeableUploaderBuilder<'u, REC> {
        let mut hashmap = HashMap::new();
        for (k, v) in vars.into_iter() {
            hashmap.insert(Cow::Owned("x:".to_owned() + &k), v);
        }
        self.custom_vars = Some(hashmap);
        self
    }

    pub(super) fn file<'n: 'u, P: Into<Cow<'n, Path>>, N: Into<Cow<'n, str>>>(
        self,
        file: File,
        file_path: P,
        file_name: Option<N>,
        file_size: u64,
        mime_type: Option<Mime>,
        checksum_enabled: bool,
    ) -> ResumeableUploader<'u, File, REC> {
        let block_size = self.bucket_uploader.config().upload_block_size();
        ResumeableUploader {
            bucket_uploader: self.bucket_uploader,
            upload_token: self.upload_token,
            key: self.key,
            file_path: Some(file_path.into()),
            io: file,
            checksum_enabled: checksum_enabled,
            is_seekable: true,
            block_size: self.bucket_uploader.config().upload_block_size(),
            completed_parts: CompletedParts {
                parts: Vec::with_capacity(
                    ((file_size + block_size as u64 - 1) / (block_size as u64))
                        .try_into()
                        .unwrap_or_else(|_| usize::max_value()),
                ),
                fname: file_name.map(|name| name.into()),
                mime_type: mime_type.map(|m| m.as_ref().into()),
                metadata: self.metadata,
                custom_vars: self.custom_vars,
            },
            from_resuming: None,
        }
    }

    pub(super) fn stream<'n: 'u, R: Read + 'u, N: Into<Cow<'n, str>>>(
        self,
        stream: R,
        mime_type: Option<Mime>,
        file_name: Option<N>,
        checksum_enabled: bool,
    ) -> ResumeableUploader<'u, seek_adapter::SeekAdapter<R>, REC> {
        ResumeableUploader {
            bucket_uploader: self.bucket_uploader,
            upload_token: self.upload_token,
            key: self.key,
            file_path: None,
            io: seek_adapter::SeekAdapter(stream),
            checksum_enabled: checksum_enabled,
            is_seekable: false,
            block_size: self.bucket_uploader.config().upload_block_size(),
            completed_parts: CompletedParts {
                parts: Vec::new(),
                fname: file_name.map(|name| name.into()),
                mime_type: mime_type.map(|m| m.as_ref().into()),
                metadata: self.metadata,
                custom_vars: self.custom_vars,
            },
            from_resuming: None,
        }
    }
}

impl<'u, R: Read + Seek, REC: recorder::Recorder> ResumeableUploader<'u, R, REC> {
    pub(super) fn send(&mut self) -> HTTPResult<UploadResult> {
        let base_path = self.make_base_path();
        let authorization = self.make_authorization();
        let mut body_buf = vec![0; self.block_size];
        let mut md5_digest = None;
        if self.checksum_enabled {
            md5_digest = Some(Md5::new());
        }
        if let Ok(Some(result)) = self.try_to_resume(&base_path, &authorization, &mut body_buf, &mut md5_digest) {
            return Ok(result);
        }
        let mut iter = self.bucket_uploader.up_urls_list().iter();
        let mut prev_err: Option<HTTPError> = None;
        while let Some(up_urls) = iter.next() {
            match self.try_to_upload(
                &up_urls.iter().map(|url| url.as_ref()).collect::<Box<[&str]>>(),
                &base_path,
                &authorization,
                &mut body_buf,
                &mut md5_digest,
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

        Err(prev_err.expect("ResumeableUploader::send() should try at lease once, but not"))
    }

    fn try_to_upload(
        &mut self,
        up_urls: &[&str],
        base_path: &str,
        authorization: &str,
        body_buf: &mut Vec<u8>,
        md5_digest: &mut Option<Md5>,
    ) -> HTTPResult<UploadResult> {
        if self.is_seekable {
            self.io
                .seek(SeekFrom::Start(0))
                .map_err(|err| HTTPError::new_unretryable_error_from_parts(HTTPErrorKind::IOError(err), None, None))?;
        }
        let upload_id = self.init_parts(&base_path, up_urls, &authorization)?;
        self.completed_parts.parts.clear();
        let recorder = if let Some(file_path) = self.file_path.as_ref() {
            self.bucket_uploader
                .recorder()
                .open_and_write_metadata(
                    file_path,
                    self.key.as_ref().map(|key| key.as_ref()),
                    &upload_id,
                    up_urls,
                )
                .ok()
        } else {
            None
        };
        self.start_uploading_blocks(
            &upload_id,
            0,
            up_urls,
            base_path,
            authorization,
            body_buf,
            md5_digest,
            recorder,
        )
    }

    fn start_uploading_blocks(
        &mut self,
        upload_id: &str,
        mut part_number: usize,
        up_urls: &[&str],
        base_path: &str,
        authorization: &str,
        body_buf: &mut Vec<u8>,
        md5_digest: &mut Option<Md5>,
        mut recorder: Option<upload_recorder::FileUploadRecorder<REC::Medium>>,
    ) -> HTTPResult<UploadResult> {
        loop {
            part_number += 1;
            let block_size = self
                .read_block(body_buf)
                .map_err(|err| HTTPError::new_unretryable_error_from_parts(HTTPErrorKind::IOError(err), None, None))?;
            if block_size == 0 {
                break;
            }
            let etag = self.upload_part(
                base_path,
                up_urls,
                authorization,
                upload_id,
                part_number,
                &body_buf[..block_size],
                if let Some(md5_digest) = md5_digest.as_mut() {
                    md5_digest.input(&body_buf[..block_size]);
                    let md5 = Some(md5_digest.result_str());
                    md5_digest.reset();
                    md5
                } else {
                    None
                },
            )?;
            if let Some(recorder) = &mut recorder {
                recorder
                    .append_record(&etag, part_number, block_size.try_into().unwrap())
                    .map_err(|err| {
                        HTTPError::new_unretryable_error_from_parts(HTTPErrorKind::IOError(err), None, None)
                    })?;
            }
            self.completed_parts.parts.push(Part {
                etag: etag,
                part_number: part_number,
            });
        }
        self.complete_parts(base_path, up_urls, authorization, upload_id)
            .map(|result| {
                if let Some(file_path) = self.file_path.as_ref() {
                    self.bucket_uploader
                        .recorder()
                        .drop_record(file_path, self.key.as_ref().map(|key| key.as_ref()))
                        .ok();
                }
                result
            })
    }

    pub(super) fn prepare_for_resuming(
        &mut self,
        file_record: upload_recorder::FileRecord,
        block_records: Box<[upload_recorder::FileBlockRecord]>,
        recorder: upload_recorder::FileUploadRecorder<REC::Medium>,
    ) {
        let mut io_offset = 0;
        let block_records: Vec<upload_recorder::FileBlockRecord> = block_records.into();
        for block_record in block_records {
            self.completed_parts.parts.push(Part {
                etag: block_record.etag,
                part_number: block_record.part_number,
            });
            io_offset += block_record.block_size;
        }
        self.from_resuming = Some(FromResuming {
            upload_id: file_record.upload_id,
            up_urls: file_record.up_urls,
            recorder: recorder,
            io_offset: io_offset,
        });
    }

    fn read_block(&mut self, buf: &mut Vec<u8>) -> IOResult<usize> {
        let mut have_read = 0;
        loop {
            match self.io.read(&mut buf[have_read..]) {
                Ok(0) => {
                    break;
                }
                Ok(n) => {
                    have_read += n;
                    if have_read == buf.len() {
                        break;
                    }
                }
                Err(ref err) if err.kind() == IOErrorKind::Interrupted => {
                    continue;
                }
                Err(err) => {
                    return Err(err);
                }
            };
        }
        Ok(have_read)
    }

    fn init_parts(&self, base_path: &str, up_urls: &[&str], authorization: &str) -> HTTPResult<Box<str>> {
        let result: InitPartsResult = self
            .bucket_uploader
            .client()
            .post(base_path, up_urls)
            .header("Authorization", authorization)
            .idempotent()
            .response_callback(&UploadResponseCallback)
            .accept_json()
            .no_body()
            .send()?
            .parse_json()?;
        Ok(result.upload_id)
    }

    fn upload_part(
        &self,
        base_path: &str,
        up_urls: &[&str],
        authorization: &str,
        upload_id: &str,
        part_number: usize,
        part: &[u8],
        md5: Option<String>,
    ) -> HTTPResult<Box<str>> {
        let path = base_path.to_owned() + "/" + upload_id + "/" + &part_number.to_string();
        let mut builder = self
            .bucket_uploader
            .client()
            .put(&path, up_urls)
            .header("Authorization", authorization);
        if let Some(md5) = md5 {
            builder = builder.header("Content-MD5", md5);
        }
        let result: UploadPartResult = builder
            .idempotent()
            .response_callback(&UploadResponseCallback)
            .accept_json()
            .raw_body("application/octet-stream", part.as_ref())
            .send()?
            .parse_json()?;
        Ok(result.etag)
    }

    fn complete_parts(
        &self,
        base_path: &str,
        up_urls: &[&str],
        authorization: &str,
        upload_id: &str,
    ) -> HTTPResult<UploadResult> {
        let path = base_path.to_owned() + "/" + upload_id;
        let value: Value = self
            .bucket_uploader
            .client()
            .post(&path, up_urls)
            .header("Authorization", authorization)
            .idempotent()
            .response_callback(&UploadResponseCallback)
            .accept_json()
            .json_body(&self.completed_parts)
            .unwrap()
            .send()?
            .parse_json()?;
        Ok(value.into())
    }

    fn try_to_resume(
        &mut self,
        base_path: &str,
        authorization: &str,
        body_buf: &mut Vec<u8>,
        md5_digest: &mut Option<Md5>,
    ) -> HTTPResult<Option<UploadResult>> {
        if let Some(from_resuming) = self.from_resuming.take() {
            self.io
                .seek(SeekFrom::Start(from_resuming.io_offset))
                .map_err(|err| HTTPError::new_unretryable_error_from_parts(HTTPErrorKind::IOError(err), None, None))?;
            self.start_uploading_blocks(
                &from_resuming.upload_id,
                self.completed_parts.parts.len(),
                &from_resuming
                    .up_urls
                    .iter()
                    .map(|url| url.as_ref())
                    .collect::<Box<[&str]>>(),
                base_path,
                authorization,
                body_buf,
                md5_digest,
                Some(from_resuming.recorder),
            )
            .map(|result| Some(result))
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
    key.map_or_else(|| Cow::Borrowed("~"), |key| base64::urlsafe(key.as_bytes()).into())
}

#[cfg(test)]
mod tests {
    use super::{super::super::upload_policy::UploadPolicyBuilder, *};
    use crate::{config::ConfigBuilder, credential::Credential};
    use qiniu_http::{Error as HTTPError, ErrorKind as HTTPErrorKind, Headers, Method, ResponseBuilder};
    use qiniu_test_utils::{http_call_mock::CallHandlers, temp_file::create_temp_file};
    use serde_json::json;
    use std::{boxed::Box, error::Error, io::Cursor, result::Result};

    #[test]
    fn test_storage_uploader_resumeable_uploader_upload_file() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_call(Box::new(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({"uploadId":"test_upload_id"}).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
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
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({ "etag": format!("etag_{}", called) }).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({"hash": "abcdef", "key": "test-key"}).to_string()))
                            .build()
                            .unwrap())
                    },
                ),
            ))
            .build()?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        let result = BucketUploader::new(
            "test_bucket",
            vec![vec![Box::from("z1h1.com")].into()],
            get_credential(),
            config,
        )
        .upload_token(UploadToken::from_policy(policy, get_credential()))
        .key("test-key")
        .upload_file(&temp_path, Some("file"), None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_resumeable_uploader_upload_file_with_1_host_failure() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_call(Box::new(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({"uploadId":"test_upload_id"}).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
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
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({ "etag": format!("etag_{}", called) }).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h2.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id/3"),
                        )
                        + "$",
                    |request, called| {
                        if called >= 2 {
                            panic!("Unexpected call `PUT {}` for {} times", request.url(), called);
                        }
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({ "etag": format!("etag_{}", called) }).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h2.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({"hash": "abcdef", "key": "test-key"}).to_string()))
                            .build()
                            .unwrap())
                    },
                ),
            ))
            .build()?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        let result = BucketUploader::new(
            "test_bucket",
            vec![vec![Box::from("z1h1.com"), Box::from("z1h2.com")].into()],
            get_credential(),
            config,
        )
        .upload_token(UploadToken::from_policy(policy, get_credential()))
        .key("test-key")
        .upload_file(&temp_path, Some("file"), None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_resumeable_uploader_upload_file_with_1_zone_failure() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_call(Box::new(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
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
                            &("z2h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({"uploadId":"test_upload_id"}).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("z2h1.com/buckets/test_bucket/objects/".to_owned()
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
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({ "etag": format!("etag_{}", called) }).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z2h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({"hash": "abcdef", "key": "test-key"}).to_string()))
                            .build()
                            .unwrap())
                    },
                ),
            ))
            .build()?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        let result = BucketUploader::new(
            "test_bucket",
            vec![
                vec![Box::from("z1h1.com"), Box::from("z1h2.com")].into(),
                vec![Box::from("z2h1.com"), Box::from("z2h2.com")].into(),
            ],
            get_credential(),
            config,
        )
        .upload_token(UploadToken::from_policy(policy, get_credential()))
        .key("test-key")
        .upload_file(&temp_path, Some("file"), None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_resumeable_uploader_upload_file_with_1_continuous_zone_failure(
    ) -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_call(Box::new(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({"uploadId":"test_upload_id_1"}).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z2h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({"uploadId":"test_upload_id_2"}).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
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
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({ "etag": format!("etag_{}", called) }).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("z2h1.com/buckets/test_bucket/objects/".to_owned()
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
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({ "etag": format!("etag_{}", called) }).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z2h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id_2"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({"hash": "abcdef", "key": "test-key"}).to_string()))
                            .build()
                            .unwrap())
                    },
                ),
            ))
            .build()?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        let result = BucketUploader::new(
            "test_bucket",
            vec![vec![Box::from("z1h1.com")].into(), vec![Box::from("z2h1.com")].into()],
            get_credential(),
            config,
        )
        .upload_token(UploadToken::from_policy(policy, get_credential()))
        .key("test-key")
        .upload_file(&temp_path, Some("file"), None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_resumeable_uploader_upload_file_with_1_unretryable_failure() -> Result<(), Box<dyn Error>>
    {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_call(Box::new(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({"uploadId": "test_upload_id"}).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
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
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({ "etag": format!("etag_{}", called) }).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({"hash": "abcdef", "key": "test-key"}).to_string()))
                            .build()
                            .unwrap())
                    },
                ),
            ))
            .build()?;
        let upload_token = UploadToken::from_policy(
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build(),
            get_credential(),
        );
        BucketUploader::new(
            "test_bucket",
            vec![vec![Box::from("z1h1.com")].into()],
            get_credential(),
            config.clone(),
        )
        .upload_token(upload_token.clone())
        .key("test-key")
        .upload_file(&temp_path, Some("file"), None)
        .unwrap_err();
        let result = BucketUploader::new(
            "test_bucket",
            vec![vec![Box::from("z1h1.com")].into()],
            get_credential(),
            config,
        )
        .upload_token(upload_token)
        .key("test-key")
        .upload_file(&temp_path, Some("file"), None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_resumeable_uploader_upload_file_with_1_unretryable_failure_on_upload_id(
    ) -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(10 * (1 << 20))?.into_temp_path();
        let config = ConfigBuilder::default()
            .http_request_call(Box::new(
                CallHandlers::new(|request| {
                    panic!("Unexpected Request: {} {}", request.method(), request.url());
                })
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads"),
                        )
                        + "$",
                    |_, called| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(
                                json!({ "uploadId": format!("test_upload_id_{}", called) }).to_string(),
                            ))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
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
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({ "etag": format!("etag_{}", called) }).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::PUT,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
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
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({ "etag": format!("etag_{}", called) }).to_string()))
                            .build()
                            .unwrap())
                    },
                )
                .install(
                    Method::POST,
                    "^".to_owned()
                        + &regex::escape(
                            &("z1h1.com/buckets/test_bucket/objects/".to_owned()
                                + &encode_key(Some("test-key"))
                                + "/uploads/test_upload_id_2"),
                        )
                        + "$",
                    |_, _| {
                        let mut headers = Headers::new();
                        headers.insert(Cow::Borrowed("Content-Type"), Cow::Borrowed("application/json"));
                        Ok(ResponseBuilder::default()
                            .status_code(200u16)
                            .headers(headers)
                            .stream(Cursor::new(json!({"hash": "abcdef", "key": "test-key"}).to_string()))
                            .build()
                            .unwrap())
                    },
                ),
            ))
            .build()?;
        let upload_token = UploadToken::from_policy(
            UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build(),
            get_credential(),
        );
        BucketUploader::new(
            "test_bucket",
            vec![vec![Box::from("z1h1.com")].into()],
            get_credential(),
            config.clone(),
        )
        .upload_token(upload_token.clone())
        .key("test-key")
        .upload_file(&temp_path, Some("file"), None)
        .unwrap_err();

        let result = BucketUploader::new(
            "test_bucket",
            vec![vec![Box::from("z1h1.com")].into()],
            get_credential(),
            config,
        )
        .upload_token(upload_token)
        .key("test-key")
        .upload_file(&temp_path, Some("file"), None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
