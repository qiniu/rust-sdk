/// 存储空间上传模块
///
/// 封装存储空间上传器和文件上传器逻辑。
/// 需要注意的是，该模块内所有提到的与线程，并发相关的概念仅在分片上传时起效
use super::{
    super::uploader::{UploadPolicy, UploadToken},
    form_uploader::FormUploaderBuilder,
    resumable_uploader::{ResumableUploader, ResumableUploaderBuilder},
    upload_recorder::UploadRecorder,
    UploadLogger, UploadResponse,
};
use crate::{
    config::Config,
    credential::Credential,
    http::Client,
    utils::{rob::Rob, ron::Ron},
};
use assert_impl::assert_impl;
use getset::Getters;
use mime::Mime;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::{Error as IOError, Read, Result as IOResult},
    path::Path,
    sync::Arc,
};
use thiserror::Error;

#[doc(hidden)]
#[derive(Getters)]
#[get = "pub(super)"]
pub struct BucketUploaderInner {
    bucket_name: Box<str>,
    up_urls_list: Box<[Box<[Box<str>]>]>,
    http_client: Client,
    upload_logger: Option<UploadLogger>,
    recorder: UploadRecorder,
    thread_pool: Option<ThreadPool>,
}

/// 存储空间上传器
///
/// 为指定存储空间的上传准备初始化数据，可以反复使用以上传多个文件
#[derive(Clone)]
pub struct BucketUploader {
    inner: Arc<BucketUploaderInner>,
}

impl BucketUploader {
    pub(super) fn bucket_name(&self) -> &str {
        self.inner.bucket_name()
    }
    pub(super) fn up_urls_list(&self) -> &[Box<[Box<str>]>] {
        self.inner.up_urls_list()
    }
    pub(super) fn http_client(&self) -> &Client {
        self.inner.http_client()
    }
    pub(super) fn upload_logger(&self) -> Option<&UploadLogger> {
        self.inner.upload_logger().as_ref()
    }
    pub(super) fn recorder(&self) -> &UploadRecorder {
        self.inner.recorder()
    }
    pub(super) fn thread_pool(&self) -> Option<&ThreadPool> {
        self.inner.thread_pool().as_ref()
    }
}

/// 存储空间上传器生成器
pub struct BucketUploaderBuilder {
    inner: BucketUploaderInner,
}

impl BucketUploaderBuilder {
    pub(super) fn new(
        bucket_name: Box<str>,
        up_urls_list: Box<[Box<[Box<str>]>]>,
        config: Config,
    ) -> BucketUploaderBuilder {
        assert!(!up_urls_list.is_empty());
        BucketUploaderBuilder {
            inner: BucketUploaderInner {
                bucket_name,
                up_urls_list,
                thread_pool: None,
                recorder: config.upload_recorder().to_owned(),
                upload_logger: config.upload_logger().to_owned(),
                http_client: Client::new(config),
            },
        }
    }

    /// 为指定的文件上传指定线程池
    pub fn thread_pool(mut self, thread_pool: ThreadPool) -> BucketUploaderBuilder {
        self.inner.thread_pool = Some(thread_pool);
        self
    }

    /// 为上传器创建专用线程池指定线程池大小
    pub fn thread_pool_size(self, num_threads: usize) -> BucketUploaderBuilder {
        self.thread_pool(
            ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .thread_name(move |index| format!("bucket_uploader_thread_{}_{}", num_threads, index))
                .build()
                .unwrap(),
        )
    }

    /// 生成存储空间上传器
    pub fn build(self) -> BucketUploader {
        BucketUploader {
            inner: Arc::new(self.inner),
        }
    }
}

impl BucketUploader {
    /// 根据上传凭证创建文件上传器生成器
    pub fn upload_token<'b>(&'b self, upload_token: impl Into<UploadToken<'b>>) -> FileUploaderBuilder<'b> {
        FileUploaderBuilder::new(Ron::Referenced(self), upload_token.into().to_string().into())
    }

