use super::{
    form_uploader::FormUploaderBuilder,
    resumable_uploader::{ResumableUploader, ResumableUploaderBuilder},
    upload_manager::UploadManager,
    upload_recorder::UploadRecorder,
    upload_token::UploadToken,
    UploadResponse,
};
use crate::utils::{rob::Rob, ron::Ron};
use mime::Mime;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::{Error as IOError, Read, Result as IOResult},
    path::Path,
};
use thiserror::Error;

pub(super) enum ResumablePolicy {
    Threshold(u32),
    Never,
    Always,
}

/// 对象上传器
///
/// 为指定的文件上传准备数据，不能跨线程使用，不能反复使用
#[must_use]
pub struct ObjectUploader<'b> {
    upload_manager: &'b UploadManager,
    bucket_name: Cow<'b, str>,
    up_urls_list: Box<[Box<[Box<str>]>]>,
    upload_token: Cow<'b, UploadToken>,
    key: Option<Cow<'b, str>>,
    vars: HashMap<Cow<'b, str>, Cow<'b, str>>,
    metadata: HashMap<Cow<'b, str>, Cow<'b, str>>,
    checksum_enabled: bool,
    resumable_policy: ResumablePolicy,
    #[allow(clippy::type_complexity)]
    on_uploading_progress: Option<Rob<'b, dyn Fn(u64, Option<u64>) + Send + Sync>>,
    thread_pool: Option<Ron<'b, ThreadPool>>,
    max_concurrency: usize,
}

impl<'b> ObjectUploader<'b> {
    pub(super) fn new(
        upload_manager: &'b UploadManager,
        upload_token: Cow<'b, UploadToken>,
        bucket_name: Cow<'b, str>,
        up_urls_list: Box<[Box<[Box<str>]>]>,
    ) -> Self {
        Self {
            upload_manager,
            upload_token,
            bucket_name,
            up_urls_list,
            key: None,
            vars: HashMap::new(),
            metadata: HashMap::new(),
            checksum_enabled: true,
            on_uploading_progress: None,
            thread_pool: None,
            max_concurrency: 0,
            resumable_policy: ResumablePolicy::Threshold(upload_manager.config().upload_threshold()),
        }
    }

    /// 为指定的文件上传指定线程池
    pub fn thread_pool(mut self, thread_pool: impl Into<Ron<'b, ThreadPool>>) -> Self {
        self.thread_pool = Some(thread_pool.into());
        self
    }

    /// 为上传器创建专用线程池指定线程池大小
    pub fn thread_pool_size(self, num_threads: usize) -> Self {
        self.thread_pool(
            ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .thread_name(move |index| format!("object_uploader_thread_{}_{}", num_threads, index))
                .build()
                .unwrap(),
        )
    }

    /// 上传文件最大并发度
    ///
    /// 默认情况下，分片上传将采用多线程并发的方式进行上传，最大并发度等于对象上传器内线程池的大小。
    /// 调用该方法可以修改最大并发度
    pub fn max_concurrency(mut self, concurrency: usize) -> Self {
        self.max_concurrency = concurrency;
        self
    }

