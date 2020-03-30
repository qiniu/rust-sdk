use crate::{
    bucket_uploader::qiniu_ng_bucket_uploader_t,
    config::qiniu_ng_config_t,
    credential::qiniu_ng_credential_t,
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr, UCString},
    upload::qiniu_ng_resumable_policy_t,
    upload_response::qiniu_ng_upload_response_t,
    upload_token::qiniu_ng_upload_token_t,
    utils::{
        convert_optional_c_string_to_rust_optional_string, convert_optional_c_string_to_rust_string,
        qiniu_ng_readable_t, qiniu_ng_str_map_t, FileReader,
    },
};
use libc::{c_void, size_t, FILE};
use mime::Mime;
use qiniu_ng::{
    storage::uploader::{
        BatchUploadJob, BatchUploadJobBuilder, BatchUploader, BucketUploader, UploadManager, UploadResult, UploadToken,
    },
    Credential,
};
use std::{
    borrow::Cow,
    collections::{hash_map::RandomState, HashMap},
    io::Result as IOResult,
    mem::transmute,
    ptr::null_mut,
};
use tap::TapOps;

/// @brief 批量上传器
/// @details 准备批量上传多个文件或数据流，可以反复使用以上传多个批次的文件或数据
/// @note
///   * 调用 `qiniu_ng_batch_uploader_new_from_bucket_uploader()` 或 `qiniu_ng_batch_uploader_new_from_config` 函数创建 `qiniu_ng_batch_uploader_t` 实例。
///   * 当 `qiniu_ng_batch_uploader_t` 使用完毕后，请务必调用 `qiniu_ng_batch_uploader_free()` 方法释放内存。
/// @note
///   该结构体不可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_batch_uploader_t(*mut c_void);

impl Default for qiniu_ng_batch_uploader_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_batch_uploader_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_batch_uploader_t> for Option<Box<BatchUploader>> {
    fn from(bucket_uploader: qiniu_ng_batch_uploader_t) -> Self {
        if bucket_uploader.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(bucket_uploader)) })
        }
    }
}

impl From<Option<Box<BatchUploader>>> for qiniu_ng_batch_uploader_t {
    fn from(bucket_uploader: Option<Box<BatchUploader>>) -> Self {
        bucket_uploader
            .map(|bucket_uploader| bucket_uploader.into())
            .unwrap_or_default()
    }
}

impl From<Box<BatchUploader>> for qiniu_ng_batch_uploader_t {
    fn from(bucket_uploader: Box<BatchUploader>) -> Self {
        unsafe { transmute(Box::into_raw(bucket_uploader)) }
    }
}

/// @brief 通过存储空间上传器创建批量上传器实例
/// @param[in] bucket_uploader 存储空间上传器实例
/// @param[in] credential 认证信息
/// @param[in] config 客户端配置实例
/// @note 创建实例时，SDK 客户端会复制并存储输入的 `bucket_uploader` 和 `credential`，因此 `bucket_uploader` 和 `credential` 在使用完毕后即可调用各自的内存释放函数释放
/// @warning 务必在使用完毕后调用 `qiniu_ng_batch_uploader_free()` 方法释放 `qiniu_ng_batch_uploader_t`
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_file_path_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_new_from_bucket_uploader(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    credential: qiniu_ng_credential_t,
) -> qiniu_ng_batch_uploader_t {
    let bucket_uploader = Option::<BucketUploader>::from(bucket_uploader).unwrap();
    let credential = Option::<Box<Credential>>::from(credential).unwrap();
    Box::new(bucket_uploader.batch_for(Cow::Borrowed(credential.as_ref())))
        .tap(|_| {
            let _ = qiniu_ng_credential_t::from(credential);
            let _ = qiniu_ng_bucket_uploader_t::from(bucket_uploader);
        })
        .into()
}