    /// 根据上传策略创建文件上传器生成器
    pub fn upload_policy<'b>(
        &'b self,
        upload_policy: UploadPolicy<'b>,
        credential: impl Into<Cow<'b, Credential>>,
    ) -> FileUploaderBuilder<'b> {
        FileUploaderBuilder::new(
            Ron::Referenced(self),
            UploadToken::new(upload_policy, credential.into()).to_string().into(),
        )
    }

    #[doc(hidden)]
    pub unsafe fn from_raw(ptr: *const BucketUploaderInner) -> BucketUploader {
        BucketUploader {
            inner: Arc::from_raw(ptr),
        }
    }

    #[doc(hidden)]
    pub fn into_raw(self) -> *const BucketUploaderInner {
        Arc::into_raw(self.inner)
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

enum ResumablePolicy {
    Threshold(u32),
    Never,
    Always,
}

/// 文件上传器
///
/// 为指定的文件上传准备数据，不能跨线程使用，不能反复使用
pub struct FileUploaderBuilder<'b> {
    bucket_uploader: Ron<'b, BucketUploader>,
    upload_token: Cow<'b, str>,
    key: Option<Cow<'b, str>>,
    vars: Option<HashMap<Cow<'b, str>, Cow<'b, str>>>,
    metadata: Option<HashMap<Cow<'b, str>, Cow<'b, str>>>,
    checksum_enabled: bool,
    resumable_policy: ResumablePolicy,
    #[allow(clippy::type_complexity)]
    on_uploading_progress: Option<Rob<'b, dyn Fn(u64, Option<u64>) + Send + Sync>>,
    thread_pool: Option<Ron<'b, ThreadPool>>,
    max_concurrency: usize,
}

