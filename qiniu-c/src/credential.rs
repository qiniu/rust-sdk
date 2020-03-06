use crate::{
    string::{qiniu_ng_char_t, ucstr},
    upload_token::qiniu_ng_upload_policy_t,
    utils::qiniu_ng_str_t,
};
use libc::{c_void, size_t};
use qiniu_http::{Method, RequestBuilder};
use qiniu_ng::{storage::uploader::UploadPolicy, Credential};
use std::{mem::transmute, ptr::null_mut, slice::from_raw_parts};
use tap::TapOps;

/// @brief 七牛认证信息
/// @note
///     认证信息仅包含两个信息，Access Key 和 Secret Key
/// @note
///   * 调用 `qiniu_ng_credential_new()` 函数创建 `qiniu_ng_credential_t` 实例。
///   * 当 `qiniu_ng_credential_t` 使用完毕后，请务必调用 `qiniu_ng_credential_free()` 方法释放内存。
/// @note
///   该结构体内部状态不可变，因此可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_credential_t(*mut c_void);

impl Default for qiniu_ng_credential_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_credential_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_credential_t> for Option<Box<Credential>> {
    fn from(credential: qiniu_ng_credential_t) -> Self {
        if credential.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(credential)) })
        }
    }
}

impl From<Option<Box<Credential>>> for qiniu_ng_credential_t {
    fn from(credential: Option<Box<Credential>>) -> Self {
        credential.map(|credential| credential.into()).unwrap_or_default()
    }
}

impl From<Box<Credential>> for qiniu_ng_credential_t {
    fn from(credential: Box<Credential>) -> Self {
        unsafe { transmute(Box::into_raw(credential)) }
    }
}

/// @brief 创建七牛认证信息实例
/// @param[in] access_key 七牛 Access Key
/// @param[in] secret_key 七牛 Secret Key
/// @retval qiniu_ng_credential_t 获取创建的七牛认证信息实例
/// @note 创建实例时，SDK 客户端会复制并存储输入的 `access_key` 和 `secret_key`，因此 `access_key` 和 `secret_key` 在使用完毕后即可释放
/// @warning 务必在使用完毕后调用 `qiniu_ng_credential_free()` 方法释放 `qiniu_ng_credential_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_new(
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
) -> qiniu_ng_credential_t {
    Box::new(Credential::new(
        unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
        unsafe { ucstr::from_ptr(secret_key) }.to_string().unwrap(),
    ))
    .into()
}

/// @brief 获取认证信息的 Access Key
/// @param[in] credential 七牛认证信息实例
/// @retval qiniu_ng_str_t 返回认证信息的 Access Key
/// @warning 对于获取的 Access Key，使用完毕后应该调用 `qiniu_ng_str_free()` 释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_get_access_key(credential: qiniu_ng_credential_t) -> qiniu_ng_str_t {
    let credential = Option::<Box<Credential>>::from(credential).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(credential.access_key()) }.tap(|_| {
        let _ = qiniu_ng_credential_t::from(credential);
    })
}

/// @brief 获取认证信息的 Secret Key
/// @param[in] credential 七牛认证信息实例
/// @retval qiniu_ng_str_t 返回认证信息的 Secret Key
/// @warning 对于获取的 Secret Key，使用完毕后应该调用 `qiniu_ng_str_free()` 释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_get_secret_key(credential: qiniu_ng_credential_t) -> qiniu_ng_str_t {
    let credential = Option::<Box<Credential>>::from(credential).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(credential.secret_key()) }.tap(|_| {
        let _ = qiniu_ng_credential_t::from(credential);
    })
}

/// @brief 使用七牛签名算法对数据进行签名
/// @param[in] credential 七牛认证信息实例
/// @param[in] data 输入数据地址
/// @param[in] data_len 输入数据长度
/// @retval qiniu_ng_str_t 返回签名结果
/// @note 该函数总是返回正确的结果
/// @warning 对于获取的签名结果，使用完毕后应该调用 `qiniu_ng_str_free()` 释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_sign(
    credential: qiniu_ng_credential_t,
    data: *const c_void,
    data_len: size_t,
) -> qiniu_ng_str_t {
    let credential = Option::<Box<Credential>>::from(credential).unwrap();
    let signature = credential.sign(unsafe { from_raw_parts(data.cast(), data_len) });
    unsafe { qiniu_ng_str_t::from_string_unchecked(signature) }.tap(|_| {
        let _ = qiniu_ng_credential_t::from(credential);
    })
}

