use super::{
    super::{
        upload_token::{Result as UploadTokenParseResult, UploadToken},
        UploadPolicy,
    },
    BucketUploader, UploadResponseCallback, UploadResult,
};
use crate::http::request::{Error as QiniuError, ErrorKind as QiniuErrorKind};
use crc::crc32::Hasher32;
use mime::Mime;
use multipart::client::lazy::Multipart;
use qiniu_http::{Error as HTTPError, ErrorKind as HTTPErrorKind, Method, Result as HTTPResult};
use std::{
    borrow::Cow,
    convert::TryInto,
    fs::File,
    io::{ErrorKind::Interrupted, Read, Result as IOResult, Seek, SeekFrom},
    path::Path,
};

pub(super) struct FormUploaderBuilder<'u> {
    bucket_uploader: &'u BucketUploader<'u>,
    multipart: Multipart<'u, 'u>,
    upload_policy: UploadPolicy<'u>,
}

pub(super) struct FormUploader<'u> {
    bucket_uploader: &'u BucketUploader<'u>,
    content_type: String,
    upload_policy: UploadPolicy<'u>,
    body: Vec<u8>,
}

impl<'u> FormUploaderBuilder<'u> {
    pub(super) fn new<T: Into<UploadToken<'u>>>(
        bucket_uploader: &'u BucketUploader<'u>,
        upload_token: T,
    ) -> UploadTokenParseResult<FormUploaderBuilder<'u>> {
        let upload_token = upload_token.into();
        let mut uploader = FormUploaderBuilder {
            bucket_uploader: bucket_uploader,
            multipart: Multipart::new(),
            upload_policy: upload_token.clone().policy()?,
        };
        uploader.multipart.add_text("token", upload_token.token());
        Ok(uploader)
    }

    pub(super) fn key<K: Into<Cow<'u, str>>>(mut self, key: K) -> FormUploaderBuilder<'u> {
        self.multipart.add_text("key", key);
        self
    }

    pub(super) fn var<K: AsRef<str>, V: Into<Cow<'u, str>>>(mut self, key: K, value: V) -> FormUploaderBuilder<'u> {
        self.multipart.add_text("x:".to_owned() + key.as_ref(), value);
        self
    }

    pub(super) fn metadata<K: AsRef<str>, V: Into<Cow<'u, str>>>(
        mut self,
        key: K,
        value: V,
    ) -> FormUploaderBuilder<'u> {
        self.multipart.add_text("x-qn-meta-".to_owned() + key.as_ref(), value);
        self
    }

    pub(super) fn file_path<'n: 'u, P: AsRef<Path>, N: Into<Cow<'n, str>>>(
        mut self,
        file_path: P,
        file_name: Option<N>,
        mime: Option<Mime>,
        crc32_check_enabled: bool,
    ) -> IOResult<FormUploader<'u>> {
        let mut file = File::open(file_path.as_ref())?;
        let mut crc32: Option<u32> = None;
        if crc32_check_enabled {
            crc32 = Some(Self::calc_crc32(&mut file)?);
            file.seek(SeekFrom::Start(0))?;
        }
        let file_name: Cow<str> = file_name.map(|name| name.into()).unwrap_or_else(|| {
            file_path
                .as_ref()
                .file_name()
                .and_then(|name| name.to_str().map(|name| name.to_owned().into()))
                .unwrap_or_else(|| "fileName".into())
        });
        let mime = mime.or_else(|| mime_guess::from_path(file_path.as_ref()).first());
        self.multipart.add_stream("file", file, Some(file_name), mime);
        if let Some(crc32) = crc32 {
            self.multipart.add_text("crc32", crc32.to_string());
        }

        let mut fields = self.multipart.prepare().map_err(|err| err.error)?;
        let mut body = Vec::with_capacity(
            self.bucket_uploader
                .config()
                .upload_threshold()
                .try_into()
                .unwrap_or(1 << 22),
        );
        fields.read_to_end(&mut body)?;
        Ok(FormUploader {
            bucket_uploader: self.bucket_uploader,
            upload_policy: self.upload_policy,
            content_type: "multipart/form-data; boundary=".to_owned() + fields.boundary(),
            body: body,
        })
    }

    // TODO:
    // pub(super) fn with_stream<R: Read, N: Into<Cow<'u, str>>>(
    //     mut self,
    //     stream: R,
    //     file_name: Option<N>,
    //     mime: Option<Mime>,
    // ) -> IOResult<FormUploader<'u>> {
    // }

    fn calc_crc32(file: &mut File) -> IOResult<u32> {
        const BUF_SIZE: usize = 1 << 22;
        let mut digest = crc::crc32::Digest::new(crc::crc32::IEEE);
        let mut buf = vec![0; BUF_SIZE];
        loop {
            let have_read = match file.read(&mut buf) {
                Ok(have_read) => have_read,
                Err(err) => {
                    if err.kind() == Interrupted {
                        continue;
                    } else {
                        return Err(err);
                    }
                }
            };
            if have_read == 0 {
                break;
            }
            digest.write(&buf[..have_read]);
        }
        Ok(digest.sum32())
    }
}

impl<'u> FormUploader<'u> {
    pub(super) fn send(&self) -> HTTPResult<UploadResult> {
        let mut iter = self.bucket_uploader.up_urls_list().iter();
        let mut prev_err: Option<HTTPError> = None;
        while let Some(up_urls) = iter.next() {
            match self
                .bucket_uploader
                .client()
                .post("/", &up_urls.iter().map(|url| url.as_ref()).collect::<Vec<&str>>())
                .idempotent()
                .response_callback(&UploadResponseCallback(&self.upload_policy))
                .accept_json()
                .raw_body(self.content_type.to_owned(), self.body.as_slice())
                .send()
            {
                Ok(mut response) => {
                    let value: serde_json::Value = response.parse_json()?;
                    return Ok(value.into());
                }
                Err(err) => match err.kind() {
                    HTTPErrorKind::RetryableError
                    | HTTPErrorKind::HostUnretryableError
                    | HTTPErrorKind::ZoneUnretryableError => {
                        prev_err = Some(err);
                        continue;
                    }
                    _ => {
                        return Err(err);
                    }
                },
            }
        }

        Err(prev_err.unwrap_or_else(|| {
            HTTPError::new_host_unretryable_error_from_parts(
                QiniuError::from(QiniuErrorKind::NoHostAvailable),
                true,
                Some(Method::POST),
                None,
            )
        }))
    }
}
