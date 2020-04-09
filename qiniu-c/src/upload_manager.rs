use crate::{
    bucket::qiniu_ng_bucket_t,
    config::qiniu_ng_config_t,
    credential::qiniu_ng_credential_t,
    object::qiniu_ng_object_t,
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, UCString},
    upload_response::qiniu_ng_upload_response_t,
    upload_token::{qiniu_ng_upload_policy_t, qiniu_ng_upload_token_t},
    utils::{
        convert_optional_c_string_to_rust_optional_string, convert_optional_c_string_to_rust_string,
        qiniu_ng_readable_t, qiniu_ng_str_map_t, FileReader, QiniuNgStrMap,
    },
};
use libc::{c_void, size_t, FILE};
use mime::Mime;
use qiniu_ng::{
    storage::{
        bucket::Bucket,
        object::Object,
        uploader::{FileUploader, UploadManager, UploadPolicy, UploadResult, UploadToken},
    },
    Config, Credential,
};
use std::{mem::transmute, ptr::null_mut};
use tap::TapOps;

/// @brief 上传管理器
/// @details 上传管理器可以用于构建批量上传器，或直接上传单个文件
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

/// @brief 创建默认上传管理器实例
/// @retval qiniu_ng_upload_manager_t 获取创建的上传管理器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_upload_manager_free()` 方法释放 `qiniu_ng_upload_manager_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_new_default() -> qiniu_ng_upload_manager_t {
    Box::new(UploadManager::new(Config::default())).into()
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
    /// @details
    ///     用于接受上传进度。
    ///     其中第一个参数为已经上传的数据量，单位为字节，第二个参数为需要上传的数据总量，单位为字节。
    ///     如果无法预期需要上传的数据总量，则第二个参数将总是传入 0。
    ///     第三个参数总是传入本结构体的 `callback_data` 字段，您可以根据您的需要为 `callback_data` 字段设置上下文数据。
    ///     该函数无需返回任何值
    /// @warning
    ///     该回调函数可能会被多个线程并发调用，因此需要保证实现的函数线程安全
    pub on_uploading_progress: Option<extern "C" fn(uploaded: u64, total: u64, data: *mut c_void)>,
    /// @brief 回调函数使用的上下文指针
    /// @details
    ///     提供给 `on_uploading_progress` 的 `data` 参数，作为上下文数据使用
    ///     由于回调函数可能被多个线程并发调用，因此需要保证该字段数据的线程安全性
    pub callback_data: *mut c_void,
    /// @brief 当且仅当 `resumable_policy` 为 `qiniu_ng_resumable_policy_threshold` 才生效，表示设置的上传策略阙值
    pub upload_threshold: u32,
    /// @brief 线程池大小，当大于 `0` 时，将为本次上传创建专用线程池
    pub thread_pool_size: size_t,
    /// @brief 上传文件最大并发度
    pub max_concurrency: size_t,
}

#[allow(clippy::large_enum_variant)]
enum UploadTokenPolicy {
    Token(UploadToken),
    Policy {
        upload_policy: UploadPolicy,
        credential: Credential,
    },
    Bucket {
        bucket_name: String,
        credential: Credential,
    },
}

/// @brief 上传指定路径的文件
/// @param[in] upload_manager 上传管理器
/// @param[in] bucket_name 目标存储空间名称
/// @param[in] credential 认证信息
/// @param[in] file_path 文件路径
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_file_path_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_upload_file_path(
    upload_manager: qiniu_ng_upload_manager_t,
    bucket_name: *const qiniu_ng_char_t,
    credential: qiniu_ng_credential_t,
    file_path: *const qiniu_ng_char_t,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_upload_manager_upload_without_upload_token(
        upload_manager,
        bucket_name,
        credential,
        UploadTarget::FilePath(file_path),
        params,
        response,
        err,
    )
}

/// @brief 上传文件
/// @param[in] upload_manager 上传管理器
/// @param[in] bucket_name 目标存储空间名称
/// @param[in] credential 认证信息
/// @param[in] file 文件实例，务必保证文件实例可以读取。上传完毕后，请不要忘记调用 `fclose()` 关闭文件实例
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_file_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_upload_file(
    upload_manager: qiniu_ng_upload_manager_t,
    bucket_name: *const qiniu_ng_char_t,
    credential: qiniu_ng_credential_t,
    file: *mut FILE,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_upload_manager_upload_without_upload_token(
        upload_manager,
        bucket_name,
        credential,
        UploadTarget::File(file),
        params,
        response,
        err,
    )
}

