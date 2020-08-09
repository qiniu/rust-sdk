use super::{
    error::qiniu_ng_err_t,
    http::{header::qiniu_ng_http_headers_t, method::qiniu_ng_http_method_t},
    qiniu_str::qiniu_ng_str_t,
    ucstring::{convert_optional_c_string_to_rust_string, qiniu_ng_char_t, UCString},
};
use libc::c_void;
use qiniu_credential::{Credential, Url};
use qiniu_ffi_struct_macros::FFIStruct;
use qiniu_http::{Headers, HeadersOwned};
use std::{mem::ManuallyDrop, slice::from_raw_parts, time::Duration};

type StaticCredential = Credential<'static>;

/// @brief 七牛认证信息
/// @details 包含七牛的 AccessKey, SecretKey
///   * 调用 `qiniu_ng_credential_new()` 函数创建 `qiniu_ng_credential_t` 实例。
///   * 当 `qiniu_ng_credential_t` 使用完毕后，请务必调用 `qiniu_ng_credential_free()` 方法释放内存。
#[repr(C)]
#[derive(Copy, Clone, PartialEq, FFIStruct)]
#[ffi_wrap(Box, StaticCredential)]
pub struct qiniu_ng_credential_t(*mut c_void);

/// @brief 创建七牛认证信息实例
/// @param[in] access_key 七牛 AccessKey
/// @param[in] secret_key 七牛 SecretKey
/// @retval qiniu_ng_credential_t 获取创建的七牛认证信息实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_credential_free()` 方法释放 `qiniu_ng_credential_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_new(
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
) -> qiniu_ng_credential_t {
    Box::new(StaticCredential::new(
        unsafe { convert_optional_c_string_to_rust_string(access_key) },
        unsafe { convert_optional_c_string_to_rust_string(secret_key) },
    ))
    .into()
}

/// @brief 获取七牛认证信息实例中的 AccessKey
/// @param[in] credential 七牛认证信息实例
/// @retval qiniu_ng_str_t 获取 AccessKey
/// @warning 务必在使用完毕后调用 `qiniu_ng_str_free()` 方法释放 `qiniu_ng_str_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_get_access_key(
    credential: qiniu_ng_credential_t,
) -> qiniu_ng_str_t {
    let credential = ManuallyDrop::new(Option::<Box<StaticCredential>>::from(credential).unwrap());
    unsafe { UCString::from_str_unchecked(credential.access_key()) }.into()
}

/// @brief 获取七牛认证信息实例中的 SecretKey
/// @param[in] credential 七牛认证信息实例
/// @retval qiniu_ng_char_t 获取 SecretKey
/// @warning 务必在使用完毕后调用 `qiniu_ng_str_free()` 方法释放 `qiniu_ng_str_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_get_secret_key(
    credential: qiniu_ng_credential_t,
) -> qiniu_ng_str_t {
    let credential = ManuallyDrop::new(Option::<Box<StaticCredential>>::from(credential).unwrap());
    unsafe { UCString::from_str_unchecked(credential.secret_key()) }.into()
}

/// @brief 使用七牛签名算法对数据进行签名
/// @param[in] credential 七牛认证信息实例
/// @param[in] data_ptr 数据地址
/// @param[in] data_len 数据长度
/// @retval qiniu_ng_str_t 获取签名
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_sign(
    credential: qiniu_ng_credential_t,
    data_ptr: *const c_void,
    data_len: usize,
) -> qiniu_ng_str_t {
    let credential = ManuallyDrop::new(Option::<Box<StaticCredential>>::from(credential).unwrap());
    let signed = credential.sign(unsafe { from_raw_parts(data_ptr.cast(), data_len) });
    unsafe { UCString::from_str_unchecked(signed) }.into()
}

/// @brief 使用七牛签名算法对数据进行签名，并同时给出签名和原数据
/// @param[in] credential 七牛认证信息实例
/// @param[in] data_ptr 数据地址
/// @param[in] data_len 数据长度
/// @retval qiniu_ng_str_t 获取签名和原始数据
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_sign_with_data(
    credential: qiniu_ng_credential_t,
    data_ptr: *const c_void,
    data_len: usize,
) -> qiniu_ng_str_t {
    let credential = ManuallyDrop::new(Option::<Box<StaticCredential>>::from(credential).unwrap());
    let signed = credential.sign_with_data(unsafe { from_raw_parts(data_ptr.cast(), data_len) });
    unsafe { UCString::from_str_unchecked(signed) }.into()
}

macro_rules! parse_url {
    ($ident:ident, $err:ident) => {
        match Url::parse(unsafe { &convert_optional_c_string_to_rust_string($ident) }) {
            Ok(url) => url,
            Err(err) => {
                if let Some(e) = unsafe { $err.as_mut() } {
                    *e = err.into();
                }
                return false;
            }
        }
    };
}

