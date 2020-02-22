#[cfg(any(feature = "use-libcurl"))]
use curl_sys::CURLcode;

use crate::{
    string::{qiniu_ng_char_t, ucstr, UCString},
    utils::{qiniu_ng_str_free, qiniu_ng_str_t},
};
use libc::{c_char, c_int, c_void, FILE};
use matches::matches;
use qiniu_ng::{
    http::{
        domains_manager::PersistentError, Error as HTTPError, ErrorKind as HTTPErrorKind, HTTPCallerErrorKind,
        RetryKind as HTTPRetryKind,
    },
    storage::{
        manager::DropBucketError,
        uploader::{UploadError, UploadTokenParseError},
    },
};
use std::io::{Error as IOError, ErrorKind as IOErrorKind};
use thiserror::Error;

/// @brief HTTP 重试类型
/// @note 通过返回不同的重试类型，可以让 SDK 采用不同的策略解决当前的错误
#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_retry_kind_t {
    /// 可重试错误，当错误是通讯超时或是其他无法预期的错误时，一般采用这样的重试策略
    qiniu_ng_retry_kind_retryable_error,
    /// 区域不可重试错误，当错误是区域不匹配时，则采用该重试策略，SDK 将直接尝试下一个区域
    qiniu_ng_retry_kind_zone_unretryable_error,
    /// 主机不可重试错误，当错误是当前服务器无法连接，则采用该重试策略，SDK 将直接尝试下一个主机
    qiniu_ng_retry_kind_host_unretryable_error,
    /// 不可重试错误，当错误是客户端报文无法被服务器解析时，则采用该重试策略，SDK 将不再重试，直接出错返回
    qiniu_ng_retry_kind_unretryable_error,
}

impl From<qiniu_ng_retry_kind_t> for HTTPRetryKind {
    fn from(kind: qiniu_ng_retry_kind_t) -> Self {
        match kind {
            qiniu_ng_retry_kind_t::qiniu_ng_retry_kind_retryable_error => Self::RetryableError,
            qiniu_ng_retry_kind_t::qiniu_ng_retry_kind_zone_unretryable_error => Self::ZoneUnretryableError,
            qiniu_ng_retry_kind_t::qiniu_ng_retry_kind_host_unretryable_error => Self::HostUnretryableError,
            qiniu_ng_retry_kind_t::qiniu_ng_retry_kind_unretryable_error => Self::UnretryableError,
        }
    }
}

/// @brief HTTP 重试类型
/// @note 通过返回不同的重试类型，可以让 SDK 采用不同的策略解决当前的错误
#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_curl_error_kind_t {
    /// 解析错误
    qiniu_ng_resolve_error,
    /// 代理错误
    qiniu_ng_proxy_error,
    /// SSL 错误
    qiniu_ng_ssl_error,
    /// 连接错误
    qiniu_ng_connection_error,
    /// 请求错误
    qiniu_ng_request_error,
    /// 响应错误
    qiniu_ng_response_error,
    /// 超时错误
    qiniu_ng_timeout_error,
    /// 未知错误
    qiniu_ng_unknown_error,
}

impl From<qiniu_ng_curl_error_kind_t> for HTTPCallerErrorKind {
    fn from(kind: qiniu_ng_curl_error_kind_t) -> Self {
        match kind {
            qiniu_ng_curl_error_kind_t::qiniu_ng_resolve_error => Self::ResolveError,
            qiniu_ng_curl_error_kind_t::qiniu_ng_proxy_error => Self::ProxyError,
            qiniu_ng_curl_error_kind_t::qiniu_ng_ssl_error => Self::SSLError,
            qiniu_ng_curl_error_kind_t::qiniu_ng_connection_error => Self::ConnectionError,
            qiniu_ng_curl_error_kind_t::qiniu_ng_request_error => Self::RequestError,
            qiniu_ng_curl_error_kind_t::qiniu_ng_response_error => Self::ResponseError,
            qiniu_ng_curl_error_kind_t::qiniu_ng_timeout_error => Self::TimeoutError,
            qiniu_ng_curl_error_kind_t::qiniu_ng_unknown_error => Self::UnknownError,
        }
    }
}

