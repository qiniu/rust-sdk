use super::{
    super::{
        recorder,
        upload_token::{Result as UploadTokenParseResult, UploadToken},
    },
    upload_response_callback, BucketUploader, UploadResult,
};
use crate::utils::crc32;
use mime::Mime;
use multipart::client::lazy::Multipart;
use qiniu_http::{Error as HTTPError, Result as HTTPResult, RetryKind};
use std::{
    borrow::Cow,
    convert::TryInto,
    io::{Read, Result as IOResult, Seek, SeekFrom},
};

pub(super) struct FormUploaderBuilder<'u, REC: recorder::Recorder> {
    bucket_uploader: &'u BucketUploader<'u, REC>,
    multipart: Multipart<'u, 'u>,
    on_uploading_progress: Option<&'u dyn Fn(usize, usize)>,
}

pub(super) struct FormUploader<'u, REC: recorder::Recorder> {
    bucket_uploader: &'u BucketUploader<'u, REC>,
    content_type: String,
    body: Vec<u8>,
    on_uploading_progress: Option<&'u dyn Fn(usize, usize)>,
}

impl<'u, REC> FormUploaderBuilder<'u, REC>
where
    REC: recorder::Recorder,
{
    pub(super) fn new(
        bucket_uploader: &'u BucketUploader<'u, REC>,
        upload_token: &'u UploadToken<'u>,
    ) -> UploadTokenParseResult<FormUploaderBuilder<'u, REC>> {
        let mut uploader = FormUploaderBuilder {
            bucket_uploader: bucket_uploader,
            multipart: Multipart::new(),
            on_uploading_progress: None,
        };
        uploader.multipart.add_text("token", upload_token.token());
        Ok(uploader)
    }

    pub(super) fn key<K: Into<Cow<'u, str>>>(mut self, key: K) -> FormUploaderBuilder<'u, REC> {
        self.multipart.add_text("key", key);
        self
    }

    pub(super) fn var<K: AsRef<str>, V: Into<Cow<'u, str>>>(
        mut self,
        key: K,
        value: V,
    ) -> FormUploaderBuilder<'u, REC> {
        self.multipart.add_text("x:".to_owned() + key.as_ref(), value);
        self
    }

    pub(super) fn metadata<K: AsRef<str>, V: Into<Cow<'u, str>>>(
        mut self,
        key: K,
        value: V,
    ) -> FormUploaderBuilder<'u, REC> {
        self.multipart.add_text("x-qn-meta-".to_owned() + key.as_ref(), value);
        self
    }

    pub(super) fn on_uploading_progress(mut self, callback: &'u dyn Fn(usize, usize)) -> FormUploaderBuilder<'u, REC> {
        self.on_uploading_progress = Some(callback);
        self
    }

    pub(super) fn seekable_stream<'n: 'u, R: Read + Seek + 'u, N: Into<Cow<'n, str>>>(
        mut self,
        mut stream: R,
        file_name: Option<N>,
        mime: Option<Mime>,
        checksum_enabled: bool,
    ) -> IOResult<FormUploader<'u, REC>> {
        let mut crc32: Option<u32> = None;
        if checksum_enabled {
            crc32 = Some(crc32::from(&mut stream)?);
            stream.seek(SeekFrom::Start(0))?;
        }
        self.multipart
            .add_stream("file", stream, file_name.map(|name| name.into()), mime);
        if let Some(crc32) = crc32 {
            self.multipart.add_text("crc32", crc32.to_string());
        }
        self.upload_multipart()
    }

    pub(super) fn stream<'n: 'u, R: Read + 'u, N: Into<Cow<'n, str>>>(
        mut self,
        stream: R,
        mime: Option<Mime>,
        file_name: Option<N>,
        crc32: Option<u32>,
    ) -> IOResult<FormUploader<'u, REC>> {
        self.multipart
            .add_stream("file", stream, file_name.map(|name| name.into()), mime);
        if let Some(crc32) = crc32 {
            self.multipart.add_text("crc32", crc32.to_string());
        }
        self.upload_multipart()
    }

    fn upload_multipart(mut self) -> IOResult<FormUploader<'u, REC>> {
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
            content_type: "multipart/form-data; boundary=".to_owned() + fields.boundary(),
            body: body,
            on_uploading_progress: self.on_uploading_progress,
        })
    }
}

