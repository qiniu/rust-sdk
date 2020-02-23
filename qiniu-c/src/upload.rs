use crate::{
    bucket::qiniu_ng_bucket_t,
    config::qiniu_ng_config_t,
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr, UCString},
    upload_token::qiniu_ng_upload_token_t,
    utils::{qiniu_ng_readable_t, qiniu_ng_str_map_t, qiniu_ng_str_t},
};
use libc::{c_void, ferror, fread, size_t, FILE};
use mime::Mime;
use qiniu_ng::storage::{
    bucket::Bucket,
    uploader::{
        BucketUploader, FileUploaderBuilder, UploadManager, UploadResponse as QiniuUploadResponse,
        UploadResult as QiniuUploadResult, UploadToken,
    },
};
use std::{
    collections::{hash_map::RandomState, HashMap},
    io::{Error, ErrorKind, Read, Result},
    mem::{drop, transmute},
    ptr::{copy_nonoverlapping, null_mut},
};
use tap::TapOps;

/// @brief 上传管理器
/// @details 上传管理器更接近于一个上传入口，帮助构建存储空间上传器或文件上传器，而本身并不具有实质管理功能
/// @note
///   * 调用 `qiniu_ng_upload_manager_new()` 函数创建 `qiniu_ng_upload_manager_t` 实例。
///   * 当 `qiniu_ng_upload_manager_t` 使用完毕后，请务必调用 `qiniu_ng_upload_manager_free()` 方法释放内存。
/// @note
///   该结构体内部状态不可变，因此可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_upload_manager_t(*mut c_void);

impl Default for qiniu_ng_upload_manager_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_upload_manager_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_upload_manager_t> for Option<Box<UploadManager>> {
    fn from(upload_manager: qiniu_ng_upload_manager_t) -> Self {
        if upload_manager.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(upload_manager)) })
        }
    }
}

impl From<Option<Box<UploadManager>>> for qiniu_ng_upload_manager_t {
    fn from(upload_manager: Option<Box<UploadManager>>) -> Self {
        upload_manager
            .map(|upload_manager| upload_manager.into())
            .unwrap_or_default()
    }
}

impl From<Box<UploadManager>> for qiniu_ng_upload_manager_t {
    fn from(upload_manager: Box<UploadManager>) -> Self {
        unsafe { transmute(Box::into_raw(upload_manager)) }
    }
}

/// @brief 创建上传管理器实例
/// @param[in] config 七牛客户端配置
/// @retval qiniu_ng_upload_manager_t 获取创建的上传管理器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_upload_manager_free()` 方法释放 `qiniu_ng_upload_manager_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_new(config: qiniu_ng_config_t) -> qiniu_ng_upload_manager_t {
    Box::new(UploadManager::new(config.get_clone().unwrap())).into()
}

/// @brief 释放上传管理器实例
/// @param[in,out] upload_manager 上传管理器实例地址，释放完毕后该上传管理器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_free(upload_manager: *mut qiniu_ng_upload_manager_t) {
    if let Some(upload_manager) = unsafe { upload_manager.as_mut() } {
        let _ = Option::<Box<UploadManager>>::from(*upload_manager);
        *upload_manager = qiniu_ng_upload_manager_t::default();
    }
}

/// @brief 判断上传管理器实例是否已经被释放
/// @param[in] upload_manager 上传管理器实例
/// @retval bool 如果返回 `true` 则表示上传管理器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_is_freed(upload_manager: qiniu_ng_upload_manager_t) -> bool {
    upload_manager.is_null()
}

/// @brief 存储空间上传器
/// @details 为指定存储空间的上传准备初始化数据，可以反复使用以上传多个文件
/// @note
///   * 调用 `qiniu_ng_bucket_uploader_new_from_bucket()` 或 `qiniu_ng_bucket_uploader_new_from_bucket_name()` 函数创建 `qiniu_ng_bucket_uploader_t` 实例。
///   * 当 `qiniu_ng_bucket_uploader_t` 使用完毕后，请务必调用 `qiniu_ng_bucket_uploader_free()` 方法释放内存。
/// @note
///   该结构体内部状态不可变，因此可以跨线程使用，但由于可能会自带线程池，所以不要跨进程使用，否则可能会发生线程池无法使用的问题
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_bucket_uploader_t(*mut c_void);