/// @brief 上传阅读器提供的数据
/// @param[in] upload_manager 上传管理器
/// @param[in] bucket_name 目标存储空间名称
/// @param[in] credential 认证信息
/// @param[in] reader 阅读器实例，将不断从阅读器中读取数据并上传
/// @param[in] len 阅读器预期将会读到的最大数据量，如果无法预期则传入 `0`。如果传入的值大于 `0`，最终读取的数据量将始终不大于该值
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_reader_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_upload_reader(
    upload_manager: qiniu_ng_upload_manager_t,
    bucket_name: *const qiniu_ng_char_t,
    credential: qiniu_ng_credential_t,
    reader: qiniu_ng_readable_t,
    len: u64,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_upload_manager_upload_without_upload_token(
        upload_manager,
        bucket_name,
        credential,
        UploadTarget::Readable { reader, len },
        params,
        response,
        err,
    )
}

fn qiniu_ng_upload_manager_upload_without_upload_token(
    upload_manager: qiniu_ng_upload_manager_t,
    bucket_name: *const qiniu_ng_char_t,
    credential: qiniu_ng_credential_t,
    upload_target: UploadTarget,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let upload_manager = Option::<Box<UploadManager>>::from(upload_manager).unwrap();
    let credential = Option::<Box<Credential>>::from(credential).unwrap();
    let bucket_name = unsafe { convert_optional_c_string_to_rust_string(bucket_name) };
    let ok = qiniu_ng_upload_manager_upload(
        &upload_manager,
        UploadTokenPolicy::Bucket {
            bucket_name,
            credential: credential.as_ref().to_owned(),
        },
        upload_target,
        params,
        response,
        err,
    );
    let _ = qiniu_ng_credential_t::from(credential);
    let _ = qiniu_ng_upload_manager_t::from(upload_manager);
    ok
}

/// @brief 使用指定的上传凭证，上传指定路径的文件
/// @param[in] upload_manager 上传管理器
/// @param[in] upload_token 上传凭证实例
/// @param[in] file_path 文件路径
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_upload_file_path_via_upload_token(
    upload_manager: qiniu_ng_upload_manager_t,
    upload_token: qiniu_ng_upload_token_t,
    file_path: *const qiniu_ng_char_t,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_upload_manager_upload_via_upload_token(
        upload_manager,
        upload_token,
        UploadTarget::FilePath(file_path),
        params,
        response,
        err,
    )
}

/// @brief 使用指定的上传凭证，上传文件
/// @param[in] upload_manager 上传管理器
/// @param[in] upload_token 上传凭证实例
/// @param[in] file 文件实例，务必保证文件实例可以读取。上传完毕后，请不要忘记调用 `fclose()` 关闭文件实例
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_upload_file_via_upload_token(
    upload_manager: qiniu_ng_upload_manager_t,
    upload_token: qiniu_ng_upload_token_t,
    file: *mut FILE,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_upload_manager_upload_via_upload_token(
        upload_manager,
        upload_token,
        UploadTarget::File(file),
        params,
        response,
        err,
    )
}

/// @brief 使用指定的上传凭证，上传阅读器提供的数据
/// @param[in] upload_manager 上传管理器
/// @param[in] upload_token 上传凭证实例
/// @param[in] reader 阅读器实例，将不断从阅读器中读取数据并上传
/// @param[in] len 阅读器预期将会读到的最大数据量，如果无法预期则传入 `0`。如果传入的值大于 `0`，最终读取的数据量将始终不大于该值
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_upload_reader_via_upload_token(
    upload_manager: qiniu_ng_upload_manager_t,
    upload_token: qiniu_ng_upload_token_t,
    reader: qiniu_ng_readable_t,
    len: u64,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_upload_manager_upload_via_upload_token(
        upload_manager,
        upload_token,
        UploadTarget::Readable { reader, len },
        params,
        response,
        err,
    )
}