impl<'b> FileUploaderBuilder<'b> {
    pub(super) fn new(bucket_uploader: Ron<'b, BucketUploader>, upload_token: Cow<'b, str>) -> FileUploaderBuilder<'b> {
        FileUploaderBuilder {
            upload_token,
            key: None,
            vars: None,
            metadata: None,
            checksum_enabled: true,
            on_uploading_progress: None,
            thread_pool: None,
            max_concurrency: 0,
            resumable_policy: ResumablePolicy::Threshold(bucket_uploader.http_client().config().upload_threshold()),
            bucket_uploader,
        }
    }

    /// 为指定的文件上传指定线程池
    pub fn thread_pool(mut self, thread_pool: impl Into<Ron<'b, ThreadPool>>) -> FileUploaderBuilder<'b> {
        self.thread_pool = Some(thread_pool.into());
        self
    }

    /// 为上传器创建专用线程池指定线程池大小
    pub fn thread_pool_size(self, num_threads: usize) -> FileUploaderBuilder<'b> {
        self.thread_pool(
            ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .thread_name(move |index| format!("file_uploader_thread_{}_{}", num_threads, index))
                .build()
                .unwrap(),
        )
    }

    /// 上传文件最大并发度
    ///
    /// 默认情况下，分片上传将采用多线程并发的方式进行上传，最大并发度等于文件上传器内线程池的大小。
    /// 调用该方法可以修改最大并发度
    ///
    /// `concurrency` 必须大于 0
    pub fn max_concurrency(mut self, concurrency: usize) -> FileUploaderBuilder<'b> {
        assert!(concurrency > 0);
        self.max_concurrency = concurrency;
        self
    }

    /// 指定上传对象的名称
    pub fn key(mut self, key: impl Into<Cow<'b, str>>) -> FileUploaderBuilder<'b> {
        self.key = Some(key.into());
        self
    }

    /// 为上传对象指定[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
    ///
    /// 可以多次调用以指定多个自定义变量
    pub fn var(mut self, key: impl Into<Cow<'b, str>>, value: impl Into<Cow<'b, str>>) -> FileUploaderBuilder<'b> {
        if let Some(vars) = &mut self.vars {
            vars.insert(key.into(), value.into());
        } else {
            let mut vars = HashMap::with_capacity(1);
            vars.insert(key.into(), value.into());
            self.vars = Some(vars);
        }
        self
    }

    /// 为上传对象指定自定义元数据
    ///
    /// 可以多次调用以指定多个自定义元数据
    pub fn metadata(mut self, key: impl Into<Cow<'b, str>>, value: impl Into<Cow<'b, str>>) -> FileUploaderBuilder<'b> {
        if let Some(metadata) = &mut self.metadata {
            metadata.insert(key.into(), value.into());
        } else {
            let mut metadata = HashMap::with_capacity(1);
            metadata.insert(key.into(), value.into());
            self.metadata = Some(metadata);
        }
        self
    }

    /// 禁用文件校验
    ///
    /// 在任何场景下都不推荐禁用文件校验
    pub fn disable_checksum(mut self) -> FileUploaderBuilder<'b> {
        self.checksum_enabled = false;
        self
    }

    /// 启用文件校验
    ///
    /// 默认总是启用，在任何场景下都不推荐禁用文件校验
    pub fn enable_checksum(mut self) -> FileUploaderBuilder<'b> {
        self.checksum_enabled = true;
        self
    }

    /// 指定分片上传策略阙值
    ///
    /// 对于上传文件的情况，如果文件尺寸大于该值，将自动使用分片上传，否则，使用表单上传。
    /// 单位为字节，默认将采用客户端配置中的配置项。
    ///
    /// 对于上传数据流的情况，由于无法预知数据尺寸，将总是使用分片上传
    pub fn upload_threshold(mut self, threshold: u32) -> FileUploaderBuilder<'b> {
        self.resumable_policy = ResumablePolicy::Threshold(threshold);
        self
    }

    /// 总是使用分片上传
    pub fn always_be_resumable(mut self) -> FileUploaderBuilder<'b> {
        self.resumable_policy = ResumablePolicy::Always;
        self
    }

    /// 总是使用表单上传
    ///
    /// 需要注意的是，虽然表单上传仅需要一次 HTTP 调用，性能优于分片上传，
    /// 但分片上传具有断点续传的特性，以及表单上传会将整个文件内容都加载进内存中，对大文件极不友好。
    /// 因此总是推荐使用默认策略，如果认为默认阙值过小，可以适当提高客户端配置的阙值。
    pub fn never_be_resumable(mut self) -> FileUploaderBuilder<'b> {
        self.resumable_policy = ResumablePolicy::Never;
        self
    }

    /// 上传进度回调
    ///
    /// 将在上传期间反复回调指定的闭包，以获取上传进度。
    /// 上传进度闭包的第一个参数为已经上传的数据量，
    /// 第二个参数为数据总量，如果为 `None` 表示数据总量不可预知，
    /// 单位均为字节
    pub fn on_progress_ref(
        mut self,
        callback: &'b (dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> FileUploaderBuilder<'b> {
        self.on_uploading_progress = Some(callback.into());
        self
    }

    /// 上传进度回调
    ///
    /// 将在上传期间反复回调指定的闭包，以获取上传进度。
    /// 上传进度闭包的第一个参数为已经上传的数据量，
    /// 第二个参数为数据总两，如果为 `None` 表示数据总量不可预知，
    /// 单位均为字节
    pub fn on_progress(
        mut self,
        callback: impl Fn(u64, Option<u64>) + Send + Sync + 'static,
    ) -> FileUploaderBuilder<'b> {
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
    pub fn upload_file<'n>(
        self,
        file_path: impl AsRef<Path>,
        file_name: impl Into<Cow<'n, str>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let file_path = file_path.as_ref();
        let file_name = file_name.into();
        let file_name = if file_name.is_empty() { None } else { Some(file_name) };
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

    /// 开始上传文件流
    ///
    /// # Arguments
    ///
    /// * `stream` - 数据流
    /// * `file_name` - 指定上传文件的文件名称，在下载文件时将会被使用
    /// * `mime` - 指定文件的 MIME 类型，参照[文档](https://docs.rs/mime/0.3.14/mime/) 传值，如果不填写，七牛服务器将根据上传策略决定 `Content-Type`
    pub fn upload_stream<'n>(
        self,
        stream: impl Read + Send,
        file_name: impl Into<Cow<'n, str>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let file_name = file_name.into();
        let file_name = if file_name.is_empty() { None } else { Some(file_name) };
        match self.resumable_policy {
            ResumablePolicy::Threshold(_) | ResumablePolicy::Always => {
                self.upload_stream_by_blocks(stream, file_name, mime)
            }
            ResumablePolicy::Never => self.upload_stream_by_form(stream, file_name, mime),
        }
    }

    fn upload_file_by_form<'n>(
        self,
        file_path: &Path,
        file_name: Option<Cow<'n, str>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let mut uploader = FormUploaderBuilder::new(&self.bucket_uploader, &self.upload_token);
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        if let Some(vars) = self.vars {
            for (k, v) in vars.into_iter() {
                uploader = uploader.var(&k, v);
            }
        }
        if let Some(metadata) = self.metadata {
            for (k, v) in metadata.into_iter() {
                uploader = uploader.metadata(&k, v);
            }
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

    fn upload_file_by_blocks<'n>(
        self,
        file_path: &Path,
        file_name: Option<Cow<'n, str>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let mut uploader = ResumableUploaderBuilder::new(&self.bucket_uploader, self.upload_token)
            .max_concurrency(self.max_concurrency);
        if let Some(key) = &self.key {
            uploader = uploader.key(key.to_owned());
        }
        if let Some(vars) = self.vars {
            uploader = uploader.vars(vars);
        }
        if let Some(metadata) = self.metadata {
            uploader = uploader.metadata(metadata);
        }
        if let Some(callback) = &self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback.as_ref());
        }
        if let Some(thread_pool) = self.thread_pool {
            uploader = uploader.thread_pool(thread_pool);
        }
        let mut uploader = uploader.file(
            File::open(file_path)?,
            file_path.into(),
            Self::guess_filename(file_path, file_name),
            file_path.metadata()?.len(),
            Self::guess_mime_from_file_path(mime, file_path),
            self.checksum_enabled,
        )?;
        Self::prepare_for_resuming(
            self.key.as_ref().map(|key| key.as_ref()),
            &self.bucket_uploader.recorder(),
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
        file_name: Option<Cow<str>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let mut uploader = FormUploaderBuilder::new(&self.bucket_uploader, &self.upload_token);
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        if let Some(vars) = self.vars {
            for (k, v) in vars.into_iter() {
                uploader = uploader.var(&k, v);
            }
        }
        if let Some(metadata) = self.metadata {
            for (k, v) in metadata.into_iter() {
                uploader = uploader.metadata(&k, v);
            }
        }
        if let Some(callback) = &self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback.as_ref());
        }
        Ok(uploader
            .stream(
                stream,
                Self::guess_mime_from_file_name(mime, file_name.as_ref().map(|name| name.as_ref())),
                file_name,
                None,
            )?
            .send()?)
    }

    fn upload_stream_by_blocks<R: Read + Send>(
        self,
        stream: R,
        file_name: Option<Cow<str>>,
        mime: Option<Mime>,
    ) -> UploadResult {
        let mut uploader = ResumableUploaderBuilder::new(&self.bucket_uploader, self.upload_token)
            .max_concurrency(self.max_concurrency);
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        if let Some(vars) = self.vars {
            uploader = uploader.vars(vars);
        }
        if let Some(metadata) = self.metadata {
            uploader = uploader.metadata(metadata);
        }
        if let Some(callback) = &self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback.as_ref());
        }
        if let Some(thread_pool) = self.thread_pool {
            uploader = uploader.thread_pool(thread_pool);
        }
        Ok(uploader
            .stream(
                stream,
                Self::guess_mime_from_file_name(mime, file_name.as_ref().map(|name| name.as_ref())),
                file_name,
                true,
            )?
            .send()?)
    }

    fn guess_filename<'n>(file_path: &Path, file_name: Option<Cow<'n, str>>) -> Option<Cow<'n, str>> {
        file_name.or_else(|| {
            file_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_owned().into())
        })
    }

    fn guess_mime_from_file_path(mime: Option<Mime>, file_path: &Path) -> Option<Mime> {
        mime.or_else(|| mime_guess::from_path(file_path).first())
    }

    fn guess_mime_from_file_name(mime: Option<Mime>, file_name: Option<&str>) -> Option<Mime> {
        mime.or_else(|| file_name.and_then(|file_name| mime_guess::from_path(file_name).first()))
    }
}

/// 上传错误
#[derive(Error, Debug)]
pub enum UploadError {
    /// 读取数据发送 IO 错误
    #[error("Failed to do local io operation during uploading: {0}")]
    IOError(#[from] IOError),
    /// 调用七牛 API 上传时发送错误
    #[error("Qiniu API call error: {0}")]
    QiniuError(#[from] crate::http::Error),
}
/// 上传结果
pub type UploadResult = Result<UploadResponse, UploadError>;
