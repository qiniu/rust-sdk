use super::{bucket_uploader::ResumablePolicy, BucketUploader, FileUploaderBuilder, UploadResult};
use crate::{utils::ron::Ron, Config};
use mime::Mime;
use object_pool::Pool;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::{Read, Result},
    path::Path,
};

type OnUploadingProgressCallback = Box<dyn Fn(u64, Option<u64>) + Send + Sync>;
type OnCompletedCallback = Box<dyn Fn(UploadResult) + Send + Sync>;

enum BatchUploadTarget {
    File(File),
    Stream(Box<dyn Read + Send>),
}

/// 批量上传任务，包装一个上传任务供批量上传器负责上传
#[must_use = "创建上传任务并不会真正上传文件，您需要将当前任务提交到批量上传器后，调用 `start` 方法执行上传任务"]
pub struct BatchUploadJob {
    key: String,
    upload_token: String,
    vars: HashMap<String, String>,
    metadata: HashMap<String, String>,
    checksum_enabled: bool,
    resumable_policy: Option<ResumablePolicy>,
    file_name: String,
    mime: Option<Mime>,
    on_uploading_progress: Option<OnUploadingProgressCallback>,
    on_completed: Option<OnCompletedCallback>,
    target: BatchUploadTarget,
    expected_stream_size: u64,
}

/// 批量上传任务生成器，提供上传数据所需的多个参数
pub struct BatchUploadJobBuilder {
    key: String,
    upload_token: String,
    vars: HashMap<String, String>,
    metadata: HashMap<String, String>,
    checksum_enabled: bool,
    on_uploading_progress: Option<OnUploadingProgressCallback>,
    on_completed: Option<OnCompletedCallback>,
    resumable_policy: Option<ResumablePolicy>,
}

struct BatchUploaderContext {
    upload_token: String,
    bucket_uploader: BucketUploader,
    max_concurrency: usize,
    thread_pool_size: usize,
}

/// 批量上传器，上传之前所有提交的任务
pub struct BatchUploader {
    context: BatchUploaderContext,
    jobs: Vec<BatchUploadJob>,
}

impl BatchUploader {
    pub(super) fn new(
        bucket_uploader: &BucketUploader,
        upload_token: impl Into<String>,
        expected_jobs_count: usize,
    ) -> Self {
        Self {
            jobs: Vec::with_capacity(expected_jobs_count),
            context: BatchUploaderContext {
                bucket_uploader: bucket_uploader.to_owned(),
                upload_token: upload_token.into(),
                max_concurrency: 0,
                thread_pool_size: 0,
            },
        }
    }

    /// 为上传器创建专用线程池指定线程池大小
    ///
    /// 批量上传器总是优先使用线程池存储空间上传器中的线程池，如果存储空间上传器中没有创建过线程池，则自行创建专用线程池
    pub fn thread_pool_size(mut self, num_threads: usize) -> Self {
        self.context.thread_pool_size = num_threads;
        self
    }

    /// 上传文件最大并发度
    ///
    /// 默认情况下，上传文件时的最大并发度等于其使用的线程池大小。
    /// 调用该方法可以修改最大并发度
    pub fn max_concurrency(mut self, concurrency: usize) -> Self {
        self.context.max_concurrency = concurrency;
        self
    }

    /// 提交上传任务
    pub fn push_job(mut self, job: BatchUploadJob) -> Self {
        self.jobs.push(job);
        self
    }

    /// 开始执行上传任务
    ///
    /// 需要注意的是，该方法会持续阻塞直到上传任务全部执行完毕。
    /// 该方法不返回任何结果，上传结果由每个上传任务内定义的 `on_completed` 回调负责返回
    pub fn start(self) {
        let Self { context, jobs } = self;
        let thread_pool = build_thread_pool(&context);

        // 防止出现当所有上传任务都是分片上传时，控制上传所用的线程占满整个线程池，没有任何线程用于实质上传，导致程序死锁
        let semaphore = Pool::new(thread_pool.current_num_threads() - 1, || ());
        thread_pool.scope(|s| {
            for job in jobs {
                let _ = if job.use_resumeable_uploader(&context.bucket_uploader.http_client().config()) {
                    Some(semaphore.pull())
                } else {
                    None
                };
                s.spawn(|_| {
                    handle_job(&context, job, &thread_pool);
                })
            }
        });
    }
}