fn qiniu_ng_upload_manager_upload_via_upload_token(
    upload_manager: qiniu_ng_upload_manager_t,
    upload_token: qiniu_ng_upload_token_t,
    upload_target: UploadTarget,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let upload_manager = Option::<Box<UploadManager>>::from(upload_manager).unwrap();
    let upload_token = Option::<UploadToken>::from(upload_token).unwrap();
    let ok = qiniu_ng_upload_manager_upload(
        &upload_manager,
        UploadTokenPolicy::Token(upload_token.to_owned()),
        upload_target,
        params,
        response,
        err,
    );
    let _ = qiniu_ng_upload_token_t::from(upload_token);
    let _ = qiniu_ng_upload_manager_t::from(upload_manager);
    ok
}

/// @brief 使用指定的上传策略，上传指定路径的文件
/// @param[in] upload_manager 上传管理器
/// @param[in] upload_policy 上传策略实例
/// @param[in] credential 认证信息
/// @param[in] file_path 文件路径
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_file_path_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_upload_file_path_via_upload_policy(
    upload_manager: qiniu_ng_upload_manager_t,
    upload_policy: qiniu_ng_upload_policy_t,
    credential: qiniu_ng_credential_t,
    file_path: *const qiniu_ng_char_t,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_upload_manager_upload_via_upload_policy(
        upload_manager,
        upload_policy,
        credential,
        UploadTarget::FilePath(file_path),
        params,
        response,
        err,
    )
}

/// @brief 使用指定的上传策略，上传文件
/// @param[in] upload_manager 上传管理器
/// @param[in] upload_policy 上传策略实例
/// @param[in] credential 认证信息
/// @param[in] file 文件实例，务必保证文件实例可以读取。上传完毕后，请不要忘记调用 `fclose()` 关闭文件实例
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_file_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_upload_file_via_upload_policy(
    upload_manager: qiniu_ng_upload_manager_t,
    upload_policy: qiniu_ng_upload_policy_t,
    credential: qiniu_ng_credential_t,
    file: *mut FILE,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_upload_manager_upload_via_upload_policy(
        upload_manager,
        upload_policy,
        credential,
        UploadTarget::File(file),
        params,
        response,
        err,
    )
}

/// @brief 使用指定的上传策略，上传阅读器提供的数据
/// @param[in] upload_manager 上传管理器
/// @param[in] upload_policy 上传策略实例
/// @param[in] credential 认证信息
/// @param[in] reader 阅读器实例，将不断从阅读器中读取数据并上传
/// @param[in] len 阅读器预期将会读到的最大数据量，如果无法预期则传入 `0`。如果传入的值大于 `0`，最终读取的数据量将始终不大于该值
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_reader_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_upload_reader_via_upload_policy(
    upload_manager: qiniu_ng_upload_manager_t,
    upload_policy: qiniu_ng_upload_policy_t,
    credential: qiniu_ng_credential_t,
    reader: qiniu_ng_readable_t,
    len: u64,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_upload_manager_upload_via_upload_policy(
        upload_manager,
        upload_policy,
        credential,
        UploadTarget::Readable { reader, len },
        params,
        response,
        err,
    )
}

fn qiniu_ng_upload_manager_upload_via_upload_policy(
    upload_manager: qiniu_ng_upload_manager_t,
    upload_policy: qiniu_ng_upload_policy_t,
    credential: qiniu_ng_credential_t,
    upload_target: UploadTarget,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let upload_manager = Option::<Box<UploadManager>>::from(upload_manager).unwrap();
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    let credential = Option::<Box<Credential>>::from(credential).unwrap();
    let ok = qiniu_ng_upload_manager_upload(
        &upload_manager,
        UploadTokenPolicy::Policy {
            upload_policy: upload_policy.as_ref().to_owned(),
            credential: credential.as_ref().to_owned(),
        },
        upload_target,
        params,
        response,
        err,
    );
    let _ = qiniu_ng_credential_t::from(credential);
    let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    let _ = qiniu_ng_upload_manager_t::from(upload_manager);
    ok
}