impl<'u, REC> FormUploader<'u, REC>
where
    REC: recorder::Recorder,
{
    pub(super) fn send(&self) -> HTTPResult<UploadResult> {
        let mut iter = self.bucket_uploader.up_urls_list().iter();
        let mut prev_err: Option<HTTPError> = None;
        while let Some(up_urls) = iter.next() {
            match self.send_form_request(&up_urls.iter().map(|url| url.as_ref()).collect::<Box<[&str]>>()) {
                Ok(value) => {
                    return Ok(value.into());
                }
                Err(err) => match err.retry_kind() {
                    RetryKind::RetryableError | RetryKind::HostUnretryableError | RetryKind::ZoneUnretryableError => {
                        prev_err = Some(err);
                    }
                    _ => {
                        return Err(err);
                    }
                },
            }
        }

        Err(prev_err.expect("FormUploader::send() should try at lease once, but not"))
    }

    fn send_form_request(&self, up_urls: &[&str]) -> HTTPResult<serde_json::Value> {
        let mut request_builder = self
            .bucket_uploader
            .client()
            .post("/", up_urls)
            .idempotent()
            .on_response(&|response, _| upload_response_callback(response));
        if let Some(on_uploading_progress) = self.on_uploading_progress {
            request_builder = request_builder.on_uploading_progress(on_uploading_progress)
        }
        request_builder
            .accept_json()
            .raw_body(self.content_type.to_owned(), self.body.as_slice())
            .send()
            .and_then(|mut response| response.parse_json())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::super::upload_policy::UploadPolicyBuilder, *};
    use crate::{config::ConfigBuilder, credential::Credential, http::DomainsManagerBuilder};
    use qiniu_http::Headers;
    use qiniu_test_utils::{
        http_call_mock::{CounterCallMock, ErrorResponseMock, JSONCallMock},
        temp_file::create_temp_file,
    };
    use serde_json::json;
    use std::{boxed::Box, error::Error, result::Result};

    #[test]
    fn test_storage_uploader_form_uploader_upload_file() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(1 << 10)?.into_temp_path();
        let mock = CounterCallMock::new(JSONCallMock::new(
            200,
            Headers::new(),
            json!({"key": "abc", "hash": "def"}),
        ));
        let config = ConfigBuilder::default()
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        let result = BucketUploader::new(
            "test_bucket",
            vec![
                vec![Box::from("http://z1h1.com"), Box::from("http://z1h2.com")].into(),
                vec![Box::from("http://z2h1.com"), Box::from("http://z2h2.com")].into(),
            ],
            get_credential(),
            config,
        )?
        .upload_token(UploadToken::from_policy(policy, get_credential()))
        .key("test:file")
        .upload_file(&temp_path, Some("file"), None)?;
        assert_eq!(result.key(), Some("abc"));
        assert_eq!(result.hash(), Some("def"));
        assert_eq!(mock.call_called(), 1);
        Ok(())
    }

    #[test]
    fn test_storage_uploader_form_uploader_upload_file_with_500_error() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(1 << 10)?.into_temp_path();
        let mock = CounterCallMock::new(ErrorResponseMock::new(500, "test error"));
        let config = ConfigBuilder::default()
            .http_request_retries(3)
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        BucketUploader::new(
            "test_bucket",
            vec![
                vec![Box::from("http://z1h1.com"), Box::from("http://z1h2.com")].into(),
                vec![Box::from("http://z2h1.com"), Box::from("http://z2h2.com")].into(),
            ],
            get_credential(),
            config,
        )?
        .upload_token(UploadToken::from_policy(policy, get_credential()))
        .key("test:file")
        .upload_file(&temp_path, Some("file"), None)
        .unwrap_err();
        assert_eq!(mock.call_called(), 16);
        Ok(())
    }

    #[test]
    fn test_storage_uploader_form_uploader_upload_file_with_503_error() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(1 << 10)?.into_temp_path();
        let mock = CounterCallMock::new(ErrorResponseMock::new(503, "test error"));
        let config = ConfigBuilder::default()
            .http_request_retries(3)
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        BucketUploader::new(
            "test_bucket",
            vec![
                vec![Box::from("http://z1h1.com"), Box::from("http://z1h2.com")].into(),
                vec![Box::from("http://z2h1.com"), Box::from("http://z2h2.com")].into(),
            ],
            get_credential(),
            config,
        )?
        .upload_token(UploadToken::from_policy(policy, get_credential()))
        .key("test:file")
        .upload_file(&temp_path, Some("file"), None)
        .unwrap_err();
        assert_eq!(mock.call_called(), 4);
        Ok(())
    }

    #[test]
    fn test_storage_uploader_form_uploader_upload_stream_with_500_error() -> Result<(), Box<dyn Error>> {
        let file = create_temp_file(1 << 10)?.into_file();
        let mock = CounterCallMock::new(ErrorResponseMock::new(500, "test error"));
        let config = ConfigBuilder::default()
            .http_request_retries(3)
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        BucketUploader::new(
            "test_bucket",
            vec![
                vec![Box::from("http://z1h1.com"), Box::from("http://z1h2.com")].into(),
                vec![Box::from("http://z2h1.com"), Box::from("http://z2h2.com")].into(),
            ],
            get_credential(),
            config,
        )?
        .upload_token(UploadToken::from_policy(policy, get_credential()))
        .key("test:file")
        .never_be_resumeable()
        .upload_stream(&file, Some("file"), None)
        .unwrap_err();
        assert_eq!(mock.call_called(), 16);
        Ok(())
    }

    #[test]
    fn test_storage_uploader_form_uploader_upload_stream_with_503_error() -> Result<(), Box<dyn Error>> {
        let file = create_temp_file(1 << 10)?.into_file();
        let mock = CounterCallMock::new(ErrorResponseMock::new(503, "test error"));
        let config = ConfigBuilder::default()
            .http_request_retries(3)
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        BucketUploader::new(
            "test_bucket",
            vec![
                vec![Box::from("http://z1h1.com"), Box::from("http://z1h2.com")].into(),
                vec![Box::from("http://z2h1.com"), Box::from("http://z2h2.com")].into(),
            ],
            get_credential(),
            config,
        )?
        .upload_token(UploadToken::from_policy(policy, get_credential()))
        .key("test:file")
        .never_be_resumeable()
        .upload_stream(&file, Some("file"), None)
        .unwrap_err();
        assert_eq!(mock.call_called(), 4);
        Ok(())
    }

    #[test]
    fn test_storage_uploader_form_uploader_upload_stream_with_400_incorrect_zone_error() -> Result<(), Box<dyn Error>> {
        let file = create_temp_file(1 << 10)?.into_file();
        let mock = CounterCallMock::new(ErrorResponseMock::new(400, "incorrect region, please use z3h1.com"));
        let config = ConfigBuilder::default()
            .http_request_retries(3)
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build();
        BucketUploader::new(
            "test_bucket",
            vec![
                vec![Box::from("http://z1h1.com"), Box::from("http://z1h2.com")].into(),
                vec![Box::from("http://z2h1.com"), Box::from("http://z2h2.com")].into(),
            ],
            get_credential(),
            config,
        )?
        .upload_token(UploadToken::from_policy(policy, get_credential()))
        .key("test:file")
        .never_be_resumeable()
        .upload_stream(&file, Some("file"), None)
        .unwrap_err();
        assert_eq!(mock.call_called(), 2);
        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