impl Default for qiniu_ng_bucket_uploader_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_bucket_uploader_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_bucket_uploader_t> for Option<BucketUploader> {
    fn from(bucket_uploader: qiniu_ng_bucket_uploader_t) -> Self {
        if bucket_uploader.is_null() {
            None
        } else {
            Some(unsafe { BucketUploader::from_raw(transmute(bucket_uploader)) })
        }
    }
}

impl From<Option<BucketUploader>> for qiniu_ng_bucket_uploader_t {
    fn from(bucket_uploader: Option<BucketUploader>) -> Self {
        bucket_uploader
            .map(|bucket_uploader| bucket_uploader.into())
            .unwrap_or_default()
    }
}

impl From<BucketUploader> for qiniu_ng_bucket_uploader_t {
    fn from(bucket_uploader: BucketUploader) -> Self {
        unsafe { transmute(bucket_uploader.into_raw()) }
    }
}

/// @brief 创建存储空间上传器实例
/// @param[in] upload_manager 上传管理器实例
/// @param[in] bucket 存储空间实例
/// @param[in] thread_pool_size 上传线程池大小，如果传入 `0`，则使用默认的线程池策略
/// @retval qiniu_ng_bucket_uploader_t 获取创建的存储空间上传器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_bucket_uploader_free()` 方法释放 `qiniu_ng_bucket_uploader_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_new_from_bucket(
    upload_manager: qiniu_ng_upload_manager_t,
    bucket: qiniu_ng_bucket_t,
    thread_pool_size: size_t,
) -> qiniu_ng_bucket_uploader_t {
    let upload_manager = Option::<Box<UploadManager>>::from(upload_manager).unwrap();
    let bucket = Option::<Box<Bucket>>::from(bucket).unwrap();
    let mut bucket_uploader_builder = upload_manager.for_bucket(&bucket).tap(|_| {
        let _ = qiniu_ng_bucket_t::from(bucket);
        let _ = qiniu_ng_upload_manager_t::from(upload_manager);
    });
    if thread_pool_size > 0 {
        bucket_uploader_builder = bucket_uploader_builder.thread_pool_size(thread_pool_size);
    }
    bucket_uploader_builder.build().into()
}

/// @brief 创建存储空间上传器实例
/// @param[in] upload_manager 上传管理器实例
/// @param[in] bucket_name 存储空间名称
/// @param[in] access_key 七牛 Access Key
/// @param[in] thread_pool_size 上传线程池大小，如果传入 `0`，则使用默认的线程池策略
/// @retval qiniu_ng_bucket_uploader_t 获取创建的存储空间上传器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_bucket_uploader_free()` 方法释放 `qiniu_ng_bucket_uploader_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_new_from_bucket_name(
    upload_manager: qiniu_ng_upload_manager_t,
    bucket_name: *const qiniu_ng_char_t,
    access_key: *const qiniu_ng_char_t,
    thread_pool_size: size_t,
) -> qiniu_ng_bucket_uploader_t {
    let upload_manager = Option::<Box<UploadManager>>::from(upload_manager).unwrap();
    let mut bucket_uploader_builder = upload_manager
        .for_bucket_name(
            unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap(),
            unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
        )
        .tap(|_| {
            let _ = qiniu_ng_upload_manager_t::from(upload_manager);
        });
    if thread_pool_size > 0 {
        bucket_uploader_builder = bucket_uploader_builder.thread_pool_size(thread_pool_size);
    }
    bucket_uploader_builder.build().into()
}

/// @brief 释放存储空间上传器实例
/// @param[in,out] bucket_uploader 存储空间上传器实例地址，释放完毕后该上传器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_free(bucket_uploader: *mut qiniu_ng_bucket_uploader_t) {
    if let Some(bucket_uploader) = unsafe { bucket_uploader.as_mut() } {
        let _ = Option::<BucketUploader>::from(*bucket_uploader);
        *bucket_uploader = qiniu_ng_bucket_uploader_t::default();
    }
}

/// @brief 判断存储空间上传器实例是否已经被释放
/// @param[in] bucket_uploader 存储空间上传器实例
/// @retval bool 如果返回 `true` 则表示存储空间上传器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_is_freed(bucket_uploader: qiniu_ng_bucket_uploader_t) -> bool {
    bucket_uploader.is_null()
}