fn qiniu_ng_upload_manager_upload(
    upload_manager: &UploadManager,
    upload_token_policy: UploadTokenPolicy,
    upload_target: UploadTarget,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let file_uploader_create_result = match upload_token_policy {
        UploadTokenPolicy::Token(upload_token) => upload_manager.upload_for_upload_token(upload_token),
        UploadTokenPolicy::Policy {
            upload_policy,
            credential,
        } => upload_manager.upload_for_upload_policy(upload_policy, credential),
        UploadTokenPolicy::Bucket {
            bucket_name,
            credential,
        } => Ok(upload_manager.upload_for_bucket(bucket_name, credential)),
    };
    match file_uploader_create_result {
        Ok(file_uploader) => qiniu_ng_file_uploader_upload(file_uploader, upload_target, params, response, err),
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            false
        }
    }
}

/// @brief 向指定存储空间上传指定路径的文件
/// @param[in] bucket 目标存储空间实例
/// @param[in] file_path 文件路径
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_file_path_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_upload_file_path(
    bucket: qiniu_ng_bucket_t,
    file_path: *const qiniu_ng_char_t,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_bucket_upload(bucket, UploadTarget::FilePath(file_path), params, response, err)
}

/// @brief 向指定存储空间上传文件
/// @param[in] bucket 目标存储空间
/// @param[in] file 文件实例，务必保证文件实例可以读取。上传完毕后，请不要忘记调用 `fclose()` 关闭文件实例
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_file_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_upload_file(
    bucket: qiniu_ng_bucket_t,
    file: *mut FILE,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_bucket_upload(bucket, UploadTarget::File(file), params, response, err)
}

/// @brief 向指定存储空间上传阅读器提供的数据
/// @param[in] bucket 目标存储空间
/// @param[in] reader 阅读器实例，将不断从阅读器中读取数据并上传
/// @param[in] len 阅读器预期将会读到的最大数据量，如果无法预期则传入 `0`。如果传入的值大于 `0`，最终读取的数据量将始终不大于该值
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_reader_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_upload_reader(
    bucket: qiniu_ng_bucket_t,
    reader: qiniu_ng_readable_t,
    len: u64,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_bucket_upload(bucket, UploadTarget::Readable { reader, len }, params, response, err)
}

fn qiniu_ng_bucket_upload(
    bucket: qiniu_ng_bucket_t,
    upload_target: UploadTarget,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let bucket = Option::<Bucket>::from(bucket).unwrap();
    qiniu_ng_file_uploader_upload(bucket.uploader(), upload_target, params, response, err).tap(|_| {
        let _ = qiniu_ng_bucket_t::from(bucket);
    })
}

/// @brief 向指定对象上传指定路径的文件
/// @param[in] object 目标对象实例
/// @param[in] file_path 文件路径
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_file_path_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_object_upload_file_path(
    object: qiniu_ng_object_t,
    file_path: *const qiniu_ng_char_t,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_object_upload(object, UploadTarget::FilePath(file_path), params, response, err)
}

/// @brief 向指定对象上传文件
/// @param[in] object 目标对象
/// @param[in] file 文件实例，务必保证文件实例可以读取。上传完毕后，请不要忘记调用 `fclose()` 关闭文件实例
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_file_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_object_upload_file(
    object: qiniu_ng_object_t,
    file: *mut FILE,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_object_upload(object, UploadTarget::File(file), params, response, err)
}

/// @brief 向指定对象上传阅读器提供的数据
/// @param[in] object 目标对象
/// @param[in] reader 阅读器实例，将不断从阅读器中读取数据并上传
/// @param[in] len 阅读器预期将会读到的最大数据量，如果无法预期则传入 `0`。如果传入的值大于 `0`，最终读取的数据量将始终不大于该值
/// @param[in] params 上传参数，如果为 `NULL`，则使用默认上传参数
/// @param[out] response 用于返回上传响应，如果传入 `NULL` 表示不获取 `response`。但如果上传成功，返回值将依然是 `true`
/// @param[out] err 用于返回上传错误，如果传入 `NULL` 表示不获取 `err`。但如果上传错误，返回值将依然是 `false`
/// @retval bool 是否上传成功，如果返回 `true`，则表示可以读取 `response` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `response` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
/// @warning 当前函数要求用户认证信息，如果被使用在客户端中，无法获取到认证信息，应该调用 `qiniu_ng_upload_manager_upload_reader_via_upload_token()` 方法替代
#[no_mangle]
pub extern "C" fn qiniu_ng_object_upload_reader(
    object: qiniu_ng_object_t,
    reader: qiniu_ng_readable_t,
    len: u64,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    qiniu_ng_object_upload(object, UploadTarget::Readable { reader, len }, params, response, err)
}