/// @brief 通过存储空间上传器创建批量上传器实例，并使用指定上传凭证
/// @param[in] bucket_name 存储空间名称
/// @param[in] credential 认证信息
/// @param[in] upload_token 上传凭证实例。除非为上传任务指定额外的上传凭证，否则所有文件都将使用该上传凭证实例上传
/// @retval qiniu_ng_batch_uploader_t 获取创建的批量上传器实例
/// @note 创建实例时，SDK 客户端会复制并存储输入的 `bucket_uploader`，`credential` 和 `config`，因此 `bucket_uploader`，`credential` 和 `config` 在使用完毕后即可调用各自的内存释放函数释放
/// @warning 务必在使用完毕后调用 `qiniu_ng_batch_uploader_free()` 方法释放 `qiniu_ng_batch_uploader_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_new_from_config(
    bucket_name: *const qiniu_ng_char_t,
    credential: qiniu_ng_credential_t,
    config: qiniu_ng_config_t,
) -> qiniu_ng_batch_uploader_t {
    let bucket_name = unsafe { convert_optional_c_string_to_rust_string(bucket_name) };
    let config = config.get_clone().unwrap();
    let credential = Option::<Box<Credential>>::from(credential).unwrap();
    Box::new(UploadManager::new(config).batch_uploader_for_bucket(bucket_name, Cow::Borrowed(credential.as_ref())))
        .tap(|_| {
            let _ = qiniu_ng_credential_t::from(credential);
        })
        .into()
}

/// @brief 通过存储空间上传器创建批量上传器实例，并使用指定上传凭证
/// @param[in] bucket_uploader 存储空间上传器实例
/// @param[in] upload_token 上传凭证实例。除非为上传任务指定额外的上传凭证，否则所有文件都将使用该上传凭证实例上传
/// @retval qiniu_ng_batch_uploader_t 获取创建的批量上传器实例
/// @note 创建实例时，SDK 客户端会复制并存储输入的 `bucket_uploader` 和 `upload_token`，因此 `bucket_uploader` 和 `upload_token` 在使用完毕后即可调用各自的内存释放函数释放
/// @warning 务必在使用完毕后调用 `qiniu_ng_batch_uploader_free()` 方法释放 `qiniu_ng_batch_uploader_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_new_from_bucket_uploader_with_upload_token(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
) -> qiniu_ng_batch_uploader_t {
    let bucket_uploader = Option::<BucketUploader>::from(bucket_uploader).unwrap();
    let upload_token = Option::<Box<UploadToken>>::from(upload_token).unwrap();
    Box::new(bucket_uploader.batch_for_upload_token(upload_token.to_string()))
        .tap(|_| {
            let _ = qiniu_ng_upload_token_t::from(upload_token);
            let _ = qiniu_ng_bucket_uploader_t::from(bucket_uploader);
        })
        .into()
}

/// @brief 通过客户端配置创建批量上传器实例，并使用指定上传凭证
/// @param[in] upload_token 上传凭证实例。除非为上传任务指定额外的上传凭证，否则所有文件都将使用该上传凭证实例上传
/// @param[in] config 客户端配置实例
/// @param[out] batch_uploader 用于返回批量上传器实例，如果传入 `NULL` 表示不获取 `batch_uploader`。但如果运行正常，返回值将依然是 `true`
/// @retval bool 是否成功创建批量上传器实例，如果失败，表示给出的上传凭证不包含存储空间信息
/// @note 创建实例时，SDK 客户端会复制并存储输入的 `upload_token` 和 `config`，因此 `upload_token` 和 `config` 在使用完毕后即可调用各自的内存释放函数释放
/// @warning 务必在使用完毕后调用 `qiniu_ng_batch_uploader_free()` 方法释放 `qiniu_ng_batch_uploader_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_new_from_config_with_upload_token(
    upload_token: qiniu_ng_upload_token_t,
    config: qiniu_ng_config_t,
    batch_uploader: *mut qiniu_ng_batch_uploader_t,
) -> bool {
    let config = config.get_clone().unwrap();
    let upload_token = Option::<Box<UploadToken>>::from(upload_token).unwrap();
    let mut result = true;
    if let Ok(bu) = UploadManager::new(config).batch_uploader_for_upload_token(upload_token.as_ref().to_owned()) {
        if let Some(batch_uploader) = unsafe { batch_uploader.as_mut() } {
            *batch_uploader = Box::new(bu).into();
        }
    } else {
        result = false;
    }
    let _ = qiniu_ng_upload_token_t::from(upload_token);
    result
}