/// @brief 使用七牛签名算法 V1 对 HTTP 请求进行签名，返回 Authorization 的值
/// @param[in] credential 七牛认证信息实例
/// @param[in] url 请求 URL
/// @param[in] content_type 请求 Content-Type
/// @param[in] body_ptr 请求体内容地址
/// @param[in] body_len 请求体内容长度
/// @param[out] result_ptr 用于返回 Authorization 的内存地址，如果传入 `NULL` 表示不获取 `result_ptr`。但如果运行正常，返回值将依然是 `true`
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `result_ptr` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_authorization_v1_for_request(
    credential: qiniu_ng_credential_t,
    url: *const qiniu_ng_char_t,
    content_type: *const qiniu_ng_char_t,
    body_ptr: *const c_void,
    body_len: usize,
    result_ptr: *mut qiniu_ng_str_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let parsed_url = parse_url!(url, error);
    if let Some(result) = unsafe { result_ptr.as_mut() } {
        let credential =
            ManuallyDrop::new(Option::<Box<StaticCredential>>::from(credential).unwrap());
        let content_type = unsafe { &convert_optional_c_string_to_rust_string(content_type) };
        let body: &[u8] = unsafe { from_raw_parts(body_ptr.cast(), body_len) };
        let authorization =
            credential.authorization_v1_for_request(&parsed_url, &content_type, body);
        *result = unsafe { UCString::from_string_unchecked(authorization) }.into();
    }
    true
}

/// @brief 使用七牛签名算法 V2 对 HTTP 请求进行签名，返回 Authorization 的值
/// @param[in] credential 七牛认证信息实例
/// @param[in] url 请求 URL
/// @param[in] method 请求方法
/// @param[in] headers 请求 Headers
/// @param[in] body_ptr 请求体内容地址
/// @param[in] body_len 请求体内容长度
/// @param[out] result_ptr 用于返回 Authorization 的内存地址，如果传入 `NULL` 表示不获取 `result_ptr`。但如果运行正常，返回值将依然是 `true`
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `result_ptr` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_authorization_v2_for_request(
    credential: qiniu_ng_credential_t,
    url: *const qiniu_ng_char_t,
    method: qiniu_ng_http_method_t,
    headers: qiniu_ng_http_headers_t,
    body_ptr: *const c_void,
    body_len: usize,
    result_ptr: *mut qiniu_ng_str_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let parsed_url = parse_url!(url, error);
    if let Some(result) = unsafe { result_ptr.as_mut() } {
        let credential =
            ManuallyDrop::new(Option::<Box<StaticCredential>>::from(credential).unwrap());
        let body: &[u8] = unsafe { from_raw_parts(body_ptr.cast(), body_len) };
        let headers = ManuallyDrop::new(Option::<Box<HeadersOwned>>::from(headers).unwrap());
        let authorization = credential.authorization_v2_for_request(
            method.into(),
            &parsed_url,
            &convert_headers(&headers),
            body,
        );
        *result = unsafe { UCString::from_string_unchecked(authorization) }.into();
    }
    return true;

    fn convert_headers(headers: &HeadersOwned) -> Headers<'static> {
        let mut owned = Headers::<'static>::with_capacity(headers.len());
        for (name, value) in headers.iter() {
            owned.insert(name.to_string().into(), value.to_string().into());
        }
        owned
    }
}

/// @brief 对对象的下载 URL 签名，可以生成私有存储空间的下载地址
/// @param[in] credential 七牛认证信息实例
/// @param[in] url 请求 URL
/// @param[in] deadline URL 有效期，单位为秒
/// @param[out] signed_url 用于返回经过签名的 URL，如果传入 `NULL` 表示不获取 `signed_url`。但如果运行正常，返回值将依然是 `true`
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `signed_url` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_sign_download_url(
    credential: qiniu_ng_credential_t,
    url: *const qiniu_ng_char_t,
    deadline: u64,
    signed_url: *mut qiniu_ng_str_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let mut parsed_url = parse_url!(url, error);
    if let Some(signed_url) = unsafe { signed_url.as_mut() } {
        let credential =
            ManuallyDrop::new(Option::<Box<StaticCredential>>::from(credential).unwrap());
        credential.sign_download_url(&mut parsed_url, Duration::from_secs(deadline));
        *signed_url = unsafe { UCString::from_str_unchecked(parsed_url.as_str()) }.into();
    }
    true
}

/// @brief 释放 七牛认证信息实例
/// @param[in,out] credential 七牛认证信息实例地址，释放完毕后该认证信息实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_free(credential: *mut qiniu_ng_credential_t) {
    if let Some(credential) = unsafe { credential.as_mut() } {
        let _ = Option::<Box<StaticCredential>>::from(*credential);
        *credential = Default::default();
    }
}

/// @brief 判断七牛认证信息实例是否是 NULL
/// @param[in] credential 七牛认证信息实例
/// @retval bool 如果返回 `true` 则表示七牛认证信息实例是 `NULL`
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_is_null(credential: qiniu_ng_credential_t) -> bool {
    credential.is_null()
}