fn qiniu_ng_object_upload(
    object: qiniu_ng_object_t,
    upload_target: UploadTarget,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let object = Option::<Box<Object>>::from(object).unwrap();
    qiniu_ng_file_uploader_upload(object.uploader(), upload_target, params, response, err).tap(|_| {
        let _ = qiniu_ng_object_t::from(object);
    })
}

fn qiniu_ng_file_uploader_upload(
    mut file_uploader: FileUploader,
    upload_target: UploadTarget,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let mut result = true;
    let mut file_name = String::new();
    let mut mime: Option<Mime> = None;
    if let Some(params) = unsafe { params.as_ref() } {
        file_uploader = set_params_to_file_uploader(file_uploader, params);
        file_name = unsafe { convert_optional_c_string_to_rust_string(params.file_name) };
        match parse_mime(params.mime) {
            Some(Ok(parsed_mime)) => {
                mime = Some(parsed_mime);
            }
            Some(Err(ref e)) => {
                if let Some(err) = unsafe { err.as_mut() } {
                    *err = e.into();
                }
                result = false;
            }
            None => {}
        };
    }
    if result {
        match upload_target.upload(file_uploader, file_name, mime) {
            Ok(resp) => {
                if let Some(response) = unsafe { response.as_mut() } {
                    *response = Box::new(resp).into();
                }
            }
            Err(ref e) => {
                if let Some(err) = unsafe { err.as_mut() } {
                    *err = e.into();
                }
                result = false;
            }
        }
    }
    result
}

fn set_params_to_file_uploader<'n>(
    mut file_uploader: FileUploader<'n>,
    params: &qiniu_ng_upload_params_t,
) -> FileUploader<'n> {
    if params.thread_pool_size > 0 {
        file_uploader = file_uploader.thread_pool_size(params.thread_pool_size);
    }
    if let Some(key) = unsafe { convert_optional_c_string_to_rust_optional_string(params.key) } {
        file_uploader = file_uploader.key(key);
    }
    {
        let vars = Option::<QiniuNgStrMap>::from(params.vars);
        if let Some(vars) = vars.as_ref() {
            for (key, value) in vars.iter() {
                file_uploader = file_uploader.var(key.to_string().unwrap(), value.to_string().unwrap());
            }
        }
        let _ = qiniu_ng_str_map_t::from(vars);
    }
    {
        let metadata = Option::<QiniuNgStrMap>::from(params.metadata);
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
        let callback_data = unsafe { params.callback_data.as_ref() }.map(|data| &*data);
        file_uploader = file_uploader.on_progress(move |uploaded: u64, total: Option<u64>| {
            (on_uploading_progress)(
                uploaded,
                total.unwrap_or(0),
                callback_data
                    .map(|data| data as *const c_void as *mut c_void)
                    .unwrap_or_else(null_mut),
            )
        });
    }
    if params.max_concurrency > 0 {
        file_uploader = file_uploader.max_concurrency(params.max_concurrency);
    }
    file_uploader
}

fn parse_mime(mime: *const qiniu_ng_char_t) -> Option<Result<Mime, mime::FromStrError>> {
    unsafe { convert_optional_c_string_to_rust_optional_string(mime) }.map(|mime| mime.parse())
}

enum UploadTarget {
    FilePath(*const qiniu_ng_char_t),
    File(*mut FILE),
    Readable { reader: qiniu_ng_readable_t, len: u64 },
}

impl UploadTarget {
    fn upload(self, file_uploader: FileUploader, file_name: String, mime: Option<Mime>) -> UploadResult {
        match self {
            UploadTarget::FilePath(file_path) => file_uploader.upload_file(
                unsafe { UCString::from_ptr(file_path) }.into_path_buf(),
                file_name,
                mime,
            ),
            UploadTarget::File(file) => {
                let mut reader = FileReader::new(file);
                let guess_file_size = reader.guess_file_size().unwrap_or(0);
                file_uploader.upload_stream(reader, guess_file_size, file_name, mime)
            }
            UploadTarget::Readable { reader, len } => file_uploader.upload_stream(reader, len, file_name, mime),
        }
    }
}
