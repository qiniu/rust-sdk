#[cfg(any(feature = "use-libcurl"))]
use curl_sys::CURLcode;

use crate::utils::{qiniu_ng_str_free, qiniu_ng_str_t};
use matches::matches;
use qiniu_ng::{
    http::{domains_manager::PersistentError, Error as HTTPError, ErrorKind as HTTPErrorKind},
    storage::{
        manager::DropBucketError,
        uploader::{UploadError, UploadTokenParseError},
    },
};
use std::io::Error as IOError;

/// @brief 非法的上传凭证错误类型
/// @note 请通过调用 `qiniu_ng_invalid_upload_token_error_t` 相关的函数来判定错误具体类型
#[repr(C)]
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
pub struct qiniu_ng_invalid_upload_token_error_t(qiniu_ng_invalid_upload_token_error_kind_t);

/// @brief SDK 错误类型
/// @note 请通过调用 `qiniu_ng_err_kind_t` 相关的函数来判定错误具体类型
#[repr(C)]
#[derive(Copy, Clone)]
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

    #[cfg(any(feature = "use-libcurl"))]
    /// Curl 错误
    qiniu_ng_err_kind_curl_error(CURLcode),
    /* Particular error */
    /// 删除非空存储空间
    qiniu_ng_err_kind_cannot_drop_non_empty_bucket_error,
    /// 非法的上传凭证错误
    qiniu_ng_err_kind_invalid_upload_token_error(qiniu_ng_invalid_upload_token_error_t),
    /// 非法的 MIME 类型错误
    qiniu_ng_err_kind_bad_mime_type(qiniu_ng_str_t),
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
#[derive(Copy, Clone)]
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

/// @brief 判定错误是否是 JSON 错误，如果是，则释放其内存
/// @param[in] err SDK 错误实例
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

/// @brief 判定错误是删除非空存储空间，如果是，则释放其内存
/// @param[in] err SDK 错误实例
/// @retval bool 当错误确实是删除非空存储空间时返回 `true`
#[no_mangle]
pub extern "C" fn qiniu_ng_err_drop_non_empty_bucket_error_extract(err: &mut qiniu_ng_err_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_cannot_drop_non_empty_bucket_error => {
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
/// @retval bool 当错误确实是 Curl 错误时返回 `true`
#[cfg(any(feature = "use-libcurl"))]
#[no_mangle]
pub extern "C" fn qiniu_ng_err_curl_error_extract(err: &mut qiniu_ng_err_t, code: *mut CURLcode) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_curl_error(curl_code) => {
            if let Some(code) = unsafe { code.as_mut() } {
                *code = curl_code;
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
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
                        .map(|e| qiniu_ng_err_kind_t::qiniu_ng_err_kind_curl_error(e.code()))
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
                qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_cannot_drop_non_empty_bucket_error)
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
