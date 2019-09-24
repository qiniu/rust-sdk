use super::{
    super::{
        upload_policy::UploadPolicy,
        upload_token::{Result as UploadTokenParseResult, UploadToken},
    },
    BucketUploader, UploadResponseCallback, UploadResult,
};
use crate::utils::{base64, seek_adapter};
use crypto::{digest::Digest, md5::Md5};
use mime::Mime;
use qiniu_http::{Error as HTTPError, ErrorKind as HTTPErrorKind, Method, Result as HTTPResult, RetryKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    borrow::Cow,
    boxed::Box,
    collections::HashMap,
    convert::TryInto,
    io::{ErrorKind as IOErrorKind, Read, Result as IOResult, Seek, SeekFrom},
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
    fname: Cow<'f, str>,
    mime_type: Option<Box<str>>,
    metadata: Option<HashMap<Cow<'f, str>, Cow<'f, str>>>,
    custom_vars: Option<HashMap<Cow<'f, str>, Cow<'f, str>>>,
}

pub(super) struct ResumeableUploaderBuilder<'u> {
    bucket_uploader: &'u BucketUploader<'u>,
    upload_policy: UploadPolicy<'u>,
    upload_token: Box<str>,
    key: Option<Cow<'u, str>>,
    fname: Option<Cow<'u, str>>,
    mime_type: Option<Mime>,
    metadata: Option<HashMap<Cow<'u, str>, Cow<'u, str>>>,
    custom_vars: Option<HashMap<Cow<'u, str>, Cow<'u, str>>>,
}

pub(super) struct ResumeableUploader<'u, R: Read + Seek + 'u> {
    bucket_uploader: &'u BucketUploader<'u>,
    upload_policy: UploadPolicy<'u>,
    upload_token: Box<str>,
    key: Option<Cow<'u, str>>,
    completed_parts: CompletedParts<'u>,
    checksum_enabled: bool,
    multi_zones_retry: bool,
    chunk_size: usize,
    io: R,
}

impl<'u> ResumeableUploaderBuilder<'u> {
    pub(super) fn new<T: Into<UploadToken<'u>>>(
        bucket_uploader: &'u BucketUploader<'u>,
        upload_token: T,
    ) -> UploadTokenParseResult<ResumeableUploaderBuilder<'u>> {
        let upload_token = upload_token.into();
        Ok(ResumeableUploaderBuilder {
            bucket_uploader: bucket_uploader,
            upload_policy: upload_token.clone().policy()?,
            upload_token: upload_token.token().as_ref().into(),
            key: None,
            fname: None,
            mime_type: None,
            metadata: None,
            custom_vars: None,
        })
    }

    pub(super) fn key<K: Into<Cow<'u, str>>>(mut self, key: K) -> ResumeableUploaderBuilder<'u> {
        self.key = Some(key.into());
        self
    }

    pub(super) fn metadata(mut self, metadata: HashMap<Cow<'u, str>, Cow<'u, str>>) -> ResumeableUploaderBuilder<'u> {
        self.metadata = Some(metadata);
        self
    }

    pub(super) fn vars(mut self, vars: HashMap<Cow<'u, str>, Cow<'u, str>>) -> ResumeableUploaderBuilder<'u> {
        let mut hashmap = HashMap::new();
        for (k, v) in vars.into_iter() {
            hashmap.insert(Cow::Owned("x:".to_owned() + &k), v);
        }
        self.custom_vars = Some(hashmap);
        self
    }

    pub(super) fn seekable_stream<'n: 'u, R: Read + Seek + 'u, N: Into<Cow<'n, str>>>(
        self,
        stream: R,
        file_name: N,
        file_size: u64,
        mime_type: Option<Mime>,
        checksum_enabled: bool,
    ) -> IOResult<ResumeableUploader<'u, R>> {
        let chunk_size = self.bucket_uploader.config().upload_chunk_size();
        Ok(ResumeableUploader {
            bucket_uploader: self.bucket_uploader,
            upload_policy: self.upload_policy,
            upload_token: self.upload_token,
            key: self.key,
            io: stream,
            checksum_enabled: checksum_enabled,
            multi_zones_retry: true,
            chunk_size: self.bucket_uploader.config().upload_chunk_size(),
            completed_parts: CompletedParts {
                parts: Vec::with_capacity(
                    ((file_size + chunk_size as u64 - 1) / (chunk_size as u64))
                        .try_into()
                        .unwrap_or_else(|_| usize::max_value()),
                ),
                fname: file_name.into(),
                mime_type: mime_type.map(|m| m.as_ref().into()),
                metadata: self.metadata,
                custom_vars: self.custom_vars,
            },
        })
    }

    pub(super) fn stream<'n: 'u, R: Read + 'u, N: Into<Cow<'n, str>>>(
        self,
        stream: R,
        mime_type: Option<Mime>,
        file_name: N,
        checksum_enabled: bool,
    ) -> IOResult<ResumeableUploader<'u, seek_adapter::SeekAdapter<R>>> {
        Ok(ResumeableUploader {
            bucket_uploader: self.bucket_uploader,
            upload_policy: self.upload_policy,
            upload_token: self.upload_token,
            key: self.key,
            io: seek_adapter::SeekAdapter(stream),
            checksum_enabled: checksum_enabled,
            multi_zones_retry: false,
            chunk_size: self.bucket_uploader.config().upload_chunk_size(),
            completed_parts: CompletedParts {
                parts: Vec::new(),
                fname: file_name.into(),
                mime_type: mime_type.map(|m| m.as_ref().into()),
                metadata: self.metadata,
                custom_vars: self.custom_vars,
            },
        })
    }
}