impl From<HTTPCallerErrorKind> for qiniu_ng_curl_error_kind_t {
    fn from(kind: HTTPCallerErrorKind) -> Self {
        match kind {
            HTTPCallerErrorKind::ResolveError => Self::qiniu_ng_resolve_error,
            HTTPCallerErrorKind::ProxyError => Self::qiniu_ng_proxy_error,
            HTTPCallerErrorKind::SSLError => Self::qiniu_ng_ssl_error,
            HTTPCallerErrorKind::ConnectionError => Self::qiniu_ng_connection_error,
            HTTPCallerErrorKind::RequestError => Self::qiniu_ng_request_error,
            HTTPCallerErrorKind::ResponseError => Self::qiniu_ng_response_error,
            HTTPCallerErrorKind::TimeoutError => Self::qiniu_ng_timeout_error,
            HTTPCallerErrorKind::UnknownError => Self::qiniu_ng_unknown_error,
        }
    }
}

/// @brief 非法的上传凭证错误类型
/// @note 请通过调用 `qiniu_ng_invalid_upload_token_error_t` 相关的函数来判定错误具体类型
#[repr(C)]
#[derive(Copy, Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_invalid_upload_token_error_kind_t {
    /// 没有错误
    qiniu_ng_invalid_upload_token_error_kind_none,
    /// 上传凭证格式错误
    qiniu_ng_invalid_upload_token_error_kind_invalid_format,
    /// 上传凭证 Base64 解码错误
    qiniu_ng_invalid_upload_token_error_kind_base64_decode_error(qiniu_ng_str_t),
    /// 上传凭证 JSON 解析错误
    qiniu_ng_invalid_upload_token_error_kind_json_decode_error(qiniu_ng_str_t),
}

/// @brief 非法的上传凭证错误
/// @note
///     对于获取了上传凭证错误的情况，可以依次调用
///         - `qiniu_ng_err_invalid_upload_token_format_extract()`
///         - `qiniu_ng_err_invalid_upload_token_base64_error_extract()`
///         - `qiniu_ng_err_invalid_upload_token_json_error_extract()`
///     判定错误的类型，并获取详细错误信息，同时释放内存。
///     如果确定无需判定错误具体类型，则调用 `qiniu_ng_err_invalid_upload_token_error_ignore()` 直接释放内存
/// @note 该结构体不可以跨线程使用
#[repr(C)]
#[derive(Copy, Debug, Clone)]
pub struct qiniu_ng_invalid_upload_token_error_t(qiniu_ng_invalid_upload_token_error_kind_t);

/// @brief SDK 错误类型
/// @note 请通过调用 `qiniu_ng_err_kind_t` 相关的函数来判定错误具体类型
#[repr(C)]
#[derive(Copy, Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_err_kind_t {
    /// 没有错误
    qiniu_ng_err_kind_none,
    /// 操作系统异常
    qiniu_ng_err_kind_os_error(i32),
    /// IO 异常
    qiniu_ng_err_kind_io_error(qiniu_ng_str_t),
    /// 未预期的重定向错误
    qiniu_ng_err_kind_unexpected_redirect_error,
    /// 用户取消
    qiniu_ng_err_kind_user_canceled,
    /// JSON 错误
    qiniu_ng_err_kind_json_error(qiniu_ng_str_t),
    /// HTTP 状态码错误
    qiniu_ng_err_kind_response_status_code_error(u16, qiniu_ng_str_t),
    /// 未知错误
    qiniu_ng_err_kind_unknown_error(qiniu_ng_str_t),

    /// Curl 错误
    #[cfg(any(feature = "use-libcurl"))]
    qiniu_ng_err_kind_curl_error(CURLcode, qiniu_ng_curl_error_kind_t),
    /* Particular error */
    /// 删除非空存储空间
    qiniu_ng_err_kind_drop_non_empty_bucket_error,
    /// 非法的上传凭证错误
    qiniu_ng_err_kind_invalid_upload_token_error(qiniu_ng_invalid_upload_token_error_t),
    /// 非法的 MIME 类型错误
    qiniu_ng_err_kind_bad_mime_type(qiniu_ng_str_t),
}

