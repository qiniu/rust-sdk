use super::super::{
    qiniu_str::qiniu_ng_str_t,
    ucstring::{
        convert_optional_c_string_to_rust_optional_string,
        convert_optional_c_string_to_rust_string, qiniu_ng_char_t, UCString,
    },
};
use libc::c_void;
use qiniu_ffi_struct_macros::FFIStruct;
use qiniu_http::HeadersOwned;
use std::mem::ManuallyDrop;

/// @brief 七牛 HTTP Header 实例
/// @details 用于表示七牛请求和响应中的 HTTP Header 实例
///   * 调用 `qiniu_ng_http_headers_new()` 函数创建 `qiniu_ng_http_headers_t` 实例。
///   * 当 `qiniu_ng_http_headers_t` 使用完毕后，请务必调用 `qiniu_ng_http_headers_free()` 方法释放内存。
#[repr(C)]
#[derive(Copy, Clone, PartialEq, FFIStruct)]
#[ffi_wrap(Box, HeadersOwned)]
pub struct qiniu_ng_http_headers_t(*mut c_void);

/// @brief 创建七牛 HTTP Header 实例
/// @retval qiniu_ng_http_headers_t 获取创建的七牛 HTTP Header 实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_http_headers_free()` 方法释放 `qiniu_ng_http_headers_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_http_headers_new() -> qiniu_ng_http_headers_t {
    Box::new(HeadersOwned::new()).into()
}

/// @brief 向七牛 HTTP Header 实例插入新的 Header 键值对，或删除已经存在的 Header 键值对
/// @param[in] headers 七牛 HTTP Header 实例
/// @param[in] name 七牛 HTTP Header 名称
/// @param[in] value 七牛 HTTP Header 值，如果传入 NULL，则表示删除该 Header 键值对
#[no_mangle]
pub extern "C" fn qiniu_ng_http_headers_put(
    headers: qiniu_ng_http_headers_t,
    name: *const qiniu_ng_char_t,
    value: *const qiniu_ng_char_t,
) {
    let mut headers = ManuallyDrop::new(Option::<Box<HeadersOwned>>::from(headers).unwrap());
    let name = unsafe { convert_optional_c_string_to_rust_string(name) };
    if let Some(value) = unsafe { convert_optional_c_string_to_rust_optional_string(value) } {
        headers.insert(name.into(), value);
    } else {
        headers.remove(&name.into());
    }
}

/// @brief 根据名称从七牛 HTTP Header 查询对应的值
/// @param[in] headers 七牛 HTTP Header 实例
/// @param[in] name 七牛 HTTP Header 名称
/// @retval qiniu_ng_str_t 查询到的七牛 HTTP Header 值，对于返回值，始终应该调用 `qiniu_ng_str_is_null` 方法判断返回的值是否是 NULL
#[no_mangle]
pub extern "C" fn qiniu_ng_http_headers_get(
    headers: qiniu_ng_http_headers_t,
    name: *const qiniu_ng_char_t,
) -> qiniu_ng_str_t {
    let headers = ManuallyDrop::new(Option::<Box<HeadersOwned>>::from(headers).unwrap());
    headers
        .get(&unsafe { convert_optional_c_string_to_rust_string(name) }.into())
        .map(|v| unsafe { UCString::from_string_unchecked(v) })
        .into()
}

/// @brief 释放七牛 HTTP Header
/// @param[in,out] headers 七牛 HTTP Header 实例，释放完毕后该实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_http_headers_free(headers: *mut qiniu_ng_http_headers_t) {
    if let Some(headers) = unsafe { headers.as_mut() } {
        let _ = Option::<Box<HeadersOwned>>::from(*headers);
        *headers = qiniu_ng_http_headers_t::default();
    }
}