/// @brief 分片上传策略
/// @details 为了防止上传文件的过程中，上传日志文件被多个进程同时修改引发竞争，因此需要在操作日志文件时使用文件锁保护
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[allow(dead_code, non_camel_case_types)]
pub enum qiniu_ng_resumable_policy_t {
    /// @brief 默认断点续传策略，将采用客户端配置中的配置项
    qiniu_ng_resumable_policy_default = 0,
    /// @brief 使用分片上传策略阙值
    /// @details
    ///     对于上传文件的情况，如果文件尺寸大于该值，将自动使用分片上传，否则，使用表单上传。
    ///     单位为字节。
    ///     对于上传数据流的情况，由于无法预知数据尺寸，将总是使用分片上传
    qiniu_ng_resumable_policy_threshold,
    /// @brief 总是使用分片上传
    qiniu_ng_resumable_policy_always_be_resumeable,
    /// @brief 总是使用表单上传
    /// @details
    ///     需要注意的是，虽然表单上传仅需要一次 HTTP 调用，性能优于分片上传，
    ///     但分片上传具有断点续传的特性，以及表单上传会将整个文件内容都加载进内存中，对大文件极不友好。
    ///     因此总是推荐使用默认策略，如果认为默认阙值过小，可以适当提高客户端配置的阙值。
    qiniu_ng_resumable_policy_never_be_resumeable,
}

/// @brief 上传参数
/// @details 该结构是个简单的开放结构体，用于为上传提供可选参数
#[repr(C)]
#[derive(Clone)]
pub struct qiniu_ng_upload_params_t {
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
    pub on_uploading_progress: Option<fn(uploaded: u64, total: u64)>,
    /// @brief 当且仅当 `resumable_policy` 为 `qiniu_ng_resumable_policy_threshold` 才生效，表示设置的上传策略阙值
    pub upload_threshold: u32,
    /// @brief 线程池大小，当大于 `0` 时，将为本次上传创建专用线程池
    pub thread_pool_size: size_t,
    /// @brief 上传文件最大并发度
    pub max_concurrency: size_t,
}

/// @brief 上传指定路径的文件
/// @param[in] bucket_uploader 存储空间上传器
/// @param[in] upload_token 上传凭证实例
/// @param[in] file_path 文件路径
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_upload_file_path(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
    file_path: *const qiniu_ng_char_t,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_upload(
        bucket_uploader,
        upload_token,
        UploadFile::FilePath(file_path),
        params,
        response,
        err,
    )
}

/// @brief 上传文件
/// @param[in] bucket_uploader 存储空间上传器
/// @param[in] upload_token 上传凭证实例
/// @param[in] file 文件实例
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_upload_file(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
    file: *mut FILE,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_upload(
        bucket_uploader,
        upload_token,
        UploadFile::File(file),
        params,
        response,
        err,
    )
}

/// @brief 上传阅读器提供的数据
/// @param[in] bucket_uploader 存储空间上传器
/// @param[in] upload_token 上传凭证实例
/// @param[in] reader 阅读器实例，将不断从阅读器中读取数据并上传
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_upload_reader(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
    reader: qiniu_ng_readable_t,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_upload(
        bucket_uploader,
        upload_token,
        UploadFile::Readable(reader),
        params,
        response,
        err,
    )
}

fn qiniu_ng_upload(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
    upload_file: UploadFile,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let bucket_uploader = Option::<BucketUploader>::from(bucket_uploader).unwrap();
    let upload_token = Option::<Box<UploadToken>>::from(upload_token).unwrap();
    let mut file_uploader = bucket_uploader.upload_token(upload_token.as_ref().to_owned()).tap(|_| {
        let _ = qiniu_ng_upload_token_t::from(upload_token);
    });
    let mut file_name: String = "".into();
    let mut mime: Option<Mime> = None;
    if let Some(params) = unsafe { params.as_ref() } {
        file_uploader = set_params_to_file_uploader(file_uploader, params);
        if let Some(f) = unsafe { params.file_name.as_ref() } {
            file_name = unsafe { UCString::from_ptr(f) }.to_string().unwrap()
        }

        mime = match unsafe { params.mime.as_ref() }
            .map(|mime| unsafe { ucstr::from_ptr(mime) }.to_string().unwrap().parse())
        {
            Some(Ok(mime)) => Some(mime),
            Some(Err(ref e)) => {
                if let Some(err) = unsafe { err.as_mut() } {
                    *err = e.into();
                }
                drop(file_uploader);
                let _ = qiniu_ng_bucket_uploader_t::from(bucket_uploader);
                return false;
            }
            _ => None,
        };
    }
    match upload_file.upload(file_uploader, file_name, mime) {
        Ok(resp) => {
            if let Some(response) = unsafe { response.as_mut() } {
                *response = Box::new(resp).into();
            }
            true
        }
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            false
        }
    }
    .tap(|_| {
        let _ = qiniu_ng_bucket_uploader_t::from(bucket_uploader);
    })
}