/// @brief 释放批量上传器实例
/// @param[in,out] bucket_uploader 批量上传器实例地址，释放完毕后该上传器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_free(batch_uploader: *mut qiniu_ng_batch_uploader_t) {
    if let Some(batch_uploader) = unsafe { batch_uploader.as_mut() } {
        let _ = Option::<Box<BatchUploader>>::from(*batch_uploader);
        *batch_uploader = qiniu_ng_batch_uploader_t::default();
    }
}

/// @brief 判断批量上传器实例是否已经被释放
/// @param[in] batch_uploader 批量上传器实例
/// @retval bool 如果返回 `true` 则表示批量上传器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_is_freed(batch_uploader: qiniu_ng_batch_uploader_t) -> bool {
    batch_uploader.is_null()
}

/// @brief 设置批量上传器预期的任务数量
/// @details 如果预先知道上传任务的数量，可以调用该函数预分配内存空间
/// @param[in] batch_uploader 批量上传器实例
/// @param[in] expected_jobs_count 预期即将推送的上传任务数量
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_set_expected_jobs_count(
    batch_uploader: qiniu_ng_batch_uploader_t,
    expected_jobs_count: size_t,
) {
    let mut batch_uploader = Option::<Box<BatchUploader>>::from(batch_uploader).unwrap();
    batch_uploader.expected_jobs_count(expected_jobs_count);
    let _ = qiniu_ng_batch_uploader_t::from(batch_uploader);
}

/// @brief 设置批量上传器线程池数量
/// @details 批量上传器总是优先使用存储空间上传器中的线程池，如果存储空间上传器中没有创建过线程池，则自行创建专用线程池
/// @param[in] batch_uploader 批量上传器实例
/// @param[in] thread_pool_size 上传线程池大小，如果传入 `0`，则使用默认的线程池策略
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_set_thread_pool_size(
    batch_uploader: qiniu_ng_batch_uploader_t,
    thread_pool_size: size_t,
) {
    let mut batch_uploader = Option::<Box<BatchUploader>>::from(batch_uploader).unwrap();
    batch_uploader.thread_pool_size(thread_pool_size);
    let _ = qiniu_ng_batch_uploader_t::from(batch_uploader);
}

/// @brief 设置上传文件最大并发度
/// @details 默认情况下，上传文件时的最大并发度等于其使用的线程池大小。调用该方法可以修改最大并发度
/// @param[in] batch_uploader 批量上传器实例
/// @param[in] max_concurrency 上传文件最大并发度，如果传入 `0`，则使用线程池大小
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_set_max_concurrency(
    batch_uploader: qiniu_ng_batch_uploader_t,
    max_concurrency: size_t,
) {
    let mut batch_uploader = Option::<Box<BatchUploader>>::from(batch_uploader).unwrap();
    batch_uploader.max_concurrency(max_concurrency);
    let _ = qiniu_ng_batch_uploader_t::from(batch_uploader);
}

/// @brief 推送上传指定路径的文件的任务
/// @param[in] batch_uploader 批量上传器实例
/// @param[in] file_path 文件路径
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] err 用于返回任务创建错误，如果传入 `NULL` 表示不获取 `err`。但如果任务创建错误，返回值将依然是 `false`
/// @retval bool 是否创建任务成功，如果返回 `false`，则表示可以读取 `err` 获得错误信息
/// @note 创建上传任务时，SDK 客户端会复制并存储传入的 `file_path`，因此 `file_path` 在使用完毕后即可释放
/// @warning 如果返回 `false`，应该调用相应的错误判断函数对 `err` 的具体错误类型进行判断
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_upload_file_path(
    batch_uploader: qiniu_ng_batch_uploader_t,
    file_path: *const qiniu_ng_char_t,
    params: *const qiniu_ng_batch_upload_params_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_batch_uploader_upload(batch_uploader, UploadTarget::FilePath(file_path), params, err)
}