    /// 指定上传对象的名称
    pub fn key(mut self, key: impl Into<Cow<'b, str>>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// 为上传对象指定[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
    ///
    /// 可以多次调用以指定多个自定义变量
    pub fn var(mut self, var_key: impl Into<Cow<'b, str>>, var_value: impl Into<Cow<'b, str>>) -> Self {
        self.vars.insert(var_key.into(), var_value.into());
        self
    }

    /// 为上传对象指定自定义元数据
    ///
    /// 可以多次调用以指定多个自定义元数据
    pub fn metadata(mut self, metadata_key: impl Into<Cow<'b, str>>, metadata_value: impl Into<Cow<'b, str>>) -> Self {
        self.metadata.insert(metadata_key.into(), metadata_value.into());
        self
    }

    /// 禁用上传数据校验
    ///
    /// 在任何场景下都不推荐禁用上传数据校验
    pub fn disable_checksum(mut self) -> Self {
        self.checksum_enabled = false;
        self
    }

    /// 启用上传数据校验
    ///
    /// 默认总是启用，在任何场景下都不推荐禁用上传数据校验
    pub fn enable_checksum(mut self) -> Self {
        self.checksum_enabled = true;
        self
    }

    /// 指定分片上传策略阙值
    ///
    /// 对于上传文件的情况，如果文件尺寸大于该值，将自动使用分片上传，否则，使用表单上传。
    /// 单位为字节，默认将采用客户端配置中的配置项。
    ///
    /// 对于上传数据流的情况，由于无法预知数据尺寸，将总是使用分片上传
    pub fn upload_threshold(mut self, threshold: u32) -> Self {
        self.resumable_policy = ResumablePolicy::Threshold(threshold);
        self
    }

    /// 总是使用分片上传
    pub fn always_be_resumable(mut self) -> Self {
        self.resumable_policy = ResumablePolicy::Always;
        self
    }

    /// 总是使用表单上传
    ///
    /// 需要注意的是，虽然表单上传仅需要一次 HTTP 调用，性能优于分片上传，
    /// 但分片上传具有断点续传的特性，以及表单上传会将整个文件内容都加载进内存中，对大文件极不友好。
    /// 因此总是推荐使用默认策略，如果认为默认阙值过小，可以适当提高客户端配置的阙值。
    pub fn never_be_resumable(mut self) -> Self {
        self.resumable_policy = ResumablePolicy::Never;
        self
    }

    /// 上传进度回调
    ///
    /// 将在上传期间反复回调指定的闭包，以获取上传进度。
    /// 上传进度闭包的第一个参数为已经上传的数据量，
    /// 第二个参数为数据总量，如果为 `None` 表示数据总量不可预知，
    /// 单位均为字节
    pub fn on_progress_ref(mut self, callback: &'b (dyn Fn(u64, Option<u64>) + Send + Sync)) -> Self {
        self.on_uploading_progress = Some(callback.into());
        self
    }

    /// 上传进度回调
    ///
    /// 将在上传期间反复回调指定的闭包，以获取上传进度。
    /// 上传进度闭包的第一个参数为已经上传的数据量，
    /// 第二个参数为数据总两，如果为 `None` 表示数据总量不可预知，
    /// 单位均为字节
    pub fn on_progress(mut self, callback: impl Fn(u64, Option<u64>) + Send + Sync + 'static) -> Self {
        self.on_uploading_progress = Some(Rob::Owned(Box::new(callback)));
        self
    }

    /// 开始上传文件
    ///
    /// # Arguments
    ///
    /// * `file_path` - 上传文件路径
    /// * `file_name` - 指定上传文件的文件名称，在下载文件时将会被使用
    /// * `mime` - 指定文件的 MIME 类型，参照[文档](https://docs.rs/mime/0.3.14/mime/) 传值，如果不填写，七牛服务器将根据上传策略决定 `Content-Type`
    pub fn upload_file(
        self,
        file_path: impl AsRef<Path>,
        file_name: impl Into<Cow<'b, str>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let file_path = file_path.as_ref();
        let file_name = file_name.into();
        match self.resumable_policy {
            ResumablePolicy::Threshold(threshold) => {
                if file_path.metadata()?.len() > threshold.into() {
                    self.upload_file_by_blocks(file_path, file_name, mime)
                } else {
                    self.upload_file_by_form(file_path, file_name, mime)
                }
            }
            ResumablePolicy::Always => self.upload_file_by_blocks(file_path, file_name, mime),
            ResumablePolicy::Never => self.upload_file_by_form(file_path, file_name, mime),
        }
    }

    /// 开始上传数据流
    ///
    /// # Arguments
    ///
    /// * `stream` - 数据流
    /// * `size` - 数据流最大长度，如果数据流大小不可预知，则传入 `0`。如果传入的值大于 `0`，则最终读取数据量将始终不大于该值。
    /// * `file_name` - 指定上传文件的文件名称，在下载文件时将会被使用
    /// * `mime` - 指定文件的 MIME 类型，参照[文档](https://docs.rs/mime/0.3.14/mime/) 传值，如果不填写，七牛服务器将根据上传策略决定 `Content-Type`
    pub fn upload_stream(
        self,
        stream: impl Read + Send,
        size: u64,
        file_name: impl Into<Cow<'b, str>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let file_name = file_name.into();
        match self.resumable_policy {
            ResumablePolicy::Threshold(threshold) => {
                if size > 0 && size < threshold.into() {
                    self.upload_stream_by_form(stream, size, file_name, mime)
                } else {
                    self.upload_stream_by_blocks(stream, size, file_name, mime)
                }
            }
            ResumablePolicy::Always => self.upload_stream_by_blocks(stream, size, file_name, mime),
            ResumablePolicy::Never => self.upload_stream_by_form(stream, size, file_name, mime),
        }
    }

    fn upload_file_by_form(self, file_path: &Path, file_name: Cow<str>, mime: Option<Mime>) -> UploadResult {
        let mut uploader = FormUploaderBuilder::new(self.upload_manager, &self.upload_token, &self.up_urls_list);
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        for (k, v) in self.vars.into_iter() {
            uploader = uploader.var(&k, v);
        }
        for (k, v) in self.metadata.into_iter() {
            uploader = uploader.metadata(&k, v);
        }
        if let Some(callback) = &self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback.as_ref());
        }
        Ok(uploader
            .seekable_stream(
                File::open(file_path)?,
                Self::guess_filename(file_path, file_name),
                Self::guess_mime_from_file_path(mime, file_path),
                self.checksum_enabled,
            )?
            .send()?)
    }

    fn upload_file_by_blocks<'n>(self, file_path: &Path, file_name: Cow<'n, str>, mime: Option<Mime>) -> UploadResult {
        let file = File::open(file_path)?;
        let file_size = file.metadata()?.len();
        if file_size == 0 {
            return Err(UploadError::EmptyFileError);
        }
        let mut uploader = ResumableUploaderBuilder::new(
            self.upload_manager,
            self.upload_token,
            &self.bucket_name,
            &self.up_urls_list,
        )
        .max_concurrency(self.max_concurrency)
        .vars(self.vars)
        .metadata(self.metadata);

        if let Some(key) = &self.key {
            uploader = uploader.key(key.to_owned());
        }
        if let Some(callback) = &self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback.as_ref());
        }
        if let Some(thread_pool) = self.thread_pool {
            uploader = uploader.thread_pool(thread_pool);
        }
        let mut uploader = uploader.file(
            file,
            file_path.into(),
            Self::guess_filename(file_path, file_name),
            file_size,
            Self::guess_mime_from_file_path(mime, file_path),
            self.checksum_enabled,
        )?;
        Self::prepare_for_resuming(
            self.key.as_ref().map(|key| key.as_ref()),
            self.upload_manager.config().upload_recorder(),
            &mut uploader,
            file_path,
        )?;
        Ok(uploader.send()?)
    }

    fn prepare_for_resuming(
        key: Option<&str>,
        recorder: &UploadRecorder,
        uploader: &mut ResumableUploader<'_, File>,
        file_path: &Path,
    ) -> IOResult<()> {
        if let Some((file_record, block_records)) = recorder.load(file_path, key)? {
            uploader.prepare_for_resuming(file_record, block_records, recorder.open_for_appending(file_path, key)?);
        }
        Ok(())
    }

    fn upload_stream_by_form<R: Read>(
        self,
        stream: R,
        size: u64,
        file_name: Cow<str>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let mut uploader = FormUploaderBuilder::new(self.upload_manager, &self.upload_token, &self.up_urls_list);
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        for (k, v) in self.vars.into_iter() {
            uploader = uploader.var(&k, v);
        }
        for (k, v) in self.metadata.into_iter() {
            uploader = uploader.metadata(&k, v);
        }
        if let Some(callback) = &self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback.as_ref());
        }
        let mime = Self::guess_mime_from_file_name(mime, file_name.as_ref());
        let upload_response = if size > 0 {
            uploader.stream(stream.take(size), file_name, mime, None)?.send()?
        } else {
            uploader.stream(stream, file_name, mime, None)?.send()?
        };
        Ok(upload_response)
    }

    fn upload_stream_by_blocks<R: Read + Send>(
        self,
        stream: R,
        size: u64,
        file_name: Cow<str>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let mut uploader = ResumableUploaderBuilder::new(
            &self.upload_manager,
            self.upload_token,
            &self.bucket_name,
            &self.up_urls_list,
        )
        .max_concurrency(self.max_concurrency)
        .vars(self.vars)
        .metadata(self.metadata);
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        if let Some(callback) = &self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback.as_ref());
        }
        if let Some(thread_pool) = self.thread_pool {
            uploader = uploader.thread_pool(thread_pool);
        }
        let mime = Self::guess_mime_from_file_name(mime, file_name.as_ref());
        let upload_response = if size > 0 {
            uploader
                .stream(stream.take(size), size, mime, file_name, true)?
                .send()?
        } else {
            uploader.stream(stream, size, mime, file_name, true)?.send()?
        };
        Ok(upload_response)
    }

    fn guess_filename<'n>(file_path: &Path, file_name: Cow<'n, str>) -> Cow<'n, str> {
        if file_name.is_empty() {
            file_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_owned().into())
                .unwrap_or_default()
        } else {
            file_name
        }
    }

    fn guess_mime_from_file_path(mime: Option<Mime>, file_path: &Path) -> Option<Mime> {
        mime.or_else(|| mime_guess::from_path(file_path).first())
    }

    fn guess_mime_from_file_name(mime: Option<Mime>, file_name: &str) -> Option<Mime> {
        mime.or_else(|| {
            Some(file_name)
                .filter(|n| !n.is_empty())
                .and_then(|file_name| mime_guess::from_path(file_name).first())
        })
    }
}