fn set_params_to_file_uploader<'n>(
    mut file_uploader: FileUploaderBuilder<'n>,
    params: &qiniu_ng_upload_params_t,
) -> FileUploaderBuilder<'n> {
    if params.thread_pool_size > 0 {
        file_uploader = file_uploader.thread_pool_size(params.thread_pool_size);
    }
    if let Some(key) = unsafe { params.key.as_ref() }.map(|key| unsafe { UCString::from_ptr(key) }.to_string().unwrap())
    {
        file_uploader = file_uploader.key(key);
    }
    {
        let vars = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(params.vars);
        if let Some(vars) = vars.as_ref() {
            for (key, value) in vars.iter() {
                file_uploader = file_uploader.var(key.to_string().unwrap(), value.to_string().unwrap());
            }
        }
        let _ = qiniu_ng_str_map_t::from(vars);
    }
    {
        let metadata = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(params.metadata);
        if let Some(metadata) = metadata.as_ref() {
            for (key, value) in metadata.iter() {
                file_uploader = file_uploader.metadata(key.to_string().unwrap(), value.to_string().unwrap());
            }
        }
        let _ = qiniu_ng_str_map_t::from(metadata);
    }
    file_uploader = if params.checksum_disabled {
        file_uploader.disable_checksum()
    } else {
        file_uploader.enable_checksum()
    };
    match params.resumable_policy {
        qiniu_ng_resumable_policy_t::qiniu_ng_resumable_policy_threshold => {
            file_uploader = file_uploader.upload_threshold(params.upload_threshold);
        }
        qiniu_ng_resumable_policy_t::qiniu_ng_resumable_policy_always_be_resumeable => {
            file_uploader = file_uploader.always_be_resumable();
        }
        qiniu_ng_resumable_policy_t::qiniu_ng_resumable_policy_never_be_resumeable => {
            file_uploader = file_uploader.never_be_resumable();
        }
        qiniu_ng_resumable_policy_t::qiniu_ng_resumable_policy_default => {}
    }
    if let Some(on_uploading_progress) = params.on_uploading_progress {
        file_uploader = file_uploader.on_progress(move |uploaded: u64, total: Option<u64>| {
            (on_uploading_progress)(uploaded, total.unwrap_or(0))
        });
    }
    if params.max_concurrency > 0 {
        file_uploader = file_uploader.max_concurrency(params.max_concurrency);
    }
    file_uploader
}

/// @brief 上传响应
/// @details
///     上传响应实例对上传响应中的响应体进行封装，提供一些辅助方法。
///     当 `qiniu_ng_upload_response_t` 使用完毕后，请务必调用 `qiniu_ng_upload_response_free()` 方法释放内存
/// @note 该结构体内部状态不可变，因此可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_upload_response_t(*mut c_void);

impl Default for qiniu_ng_upload_response_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_upload_response_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_upload_response_t> for Option<Box<QiniuUploadResponse>> {
    fn from(upload_response: qiniu_ng_upload_response_t) -> Self {
        if upload_response.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(upload_response)) })
        }
    }
}

impl From<Option<Box<QiniuUploadResponse>>> for qiniu_ng_upload_response_t {
    fn from(upload_response: Option<Box<QiniuUploadResponse>>) -> Self {
        upload_response
            .map(|upload_response| upload_response.into())
            .unwrap_or_default()
    }
}

impl From<Box<QiniuUploadResponse>> for qiniu_ng_upload_response_t {
    fn from(upload_response: Box<QiniuUploadResponse>) -> Self {
        unsafe { transmute(Box::into_raw(upload_response)) }
    }
}