impl Default for qiniu_ng_err_kind_t {
    #[inline]
    fn default() -> Self {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_none
    }
}

/// @brief SDK 错误
/// @note
///     对于获取了上传凭证错误的情况，可以依次调用
///         - `qiniu_ng_err_os_error_extract()`
///         - `qiniu_ng_err_io_error_extract()`
///         - `qiniu_ng_err_response_status_code_error_extract()`
///         - `qiniu_ng_err_json_error_extract()`
///         - `qiniu_ng_err_bad_mime_type_error_extract()`
///         - `qiniu_ng_err_curl_error_extract()`
///         - `qiniu_ng_err_unexpected_redirect_error_extract()`
///         - `qiniu_ng_err_drop_non_empty_bucket_error_extract()`
///         - `qiniu_ng_err_user_canceled_error_extract()`
///         - `qiniu_ng_err_invalid_upload_token_extract()`
///         - `qiniu_ng_err_unknown_error_extract()`
///     判定错误的类型，并获取详细错误信息，同时释放内存。
///     如果确定无需判定错误具体类型，则调用 `qiniu_ng_err_ignore()` 直接释放内存
/// @note 该结构体不可以跨线程使用
#[repr(C)]
#[derive(Copy, Debug, Default, Clone)]
pub struct qiniu_ng_err_t(qiniu_ng_err_kind_t);

/// @brief 判定错误是否确实存在
/// @param[in] err SDK 错误实例
/// @retval bool 当错误确实存在时返回 `true`
/// @note 对于并无实际错误存在的错误实例，无须释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_err_any_error(err: &qiniu_ng_err_t) -> bool {
    !matches!(err.0, qiniu_ng_err_kind_t::qiniu_ng_err_kind_none)
}

/// @brief 判定错误是否是操作系统异常，如果是，则释放其内存
/// @param[in] err SDK 错误实例
/// @param[out] code 用于返回操作系统异常号码，如果传入 `NULL` 表示不获取 `code`，但如果错误确实是操作系统异常，返回值依然是 `true` 且内存依然会被释放
/// @retval bool 当错误确实是操作系统异常时返回 `true`
#[no_mangle]
pub extern "C" fn qiniu_ng_err_os_error_extract(err: &mut qiniu_ng_err_t, code: *mut i32) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_os_error(os_error_code) => {
            if let Some(code) = unsafe { code.as_mut() } {
                *code = os_error_code;
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 创建操作系统异常
/// @param[in] code 操作系统异常号码
/// @retval qiniu_ng_err_t 返回创建的操作系统异常
#[no_mangle]
pub extern "C" fn qiniu_ng_err_os_error_new(code: i32) -> qiniu_ng_err_t {
    qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_os_error(code))
}

/// @brief 判定错误是否是 IO 异常，如果是，则释放其内存
/// @details IO 异常是 SDK 对操作系统异常的补充，当 SDK 发生了无法用操作系统错误表示的 IO 异常时，则使用该类型表示
/// @param[in] err SDK 错误实例
/// @param[out] description 用于返回 IO 错误描述，如果传入 `NULL` 表示不获取 `description`，但如果错误确实是 IO 异常，返回值依然是 `true` 且内存依然会被释放
/// @retval bool 当错误确实是 IO 异常时返回 `true`
/// @warning 对于获取的 `description`，使用完毕后应该调用 `qiniu_ng_str_free()` 释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_err_io_error_extract(err: &mut qiniu_ng_err_t, description: *mut qiniu_ng_str_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_io_error(mut error_description) => {
            if let Some(description) = unsafe { description.as_mut() } {
                *description = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 创建 IO 异常
/// @param[in] description IO 异常描述
/// @retval qiniu_ng_err_t 返回创建的 IO 异常
/// @warning 对于创建的 IO 异常，需要自己调用 `qiniu_ng_err_ignore()` 函数释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_err_io_error_new(description: *const qiniu_ng_char_t) -> qiniu_ng_err_t {
    qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_io_error(
        unsafe { ucstr::from_ptr(description) }.to_owned().into(),
    ))
}