/// @brief 使用七牛签名算法对数据进行签名，并同时给出签名和原数据
/// @param[in] credential 七牛认证信息实例
/// @param[in] data 输入数据地址
/// @param[in] data_len 输入数据长度
/// @retval qiniu_ng_str_t 返回签名结果，并附带原数据
/// @note 该函数总是返回正确的结果
/// @warning 对于获取的签名结果，使用完毕后应该调用 `qiniu_ng_str_free()` 释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_sign_with_data(
    credential: qiniu_ng_credential_t,
    data: *const c_void,
    data_len: size_t,
) -> qiniu_ng_str_t {
    let credential = Option::<Box<Credential>>::from(credential).unwrap();
    let signature = credential.sign_with_data(unsafe { from_raw_parts(data.cast(), data_len) });
    unsafe { qiniu_ng_str_t::from_string_unchecked(signature) }.tap(|_| {
        let _ = qiniu_ng_credential_t::from(credential);
    })
}

/// @brief 验证七牛回调请求
/// @param[in] credential 七牛认证信息实例
/// @param[in] url 请求 URL
/// @param[in] authorization 请求 Header 中 `Authorization` 的值
/// @param[in] content_type 请求 Header 中 `Content-Type` 的值
/// @param[in] body 请求体数据地址
/// @param[in] body_len 请求体数据长度
/// @retval bool 是否确实是七牛回调请求
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_validate_qiniu_callback_request(
    credential: qiniu_ng_credential_t,
    url: *const qiniu_ng_char_t,
    authorization: *const qiniu_ng_char_t,
    content_type: *const qiniu_ng_char_t,
    body: *const c_void,
    body_len: size_t,
) -> bool {
    let credential = Option::<Box<Credential>>::from(credential).unwrap();
    let mut request_builder = RequestBuilder::default().method(Method::POST);
    if !url.is_null() {
        request_builder = request_builder.url(unsafe { ucstr::from_ptr(url) }.to_string().unwrap());
    }
    if !authorization.is_null() {
        request_builder = request_builder.header(
            "Authorization",
            unsafe { ucstr::from_ptr(authorization) }.to_string().unwrap(),
        );
    }
    if !content_type.is_null() {
        request_builder = request_builder.header(
            "Content-Type",
            unsafe { ucstr::from_ptr(content_type) }.to_string().unwrap(),
        );
    }
    if !body.is_null() && body_len > 0 {
        request_builder = request_builder.body(unsafe { from_raw_parts(body.cast(), body_len) });
    }
    credential.is_valid_request(&request_builder.build()).tap(|_| {
        let _ = qiniu_ng_credential_t::from(credential);
    })
}

/// @brief 对七牛上传策略进行签名，生成上传凭证字符串
/// @param[in] credential 七牛认证信息实例
/// @param[in] upload_policy 上传凭证实例
/// @retval qiniu_ng_str_t 返回上传凭证字符串
/// @warning 务必在使用 `qiniu_ng_str_t` 完毕后调用 `qiniu_ng_str_free()` 方法释放 `qiniu_ng_str_t`
/// @warning 在调用完毕后 `qiniu_ng_upload_policy_t` 依然需要被 `qiniu_ng_upload_policy_free()` 释放
/// @note 该方法直接返回上传凭证字符串而 `qiniu_ng_upload_token_new_from_policy()` 将会返回上传凭证实例
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_sign_upload_policy(
    credential: qiniu_ng_credential_t,
    upload_policy: qiniu_ng_upload_policy_t,
) -> qiniu_ng_str_t {
    let credential = Option::<Box<Credential>>::from(credential).unwrap();
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    let upload_token = credential.sign_upload_policy(&upload_policy);
    unsafe { qiniu_ng_str_t::from_string_unchecked(upload_token) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
        let _ = qiniu_ng_credential_t::from(credential);
    })
}

/// @brief 释放 七牛认证信息实例
/// @param[in,out] credential 七牛认证信息实例地址，释放完毕后该客户端实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_free(credential: *mut qiniu_ng_credential_t) {
    if let Some(credential) = unsafe { credential.as_mut() } {
        let _ = Option::<Box<Credential>>::from(*credential);
        *credential = qiniu_ng_credential_t::default();
    }
}

// TODO: qiniu_ng_credential_is_valid_request

/// @brief 判断 七牛认证信息实例是否已经被释放
/// @param[in] credential 七牛认证信息实例
/// @retval bool 如果返回 `true` 则表示七牛认证信息实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_credential_is_freed(credential: qiniu_ng_credential_t) -> bool {
    credential.is_null()
}