/// @brief 获取上传响应中的对象名称
/// @param[in] upload_response 上传响应实例
/// @retval qiniu_ng_str_t 对象名称
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_get_key(upload_response: qiniu_ng_upload_response_t) -> qiniu_ng_str_t {
    let upload_response = Option::<Box<QiniuUploadResponse>>::from(upload_response).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_response.key()) }.tap(|_| {
        let _ = qiniu_ng_upload_response_t::from(upload_response);
    })
}

/// @brief 获取上传响应中的校验和字段
/// @param[in] upload_response 上传响应实例
/// @param[out] result_ptr 提供内存地址用于返回校验和字段，如果传入 `NULL` 表示不获取 `result_ptr`。但如果该字段存在，返回值依然是 `true`，且不影响其他字段的获取
/// @param[out] result_size 用于返回校验和字段长度，如果传入 `NULL` 表示不获取 `result_size`。但如果该字段存在，返回值依然是 `true`，且不影响其他字段的获取。该字段一般返回的是 Etag，因此长度一般会等于 `ETAG_SIZE`。如果返回 `0`，则表明该校验和字段并不存在
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_get_hash(
    upload_response: qiniu_ng_upload_response_t,
    result_ptr: *mut c_void,
    result_size: *mut size_t,
) {
    let upload_response = Option::<Box<QiniuUploadResponse>>::from(upload_response).unwrap();
    if let Some(hash) = upload_response.hash().map(|hash| hash.as_bytes()) {
        if let Some(result_size) = unsafe { result_size.as_mut() } {
            *result_size = hash.len();
        }
        if let Some(result_ptr) = unsafe { result_ptr.as_mut() } {
            unsafe { copy_nonoverlapping(hash.as_ptr(), result_ptr as *mut c_void as *mut u8, hash.len()) };
        }
    } else if let Some(result_size) = unsafe { result_size.as_mut() } {
        *result_size = 0;
    }
    let _ = qiniu_ng_upload_response_t::from(upload_response);
}

/// @brief 获取上传响应的字符串
/// @param[in] upload_response 上传响应实例
/// @retval qiniu_ng_str_t 上传响应字符串，一般是 JSON 格式的
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_get_string(upload_response: qiniu_ng_upload_response_t) -> qiniu_ng_str_t {
    let upload_response = Option::<Box<QiniuUploadResponse>>::from(upload_response).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(upload_response.to_string()) }.tap(|_| {
        let _ = qiniu_ng_upload_response_t::from(upload_response);
    })
}

/// @brief 释放上传响应实例
/// @param[in,out] upload_response 上传响应实例地址，释放完毕后该实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_free(upload_response: *mut qiniu_ng_upload_response_t) {
    if let Some(upload_response) = unsafe { upload_response.as_mut() } {
        let _ = Option::<Box<QiniuUploadResponse>>::from(*upload_response);
        *upload_response = qiniu_ng_upload_response_t::default();
    }
}

/// @brief 判断上传响应实例是否已经被释放
/// @param[in] upload_response 上传响应实例
/// @retval bool 如果返回 `true` 则表示上传响应实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_is_freed(upload_response: qiniu_ng_upload_response_t) -> bool {
    upload_response.is_null()
}

enum UploadFile {
    FilePath(*const qiniu_ng_char_t),
    File(*mut FILE),
    Readable(qiniu_ng_readable_t),
}

impl UploadFile {
    fn upload(self, file_uploader: FileUploaderBuilder, file_name: String, mime: Option<Mime>) -> QiniuUploadResult {
        match self {
            UploadFile::FilePath(file_path) => {
                file_uploader.upload_file(unsafe { UCString::from_ptr(file_path) }.to_path_buf(), file_name, mime)
            }
            UploadFile::File(file) => file_uploader.upload_stream(FileReader(file), file_name, mime),
            UploadFile::Readable(reader) => file_uploader.upload_stream(reader, file_name, mime),
        }
    }
}

struct FileReader(*mut FILE);

impl Read for FileReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let have_read = unsafe { fread(buf.as_mut_ptr().cast(), 1, buf.len(), self.0) };
        if have_read < buf.len() && unsafe { ferror(self.0) } != 0 {
            return Err(Error::new(ErrorKind::Other, "ferror() returns non-zero"));
        }
        Ok(have_read)
    }
}
unsafe impl Send for FileReader {}