/// @brief 推送上传文件的任务
/// @param[in] batch_uploader 批量上传器实例
/// @param[in] file 文件实例，务必保证文件实例可以读取。批量上传任务执行完毕后，请不要忘记调用 `fclose()` 关闭文件实例
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] err 用于返回任务创建错误，如果传入 `NULL` 表示不获取 `err`。但如果任务创建错误，返回值将依然是 `false`
/// @retval bool 是否创建任务成功，如果返回 `false`，则表示可以读取 `err` 获得错误信息
/// @warning 如果返回 `false`，应该调用相应的错误判断函数对 `err` 的具体错误类型进行判断
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_upload_file(
    batch_uploader: qiniu_ng_batch_uploader_t,
    file: *mut FILE,
    params: *const qiniu_ng_batch_upload_params_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_batch_uploader_upload(batch_uploader, UploadTarget::File(file), params, err)
}

/// @brief 推送上传阅读器提供的数据的任务
/// @param[in] batch_uploader 批量上传器实例
/// @param[in] reader 阅读器实例，将不断从阅读器中读取数据并上传
/// @param[in] len 阅读器预期将会读到的最大数据量，如果无法预期则传入 `0`。如果传入的值大于 `0`，最终读取的数据量将始终不大于该值
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] err 用于返回任务创建错误，如果传入 `NULL` 表示不获取 `err`。但如果任务创建错误，返回值将依然是 `false`
/// @retval bool 是否创建任务成功，如果返回 `false`，则表示可以读取 `err` 获得错误信息
/// @warning 如果返回 `false`，应该调用相应的错误判断函数对 `err` 的具体错误类型进行判断
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_upload_reader(
    batch_uploader: qiniu_ng_batch_uploader_t,
    reader: qiniu_ng_readable_t,
    len: u64,
    params: *const qiniu_ng_batch_upload_params_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_batch_uploader_upload(batch_uploader, UploadTarget::Readable { reader, len }, params, err)
}

/// @brief 开始执行上传任务
/// @details
///     需要注意的是，该方法会持续阻塞直到上传任务全部执行完毕（不保证执行顺序）。
///     该方法不返回任何结果，上传结果由每个上传任务内定义的 `on_completed` 回调函数负责返回。
///     方法返回后，当前批量上传器的上传任务将被清空，但其他参数都将保留，可以重新添加任务并复用。
/// @param[in] batch_uploader 批量上传器实例
#[no_mangle]
pub extern "C" fn qiniu_ng_batch_uploader_start(batch_uploader: qiniu_ng_batch_uploader_t) {
    let mut batch_uploader = Option::<Box<BatchUploader>>::from(batch_uploader).unwrap();
    batch_uploader.start();
    let _ = qiniu_ng_batch_uploader_t::from(batch_uploader);
}

fn qiniu_ng_batch_uploader_upload(
    batch_uploader: qiniu_ng_batch_uploader_t,
    upload_target: UploadTarget,
    params: *const qiniu_ng_batch_upload_params_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let mut batch_uploader = Option::<Box<BatchUploader>>::from(batch_uploader).unwrap();
    let mut job_builder = BatchUploadJobBuilder::default();
    let mut file_name = String::new();
    let mut mime: Option<Mime> = None;
    if let Some(params) = unsafe { params.as_ref() } {
        job_builder = set_params_to_job_builder(job_builder, params);
        file_name = unsafe { convert_optional_c_string_to_rust_string(params.file_name) };
        mime = match unsafe { convert_optional_c_string_to_rust_optional_string(params.mime) }.map(|mime| mime.parse())
        {
            Some(Ok(mime)) => Some(mime),
            Some(Err(ref e)) => {
                if let Some(err) = unsafe { err.as_mut() } {
                    *err = e.into();
                }
                let _ = qiniu_ng_batch_uploader_t::from(batch_uploader);
                return false;
            }
            _ => None,
        };
    }
    match upload_target.create_batch_job(job_builder, file_name, mime) {
        Ok(job) => {
            batch_uploader.push_job(job);
        }
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            let _ = qiniu_ng_batch_uploader_t::from(batch_uploader);
            return false;
        }
    }
    let _ = qiniu_ng_batch_uploader_t::from(batch_uploader);
    true
}