fn build_thread_pool(context: &BatchUploaderContext) -> Ron<'_, ThreadPool> {
    context
        .bucket_uploader
        .thread_pool()
        .map(Ron::Referenced)
        .unwrap_or_else(|| {
            let mut builder = ThreadPoolBuilder::new();
            if context.thread_pool_size > 0 {
                builder = builder.num_threads(context.thread_pool_size);
            }
            Ron::Owned(
                builder
                    .thread_name(|index| format!("qiniu_ng_batch_uploader_worker_{}", index))
                    .build()
                    .unwrap(),
            )
        })
}

fn handle_job(context: &BatchUploaderContext, job: BatchUploadJob, thread_pool: &ThreadPool) {
    let BatchUploadJob {
        key,
        upload_token,
        vars,
        metadata,
        checksum_enabled,
        resumable_policy,
        file_name,
        mime,
        target,
        expected_stream_size,
        on_uploading_progress,
        on_completed,
    } = job;

    let mut builder = FileUploaderBuilder::new(
        Ron::Referenced(&context.bucket_uploader),
        if upload_token.is_empty() {
            Cow::Borrowed(&context.upload_token)
        } else {
            Cow::Owned(upload_token)
        },
    )
    .thread_pool(thread_pool)
    .max_concurrency(context.max_concurrency)
    .key(key);
    for (var_name, var_value) in vars.into_iter() {
        builder = builder.var(var_name, var_value);
    }
    for (metadata_name, metadata_value) in metadata.into_iter() {
        builder = builder.metadata(metadata_name, metadata_value);
    }
    if checksum_enabled {
        builder = builder.enable_checksum();
    } else {
        builder = builder.disable_checksum();
    }
    if let Some(on_uploading_progress) = on_uploading_progress {
        builder = builder.on_progress(on_uploading_progress);
    }
    if let Some(resumable_policy) = resumable_policy {
        match resumable_policy {
            ResumablePolicy::Threshold(threshold) => {
                builder = builder.upload_threshold(threshold);
            }
            ResumablePolicy::Never => {
                builder = builder.never_be_resumable();
            }
            ResumablePolicy::Always => {
                builder = builder.always_be_resumable();
            }
        }
    }
    let upload_result = match target {
        BatchUploadTarget::File(file) => builder.upload_stream(file, expected_stream_size, file_name, mime),
        BatchUploadTarget::Stream(reader) => builder.upload_stream(reader, expected_stream_size, file_name, mime),
    };
    if let Some(on_completed) = on_completed.as_ref() {
        on_completed(upload_result);
    }
}

impl Default for BatchUploadJobBuilder {
    fn default() -> Self {
        Self {
            key: String::new(),
            upload_token: String::new(),
            vars: HashMap::new(),
            metadata: HashMap::new(),
            checksum_enabled: true,
            on_uploading_progress: None,
            on_completed: None,
            resumable_policy: None,
        }
    }
}