/// 上传错误
#[derive(Error, Debug)]
pub enum UploadError {
    /// 读取数据发送 IO 错误
    #[error("Failed to do local io operation during uploading: {0}")]
    IOError(#[from] IOError),
    /// 读取数据发送 IO 错误
    #[error("Should not upload empty file")]
    EmptyFileError,
    /// 调用七牛 API 上传时发送错误
    #[error("Qiniu API call error: {0}")]
    QiniuError(#[from] crate::http::Error),
}
/// 上传结果
pub type UploadResult = Result<UploadResponse, UploadError>;

#[cfg(test)]
mod tests {
    use super::{
        super::{resumable_uploader::encode_key, UploadPolicyBuilder},
        *,
    };
    use crate::{
        http::{DomainsManagerBuilder, Error as HTTPError, ErrorKind as HTTPErrorKind, Headers, Method},
        utils::mime,
        ConfigBuilder, Credential,
    };
    use qiniu_http::ResponseBuilder;
    use qiniu_test_utils::{
        http_call_mock::{fake_req_id, CallHandlers},
        temp_file::create_temp_file,
    };
    use serde_json::json;
    use std::{error::Error, result::Result};

    #[test]
    fn test_storage_uploader_object_uploader_upload_file_with_recovering() -> Result<(), Box<dyn Error>> {
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
                        headers.insert("Content-Type".into(), mime::JSON_MIME.into());
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
                        headers.insert("Content-Type".into(), mime::JSON_MIME.into());
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
                        headers.insert("Content-Type".into(), mime::JSON_MIME.into());
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
        {
            let medium = config.upload_recorder().open_and_write_metadata(
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
        let result = ObjectUploader::new(
            &UploadManager::new(config),
            Cow::Owned(UploadToken::new(policy, get_credential())),
            "test_bucket".into(),
            vec![vec![Box::from("http://z1h1.com")].into()].into(),
        )
        .key("test-key")
        .upload_file(temp_path, "", None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }
    #[test]
    fn test_storage_uploader_object_uploader_upload_file_with_1_unretryable_failure() -> Result<(), Box<dyn Error>> {
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
                        headers.insert("Content-Type".into(), mime::JSON_MIME.into());
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
                            return Err(HTTPError::new_unretryable_error(
                                HTTPErrorKind::MaliciousResponse,
                                None,
                                None,
                                None,
                            ));
                        } else if called >= 5 {
                            panic!("Unexpected call `PUT {}` for {} times", request.url(), called);
                        }
                        let mut headers = Headers::new();
                        headers.insert("Content-Type".into(), mime::JSON_MIME.into());
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
                        headers.insert("Content-Type".into(), mime::JSON_MIME.into());
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
        let token = UploadToken::new(policy, get_credential());

        assert!(ObjectUploader::new(
            &UploadManager::new(config.to_owned()),
            Cow::Borrowed(&token),
            "test_bucket".into(),
            vec![vec![Box::from("http://z1h1.com")].into(),].into(),
        )
        .key("test-key")
        .upload_file(&temp_path, "", None)
        .is_err());

        let result = ObjectUploader::new(
            &UploadManager::new(config),
            Cow::Borrowed(&token),
            "test_bucket".into(),
            vec![vec![Box::from("http://z1h1.com")].into()].into(),
        )
        .key("test-key")
        .upload_file(temp_path, "", None)?;
        assert_eq!(result.key(), Some("test-key"));
        assert_eq!(result.hash(), Some("abcdef"));
        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