/// @brief 判定错误是否是未知异常，如果是，则释放其内存
/// @details 未知异常出现的概率较低，所以一般作为最后一个错误判断函数调用。但不要因为其概率低就不予判定
/// @param[in] err SDK 错误实例
/// @param[out] description 用于返回未知异常描述，如果传入 `NULL` 表示不获取 `description`，但如果错误确实是未知异常，返回值依然是 `true` 且内存依然会被释放
/// @retval bool 当错误确实是未知异常时返回 `true`
/// @warning 对于获取的 `description`，使用完毕后应该调用 `qiniu_ng_str_free()` 释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_err_unknown_error_extract(
    err: &mut qiniu_ng_err_t,
    description: *mut qiniu_ng_str_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(mut error_description) => {
            if let Some(description) = unsafe { description.as_mut() } {
                *description = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 判定错误是否是未预期的重定向错误，如果是，则释放其内存
/// @param[in] err SDK 错误实例
/// @retval bool 当错误确实是未预期的重定向错误时返回 `true`
#[no_mangle]
pub extern "C" fn qiniu_ng_err_unexpected_redirect_error_extract(err: &mut qiniu_ng_err_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_unexpected_redirect_error => {
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 创建未预期的重定向错误
/// @retval qiniu_ng_err_t 返回创建的未预期的重定向错误
#[no_mangle]
pub extern "C" fn qiniu_ng_err_unexpected_redirect_error_new() -> qiniu_ng_err_t {
    qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_unexpected_redirect_error)
}

/// @brief 判定错误是否是用户取消错误，如果是，则释放其内存
/// @param[in] err SDK 错误实例
/// @retval bool 当错误确实是用户取消错误时返回 `true`
#[no_mangle]
pub extern "C" fn qiniu_ng_err_user_canceled_error_extract(err: &mut qiniu_ng_err_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_user_canceled => {
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 创建用户取消错误
/// @retval qiniu_ng_err_t 返回创建的用户取消错误
#[no_mangle]
pub extern "C" fn qiniu_ng_err_user_canceled_error_new() -> qiniu_ng_err_t {
    qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_user_canceled)
}

/// @brief 判定错误是否是 JSON 错误，如果是，则释放其内存
/// @param[in] err SDK 错误实例
/// @param[in] description 用于返回错误描述，如果传入 `NULL` 表示不获取 `description`，但如果错误确实是 JSON 错误，返回值依然是 `true` 且内存依然会被释放
/// @retval bool 当错误确实是 JSON 错误时返回 `true`
/// @warning 对于获取的 `description`，使用完毕后应该调用 `qiniu_ng_str_free()` 释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_err_json_error_extract(err: &mut qiniu_ng_err_t, description: *mut qiniu_ng_str_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_json_error(mut error_description) => {
            if let Some(description) = unsafe { description.as_mut() } {
                *description = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 创建 JSON 错误
/// @param[in] description JSON 错误描述
/// @retval qiniu_ng_err_t 返回创建的 JSON 错误
/// @warning 对于创建的 JSON 错误，需要自己调用 `qiniu_ng_err_ignore()` 函数释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_err_json_error_new(description: *const qiniu_ng_char_t) -> qiniu_ng_err_t {
    qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_json_error(
        unsafe { ucstr::from_ptr(description) }.to_owned().into(),
    ))
}

/// @brief 判定错误是否是 HTTP 状态码错误，如果是，则释放其内存
/// @param[in] err SDK 错误实例
/// @param[out] status_code HTTP 状态码，如果传入 `NULL` 表示不获取 `status_code`，但如果错误确实是 HTTP 状态码错误，返回值依然是 `true` 且内存依然会被释放
/// @param[out] error 用于返回错误描述，如果传入 `NULL` 表示不获取 `error`，但如果错误确实是 HTTP 状态码错误，返回值依然是 `true` 且内存依然会被释放
/// @retval bool 当错误确实是 HTTP 状态码错误时返回 `true`
/// @warning 对于获取的 `error`，使用完毕后应该调用 `qiniu_ng_str_free()` 释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_err_response_status_code_error_extract(
    err: &mut qiniu_ng_err_t,
    status_code: *mut u16,
    error: *mut qiniu_ng_str_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_response_status_code_error(code, mut error_description) => {
            if let Some(status_code) = unsafe { status_code.as_mut() } {
                *status_code = code;
            }
            if let Some(error) = unsafe { error.as_mut() } {
                *error = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 创建 HTTP 状态码错误
/// @param[in] status_code HTTP 状态码
/// @param[in] error HTTP 状态码错误描述
/// @retval qiniu_ng_err_t 返回创建的 HTTP 状态码错误
/// @warning 对于创建的 HTTP 状态码错误，需要自己调用 `qiniu_ng_err_ignore()` 函数释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_err_response_status_code_error_new(
    status_code: u16,
    error: *const qiniu_ng_char_t,
) -> qiniu_ng_err_t {
    qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_response_status_code_error(
        status_code,
        unsafe { ucstr::from_ptr(error) }.to_owned().into(),
    ))
}

/// @brief 判定错误是否是删除非空存储空间，如果是，则释放其内存
/// @param[in] err SDK 错误实例
/// @retval bool 当错误确实是删除非空存储空间时返回 `true`
#[no_mangle]
pub extern "C" fn qiniu_ng_err_drop_non_empty_bucket_error_extract(err: &mut qiniu_ng_err_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_drop_non_empty_bucket_error => {
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 判定错误是否是非法的 MIME 类型错误，如果是，则释放其内存
/// @param[in] err SDK 错误实例
/// @param[out] description 用于返回非法的 MIME 类型错误描述，如果传入 `NULL` 表示不获取 `description`，但如果错误确实是非法的 MIME 类型错误，返回值依然是 `true` 且内存依然会被释放
/// @retval bool 当错误确实是 JSON 错误时返回 `true`
/// @warning 对于获取的 `description`，使用完毕后应该调用 `qiniu_ng_str_free()` 释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_err_bad_mime_type_error_extract(
    err: &mut qiniu_ng_err_t,
    description: *mut qiniu_ng_str_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_bad_mime_type(mut error_description) => {
            if let Some(description) = unsafe { description.as_mut() } {
                *description = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 判定错误是否是 Curl 错误，如果是，则释放其内存
/// @param[in] err SDK 错误实例
/// @param[out] code 用于返回 Curl 错误号码，如果传入 `NULL` 表示不获取 `code`，但如果错误确实是 Curl 错误，返回值依然是 `true` 且内存依然会被释放
/// @param[out] kind 用于返回 Curl 错误类型，如果传入 `NULL` 表示不获取 `kind`，但如果错误确实是 Curl 错误，返回值依然是 `true` 且内存依然会被释放
/// @retval bool 当错误确实是 Curl 错误时返回 `true`
#[cfg(any(feature = "use-libcurl"))]
#[no_mangle]
pub extern "C" fn qiniu_ng_err_curl_error_extract(
    err: &mut qiniu_ng_err_t,
    code: *mut CURLcode,
    kind: *mut qiniu_ng_curl_error_kind_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_curl_error(curl_code, curl_kind) => {
            if let Some(code) = unsafe { code.as_mut() } {
                *code = curl_code;
            }
            if let Some(kind) = unsafe { kind.as_mut() } {
                *kind = curl_kind;
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 创建 Curl 错误
/// @param[in] code Curl 错误代码
/// @retval qiniu_ng_err_t 返回创建的 Curl 错误
#[no_mangle]
pub extern "C" fn qiniu_ng_err_curl_error_new(code: CURLcode, kind: qiniu_ng_curl_error_kind_t) -> qiniu_ng_err_t {
    qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_curl_error(code, kind))
}

/// @brief 判定错误是否是非法的上传凭证错误，如果是，则释放其内存
/// @param[in] err SDK 错误实例
/// @param[out] upload_token_error 用于返回非法的上传凭证错误，如果传入 `NULL` 表示不获取 `upload_token_error`，但如果错误确实是非法的上传凭证错误，返回值依然是 `true` 且内存依然会被释放
/// @retval bool 当错误确实是非法的上传凭证错误时返回 `true`
/// @note 对于返回的 `qiniu_ng_invalid_upload_token_error_t`，需要用相应的方法判定具体错误类型并释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_extract(
    err: &mut qiniu_ng_err_t,
    upload_token_error: *mut qiniu_ng_invalid_upload_token_error_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_invalid_upload_token_error(mut e) => {
            if let Some(upload_token_error) = unsafe { upload_token_error.as_mut() } {
                *upload_token_error = e;
            } else {
                qiniu_ng_err_invalid_upload_token_error_ignore(&mut e);
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 判定错误是非法的上传凭证格式，如果是，则释放其内存
/// @param[in] err 上传凭证错误实例
/// @retval bool 当错误确实是非法的上传凭证格式时返回 `true`
#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_format_extract(
    err: &mut qiniu_ng_invalid_upload_token_error_t,
) -> bool {
    match err.0 {
        qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_invalid_format => {
            err.0 = qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 判定错误是上传凭证 Base64 解码错误，如果是，则释放其内存
/// @param[in] err 上传凭证错误实例
/// @param[out] error 用于返回错误描述，如果传入 `NULL` 表示不获取 `error`，但如果错误确实是上传凭证 Base64 解码错误，返回值依然是 `true` 且内存依然会被释放
/// @retval bool 当错误确实是上传凭证 Base64 解码错误时返回 `true`
/// @warning 对于获取的 `error`，使用完毕后应该调用 `qiniu_ng_str_free()` 释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_base64_error_extract(
    err: &mut qiniu_ng_invalid_upload_token_error_t,
    error: *mut qiniu_ng_str_t,
) -> bool {
    match err.0 {
        qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_base64_decode_error(
            mut error_description,
        ) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 判定错误是上传凭证 JSON 解析错误，如果是，则释放其内存
/// @param[in] err 上传凭证错误实例
/// @param[out] error 用于返回错误描述，如果传入 `NULL` 表示不获取 `error`，但如果错误确实是上传凭证 JSON 解析错误，返回值依然是 `true` 且内存依然会被释放
/// @retval bool 当错误确实是上传凭证 JSON 解析错误时返回 `true`
/// @warning 对于获取的 `error`，使用完毕后应该调用 `qiniu_ng_str_free()` 释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_json_error_extract(
    err: &mut qiniu_ng_invalid_upload_token_error_t,
    error: *mut qiniu_ng_str_t,
) -> bool {
    match err.0 {
        qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_json_decode_error(
            mut error_description,
        ) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_none;
            true
        }
        _ => false,
    }
}

/// @brief 输出错误信息
/// @param[in] stream 输出流
/// @param[in] format 输出格式，采用 `fprintf` 语法，本函数向该格式输出一个字符串类型的参数作为错误信息，因此，如果该参数设置为 `"%s"` 将会直接输出错误信息，而 `"%s\n"` 将会输出错误信息并换行
/// @param[in] err SDK 错误实例
#[cfg(not(windows))]
#[no_mangle]
pub extern "C" fn qiniu_ng_err_fprintf(stream: *mut FILE, format: *const c_char, err: qiniu_ng_err_t) -> c_int {
    if let Some(err_kind) = Option::<HTTPErrorKind>::from(&err.0) {
        let error_description = unsafe { UCString::from_string_unchecked(err_kind.to_string()) };
        unsafe { libc::fprintf(stream, format, error_description.as_ptr() as *mut c_void) }
    } else {
        0
    }
}

/// @brief 忽略错误具体类型，释放内存
/// @param[in,out] err SDK 错误实例地址，释放完毕后该错误实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_err_ignore(err: &mut qiniu_ng_err_t) {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_io_error(mut desc) => qiniu_ng_str_free(&mut desc),
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(mut desc) => qiniu_ng_str_free(&mut desc),
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_json_error(mut desc) => qiniu_ng_str_free(&mut desc),
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_response_status_code_error(_, mut desc) => qiniu_ng_str_free(&mut desc),
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_invalid_upload_token_error(mut err) => {
            qiniu_ng_err_invalid_upload_token_error_ignore(&mut err)
        }
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_bad_mime_type(mut desc) => qiniu_ng_str_free(&mut desc),
        _ => {}
    }
    err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
}

/// @brief 忽略非法的上传凭证错误具体类型，释放内存
/// @param[in,out] err 非法的上传凭证错误实例地址，释放完毕后该错误实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_error_ignore(err: &mut qiniu_ng_invalid_upload_token_error_t) {
    match err.0 {
        qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_base64_decode_error(
            mut desc,
        ) => qiniu_ng_str_free(&mut desc),
        qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_json_decode_error(
            mut desc,
        ) => qiniu_ng_str_free(&mut desc),
        _ => {}
    }
    err.0 = qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_none;
}

impl From<&IOError> for qiniu_ng_err_t {
    fn from(err: &IOError) -> Self {
        qiniu_ng_err_t(
            err.raw_os_error()
                .map(qiniu_ng_err_kind_t::qiniu_ng_err_kind_os_error)
                .unwrap_or_else(|| {
                    qiniu_ng_err_kind_t::qiniu_ng_err_kind_io_error(unsafe {
                        qiniu_ng_str_t::from_string_unchecked(err.to_string())
                    })
                }),
        )
    }
}

impl From<&HTTPError> for qiniu_ng_err_t {
    fn from(err: &HTTPError) -> Self {
        match err.error_kind() {
            HTTPErrorKind::HTTPCallerError(e) => qiniu_ng_err_t(
                #[cfg(any(feature = "use-libcurl"))]
                {
                    e.inner()
                        .downcast_ref::<curl::Error>()
                        .map(|curl_err| {
                            qiniu_ng_err_kind_t::qiniu_ng_err_kind_curl_error(
                                curl_err.code(),
                                e.kind().to_owned().into(),
                            )
                        })
                        .unwrap_or_else(|| {
                            qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(unsafe {
                                qiniu_ng_str_t::from_string_unchecked(err.to_string())
                            })
                        })
                },
                #[cfg(not(feature = "use-libcurl"))]
                {
                    qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(qiniu_ng_str_t::from_str_unchecked(
                        "Unrecognized HTTP Error kind",
                    ))
                },
            ),
            HTTPErrorKind::ResponseStatusCodeError(status_code, error_description) => qiniu_ng_err_t(
                qiniu_ng_err_kind_t::qiniu_ng_err_kind_response_status_code_error(status_code.to_owned(), unsafe {
                    qiniu_ng_str_t::from_str_unchecked(error_description)
                }),
            ),
            HTTPErrorKind::IOError(e) => e.into(),
            HTTPErrorKind::JSONError(e) => qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_json_error(unsafe {
                qiniu_ng_str_t::from_string_unchecked(e.to_string())
            })),
            HTTPErrorKind::UnexpectedRedirect => {
                qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_unexpected_redirect_error)
            }
            HTTPErrorKind::MaliciousResponse => {
                qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(unsafe {
                    qiniu_ng_str_t::from_str_unchecked("Malicious HTTP Response, please try HTTPs protocol")
                }))
            }
            HTTPErrorKind::UserCanceled => qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_user_canceled),
            HTTPErrorKind::UnknownError(e) => {
                qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(unsafe {
                    qiniu_ng_str_t::from_string_unchecked(e.to_string())
                }))
            }
        }
    }
}

impl From<&UploadError> for qiniu_ng_err_t {
    fn from(err: &UploadError) -> Self {
        match err {
            UploadError::IOError(err) => err.into(),
            UploadError::QiniuError(err) => err.into(),
        }
    }
}

impl From<&DropBucketError> for qiniu_ng_err_t {
    fn from(err: &DropBucketError) -> Self {
        match err {
            DropBucketError::CannotDropNonEmptyBucket => {
                qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_drop_non_empty_bucket_error)
            }
            DropBucketError::HTTPError(e) => e.into(),
        }
    }
}

impl From<&UploadTokenParseError> for qiniu_ng_err_t {
    fn from(err: &UploadTokenParseError) -> Self {
        qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_invalid_upload_token_error(
            qiniu_ng_invalid_upload_token_error_t(match err {
                UploadTokenParseError::InvalidUploadTokenFormat => {
                    qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_invalid_format
                }
                UploadTokenParseError::Base64DecodeError(e) => {
                    qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_base64_decode_error(unsafe {
                        qiniu_ng_str_t::from_string_unchecked(e.to_string())
                    })
                }
                UploadTokenParseError::JSONDecodeError(e) => {
                    qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_json_decode_error(unsafe {
                        qiniu_ng_str_t::from_string_unchecked(e.to_string())
                    })
                }
            }),
        ))
    }
}

impl From<&mime::FromStrError> for qiniu_ng_err_t {
    fn from(e: &mime::FromStrError) -> Self {
        qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_bad_mime_type(unsafe {
            qiniu_ng_str_t::from_string_unchecked(e.to_string())
        }))
    }
}

impl From<&PersistentError> for qiniu_ng_err_t {
    fn from(err: &PersistentError) -> Self {
        match err {
            PersistentError::JSONError(e) => {
                qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_json_error(unsafe {
                    qiniu_ng_str_t::from_string_unchecked(e.to_string())
                }))
            }
            PersistentError::IOError(ref err) => err.into(),
        }
    }
}