impl BatchUploadJobBuilder {
    /// 指定上传对象的名称
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = key.into();
        self
    }

    /// 指定上传所用的上传凭证
    ///
    /// 默认情况下，总是复用批量上传器创建时传入的上传凭证。
    /// 该方法则可以在指定上传当前对象时使用上传凭证
    pub fn upload_token(mut self, upload_token: impl Into<String>) -> Self {
        self.upload_token = upload_token.into();
        self
    }

    /// 为上传对象指定[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
    ///
    /// 可以多次调用以指定多个自定义变量
    pub fn var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    /// 为上传对象指定自定义元数据
    ///
    /// 可以多次调用以指定多个自定义元数据
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
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
        self.resumable_policy = Some(ResumablePolicy::Threshold(threshold));
        self
    }

    /// 总是使用分片上传
    pub fn always_be_resumable(mut self) -> Self {
        self.resumable_policy = Some(ResumablePolicy::Always);
        self
    }

    /// 总是使用表单上传
    ///
    /// 需要注意的是，虽然表单上传仅需要一次 HTTP 调用，性能优于分片上传，
    /// 但分片上传具有断点续传的特性，以及表单上传会将整个文件内容都加载进内存中，对大文件极不友好。
    /// 因此总是推荐使用默认策略，如果认为默认阙值过小，可以适当提高客户端配置的阙值。
    pub fn never_be_resumable(mut self) -> Self {
        self.resumable_policy = Some(ResumablePolicy::Never);
        self
    }

    /// 上传进度回调
    ///
    /// 将在上传期间反复回调指定的闭包，以获取上传进度。
    /// 上传进度闭包的第一个参数为已经上传的数据量，
    /// 第二个参数为数据总量，如果为 `None` 表示数据总量不可预知，
    /// 单位均为字节
    pub fn on_uploading_progress(mut self, progress: impl Fn(u64, Option<u64>) + Send + Sync + 'static) -> Self {
        self.on_uploading_progress = Some(Box::new(progress));
        self
    }

    /// 完成上传回调
    ///
    /// 将在上传完毕后回调指定的闭包，返回上传结果。
    pub fn on_completed(mut self, on_completed: impl Fn(UploadResult) + Send + Sync + 'static) -> Self {
        self.on_completed = Some(Box::new(on_completed));
        self
    }

    /// 上传文件
    ///
    /// 该方法用于生成批量上传任务，用于上传指定路径的文件
    pub fn upload_file(
        self,
        file_path: impl AsRef<Path>,
        file_name: impl Into<String>,
        mime: Option<Mime>,
    ) -> Result<BatchUploadJob> {
        let file = File::open(file_path.as_ref())?;
        let job = BatchUploadJob {
            key: self.key,
            upload_token: self.upload_token,
            vars: self.vars,
            metadata: self.metadata,
            checksum_enabled: self.checksum_enabled,
            resumable_policy: self.resumable_policy,
            on_uploading_progress: self.on_uploading_progress,
            on_completed: self.on_completed,
            file_name: file_name.into(),
            mime,
            expected_stream_size: file.metadata()?.len(),
            target: BatchUploadTarget::File(file),
        };
        Ok(job)
    }

    /// 上传数据流
    ///
    /// 该方法用于生成批量上传任务，用于上传指定的数据流
    pub fn upload_stream(
        self,
        stream: impl Read + Send + 'static,
        size: u64,
        file_name: impl Into<String>,
        mime: Option<Mime>,
    ) -> BatchUploadJob {
        BatchUploadJob {
            key: self.key,
            upload_token: self.upload_token,
            vars: self.vars,
            metadata: self.metadata,
            checksum_enabled: self.checksum_enabled,
            resumable_policy: self.resumable_policy,
            on_uploading_progress: self.on_uploading_progress,
            on_completed: self.on_completed,
            file_name: file_name.into(),
            mime,
            expected_stream_size: size,
            target: BatchUploadTarget::Stream(Box::new(stream)),
        }
    }
}

impl BatchUploadJob {
    fn use_resumeable_uploader(&self, config: &Config) -> bool {
        match self.resumable_policy {
            Some(ResumablePolicy::Never) => false,
            Some(ResumablePolicy::Always) => true,
            Some(ResumablePolicy::Threshold(threshold)) => {
                self.expected_stream_size == 0 || self.expected_stream_size > threshold.into()
            }
            None => self.expected_stream_size == 0 || self.expected_stream_size > config.upload_threshold().into(),
        }
    }
}