fn set_params_to_job_builder(
    mut job_builder: BatchUploadJobBuilder,
    params: &qiniu_ng_batch_upload_params_t,
) -> BatchUploadJobBuilder {
    if let Some(key) = unsafe { convert_optional_c_string_to_rust_optional_string(params.key) } {
        job_builder = job_builder.key(key);
    }
    if let Some(upload_token) = unsafe { params.upload_token.as_ref() }
        .and_then(|upload_token| Option::<Box<UploadToken>>::from(upload_token.to_owned()))
    {
        job_builder = job_builder.upload_token(upload_token.to_string());
        let _ = qiniu_ng_upload_token_t::from(upload_token);
    }
    {
        let vars = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(params.vars);
        if let Some(vars) = vars.as_ref() {
            for (key, value) in vars.iter() {
                job_builder = job_builder.var(key.to_string().unwrap(), value.to_string().unwrap());
            }
        }
        let _ = qiniu_ng_str_map_t::from(vars);
    }
    {
        let metadata = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(params.metadata);
        if let Some(metadata) = metadata.as_ref() {
            for (key, value) in metadata.iter() {
                job_builder = job_builder.metadata(key.to_string().unwrap(), value.to_string().unwrap());
            }
        }
        let _ = qiniu_ng_str_map_t::from(metadata);
    }
    job_builder = if params.checksum_disabled {
        job_builder.disable_checksum()
    } else {
        job_builder.enable_checksum()
    };
    match params.resumable_policy {
        qiniu_ng_resumable_policy_t::qiniu_ng_resumable_policy_threshold => {
            job_builder = job_builder.upload_threshold(params.upload_threshold);
        }
        qiniu_ng_resumable_policy_t::qiniu_ng_resumable_policy_always_be_resumeable => {
            job_builder = job_builder.always_be_resumable();
        }
        qiniu_ng_resumable_policy_t::qiniu_ng_resumable_policy_never_be_resumeable => {
            job_builder = job_builder.never_be_resumable();
        }
        qiniu_ng_resumable_policy_t::qiniu_ng_resumable_policy_default => {}
    }
    if let Some(on_uploading_progress) = params.on_uploading_progress {
        let callback_data = unsafe { params.callback_data.as_ref() }.map(|data| &*data);
        job_builder = job_builder.on_progress(move |uploaded: u64, total: Option<u64>| {
            (on_uploading_progress)(
                uploaded,
                total.unwrap_or(0),
                callback_data
                    .map(|data| data as *const c_void as *mut c_void)
                    .unwrap_or_else(null_mut),
            )
        });
    }
    if let Some(on_completed) = params.on_completed {
        let callback_data = unsafe { params.callback_data.as_ref() }.map(|data| &*data);
        job_builder = job_builder.on_completed(move |result: UploadResult| {
            let callback_data = callback_data
                .map(|data| data as *const c_void as *mut c_void)
                .unwrap_or_else(null_mut);
            match result {
                Ok(response) => (on_completed)(Box::new(response).into(), qiniu_ng_err_t::default(), callback_data),
                Err(ref err) => (on_completed)(None.into(), err.into(), callback_data),
            };
        });
    }
    job_builder
}