impl<'u, R: Read + Seek> ResumeableUploader<'u, R> {
    pub(super) fn send(&mut self) -> HTTPResult<UploadResult> {
        let mut iter = self.bucket_uploader.up_urls_list().iter();
        let mut prev_err: Option<HTTPError> = None;
        while let Some(up_urls) = iter.next() {
            match self.send_requests(&up_urls.iter().map(|url| url.as_ref()).collect::<Vec<&str>>()) {
                Ok(response) => {
                    return Ok(response);
                }
                Err(err) => match err.retry_kind() {
                    RetryKind::RetryableError | RetryKind::HostUnretryableError | RetryKind::ZoneUnretryableError => {
                        if self.multi_zones_retry {
                            self.io.seek(SeekFrom::Start(0)).map_err(|err| {
                                HTTPError::new_unretryable_error_from_parts(HTTPErrorKind::IOError(err), None, None)
                            })?;
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

        Err(prev_err.unwrap_or_else(|| {
            HTTPError::new_host_unretryable_error_from_parts(
                HTTPErrorKind::NoHostAvailable,
                true,
                Some(Method::POST),
                None,
            )
        }))
    }

    pub(super) fn send_requests(&mut self, up_urls: &[&str]) -> HTTPResult<UploadResult> {
        let base_path = "/buckets/".to_owned()
            + self.bucket_uploader.bucket_name().as_ref()
            + "/objects/"
            + Self::encode_key(self.key.as_ref()).as_ref()
            + "/uploads";
        let authorization = "UpToken ".to_owned() + self.upload_token.as_ref();
        let upload_id = self.init_parts(&base_path, up_urls, &authorization)?;
        let mut body_buf = vec![0; self.chunk_size];
        let mut part_number = 0;
        let mut md5_digest = None;
        if self.checksum_enabled {
            md5_digest = Some(Md5::new());
        }

        loop {
            part_number += 1;
            let chunk_size = self
                .read_chunk(&mut body_buf)
                .map_err(|err| HTTPError::new_unretryable_error_from_parts(HTTPErrorKind::IOError(err), None, None))?;
            if chunk_size == 0 {
                break;
            }
            let etag = self.upload_part(
                &base_path,
                up_urls,
                &authorization,
                &upload_id,
                part_number,
                &body_buf[..chunk_size],
                if let Some(md5_digest) = md5_digest.as_mut() {
                    md5_digest.input(&body_buf[..chunk_size]);
                    let md5 = Some(md5_digest.result_str());
                    md5_digest.reset();
                    md5
                } else {
                    None
                },
            )?;
            self.completed_parts.parts.push(Part {
                etag: etag,
                part_number: part_number,
            });
        }
        self.complete_parts(&base_path, up_urls, &authorization, &upload_id)
    }

    fn read_chunk(&mut self, buf: &mut Vec<u8>) -> IOResult<usize> {
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

    fn encode_key(key: Option<&Cow<str>>) -> Cow<'static, str> {
        key.map_or_else(|| Cow::Borrowed("~"), |k| base64::urlsafe(k.as_ref().as_bytes()).into())
    }

    fn init_parts(&self, base_path: &str, up_urls: &[&str], authorization: &str) -> HTTPResult<Box<str>> {
        let result: InitPartsResult = self
            .bucket_uploader
            .client()
            .post(base_path, up_urls)
            .header("Authorization", authorization)
            .idempotent()
            .response_callback(&UploadResponseCallback(&self.upload_policy))
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
        chunk: &[u8],
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
            .response_callback(&UploadResponseCallback(&self.upload_policy))
            .accept_json()
            .raw_body("application/octet-stream", chunk.as_ref())
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
            .response_callback(&UploadResponseCallback(&self.upload_policy))
            .accept_json()
            .json_body(&self.completed_parts)
            .unwrap()
            .send()?
            .parse_json()?;
        Ok(value.into())
    }
}