impl From<&serde_json::Error> for qiniu_ng_err_t {
    fn from(err: &serde_json::Error) -> Self {
        qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_json_error(unsafe {
            qiniu_ng_str_t::from_string_unchecked(err.to_string())
        }))
    }
}

impl From<&qiniu_ng_err_kind_t> for Option<HTTPErrorKind> {
    fn from(kind: &qiniu_ng_err_kind_t) -> Self {
        return match kind {
            qiniu_ng_err_kind_t::qiniu_ng_err_kind_none => None,
            qiniu_ng_err_kind_t::qiniu_ng_err_kind_os_error(code) => {
                Some(HTTPErrorKind::IOError(IOError::from_raw_os_error(*code)))
            }
            qiniu_ng_err_kind_t::qiniu_ng_err_kind_io_error(desc) => Some(HTTPErrorKind::IOError(IOError::new(
                IOErrorKind::Other,
                convert_qiniu_ng_str_to_string(*desc),
            ))),
            qiniu_ng_err_kind_t::qiniu_ng_err_kind_unexpected_redirect_error => Some(HTTPErrorKind::UnexpectedRedirect),
            qiniu_ng_err_kind_t::qiniu_ng_err_kind_user_canceled => Some(HTTPErrorKind::UserCanceled),
            qiniu_ng_err_kind_t::qiniu_ng_err_kind_json_error(desc) => {
                Some(HTTPErrorKind::JSONError(convert_qiniu_ng_str_to_string(*desc).into()))
            }
            qiniu_ng_err_kind_t::qiniu_ng_err_kind_response_status_code_error(code, error) => Some(
                HTTPErrorKind::ResponseStatusCodeError(*code, convert_qiniu_ng_str_to_string(*error).into()),
            ),
            qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(desc) => Some(HTTPErrorKind::UnknownError(Box::new(
                StrError::Str(convert_qiniu_ng_str_to_string(*desc).into_boxed_str()),
            ))),
            qiniu_ng_err_kind_t::qiniu_ng_err_kind_curl_error(code, kind) => Some(
                HTTPErrorKind::new_http_caller_error_kind(kind.to_owned().into(), curl::Error::new(*code)),
            ),
            _ => panic!("Cannot convert this error kind: {:?}", kind),
        };

        fn convert_qiniu_ng_str_to_string(s: qiniu_ng_str_t) -> String {
            Option::<Box<ucstr>>::from(s).unwrap().to_string().unwrap()
        }
    }
}

#[derive(Error, Debug)]
enum StrError {
    #[error("{0}")]
    Str(Box<str>),
}

impl From<&qiniu_ng_err_t> for Option<HTTPErrorKind> {
    fn from(kind: &qiniu_ng_err_t) -> Self {
        Self::from(&kind.0)
    }
}