/// @brief 批量上传参数
/// @details 该结构是个简单的开放结构体，用于为批量上传提供可选参数
#[repr(C)]
#[derive(Clone)]
pub struct qiniu_ng_batch_upload_params_t {
    /// @brief 对象名称，如果不指定，服务器将使用默认的对象名称
    pub key: *const qiniu_ng_char_t,
    /// @brief 指定上传文件的文件名称，在下载文件时将会被使用，如果不指定，SDK 将生成默认的文件名称
    pub file_name: *const qiniu_ng_char_t,
    /// @brief 指定文件的 MIME 类型
    pub mime: *const qiniu_ng_char_t,
    /// @brief 为上传对象指定[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
    pub vars: qiniu_ng_str_map_t,
    /// @brief 为上传对象指定自定义元数据
    pub metadata: qiniu_ng_str_map_t,
    /// @brief 是否禁用文件校验，在任何场景下都不推荐禁用文件校验
    pub checksum_disabled: bool,
    /// @brief 断点续传策略，建议使用默认策略
    pub resumable_policy: qiniu_ng_resumable_policy_t,
    /// @brief 上传进度回调函数
    /// @details
    ///     用于接受上传进度。
    ///     其中第一个参数为已经上传的数据量，单位为字节，第二个参数为需要上传的数据总量，单位为字节。
    ///     如果无法预期需要上传的数据总量，则第二个参数将总是传入 0。
    ///     第三个参数总是传入本结构体的 `callback_data` 字段，您可以根据您的需要为 `callback_data` 字段设置上下文数据。
    ///     该函数无需返回任何值
    /// @warning
    ///     该回调函数可能会被多个线程并发调用，因此需要保证实现的函数线程安全
    pub on_uploading_progress: Option<extern "C" fn(uploaded: u64, total: u64, data: *mut c_void)>,
    /// @brief 上传完成后回调函数
    /// @details
    ///     用于接受上传完成后的结果。
    ///     其中第一个参数为上传成功结果，第二个参数为上传失败时的错误。
    ///     应该首先判断上传是否出错，如果没有出错再处理上传成功的情况。
    ///     第二个参数总是传入本结构体的 `callback_data` 字段，您可以根据您的需要为 `callback_data` 字段设置上下文数据。
    ///     该函数无需返回任何值
    /// @warning
    ///     对于获取的 `response` 和 `err`，一旦使用完毕，应该调用各自的内存释放方法释放内存
    /// @warning
    ///     该回调函数可能会被多个线程并发调用，因此需要保证实现的函数线程安全
    pub on_completed:
        Option<extern "C" fn(response: qiniu_ng_upload_response_t, err: qiniu_ng_err_t, data: *mut c_void)>,
    /// @brief 回调函数使用的上下文指针
    /// @details
    ///     提供给 `on_uploading_progress` 和 `on_completed` 的 `data` 参数，作为上下文数据使用。
    ///     由于回调函数可能被多个线程并发调用，因此需要保证该字段数据的线程安全性
    pub callback_data: *mut c_void,
    /// @brief 指定上传所用的上传凭证
    /// @details
    ///     默认情况下，总是复用批量上传器创建时传入的上传凭证。
    ///     该方法则可以在指定上传当前对象时使用上传凭证
    pub upload_token: *const qiniu_ng_upload_token_t,
    /// @brief 当且仅当 `resumable_policy` 为 `qiniu_ng_resumable_policy_threshold` 才生效，表示设置的上传策略阙值
    pub upload_threshold: u32,
}

unsafe impl Sync for qiniu_ng_batch_upload_params_t {}

enum UploadTarget {
    FilePath(*const qiniu_ng_char_t),
    File(*mut FILE),
    Readable { reader: qiniu_ng_readable_t, len: u64 },
}

impl UploadTarget {
    fn create_batch_job(
        self,
        job_builder: BatchUploadJobBuilder,
        file_name: String,
        mime: Option<Mime>,
    ) -> IOResult<BatchUploadJob> {
        match self {
            UploadTarget::FilePath(file_path) => job_builder.upload_file(
                unsafe { UCString::from_ptr(file_path) }.into_path_buf(),
                file_name,
                mime,
            ),
            UploadTarget::File(file) => {
                let mut reader = FileReader::new(file);
                let guess_file_size = reader.guess_file_size().unwrap_or(0);
                Ok(job_builder.upload_stream(reader, guess_file_size, file_name, mime))
            }
            UploadTarget::Readable { reader, len } => Ok(job_builder.upload_stream(reader, len, file_name, mime)),
        }
    }
}
