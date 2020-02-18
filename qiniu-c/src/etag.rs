use super::{
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, UCString},
};
use digest::{FixedOutput, Input, Reset};
use libc::{c_char, c_void, size_t};
use qiniu_ng::utils::etag;
use std::{
    mem::{replace, transmute},
    ptr::{copy_nonoverlapping, null_mut},
    slice::from_raw_parts,
};

pub const ETAG_SIZE: size_t = 28;

/// @brief 计算指定路径的文件的 七牛 Etag
/// @param[in] path 文件路径
/// @param[out] result 用于返回 etag 的内存地址
/// @param[out] error 用于返回错误
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `result` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 保证提供给 `result` 至少 `ETAG_SIZE` 长度的内存
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_from_file_path(
    path: *const qiniu_ng_char_t,
    result: *mut c_char,
    error: *mut qiniu_ng_err_t,
) -> bool {
    match etag::from_file(unsafe { UCString::from_ptr(path) }.into_path_buf()) {
        Ok(etag_string) => {
            let etag_bytes = etag_string.as_bytes();
            unsafe { copy_nonoverlapping(etag_bytes.as_ptr(), result.cast(), etag_bytes.len()) };
            true
        }
        Err(ref err) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = err.into();
            }
            false
        }
    }
}

/// @brief 计算指定二进制数据的 七牛 Etag
/// @param[in] buffer 输入数据地址
/// @param[in] buffer_len 输入数据长度
/// @param[out] result 用于返回 etag 的内存地址
/// @note 该函数总是返回正确的结果
/// @warning 保证提供给 `result` 至少 ETAG_SIZE 长度的内存
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_from_buffer(buffer: *const c_void, buffer_len: size_t, result: *mut c_char) {
    unsafe {
        let e = etag::from_bytes(from_raw_parts(buffer.cast(), buffer_len));
        let e = e.as_bytes();
        copy_nonoverlapping(e.as_ptr(), result.cast(), e.len());
    }
}

/// @brief 七牛 Etag 计算器
/// @details 可以多次接受输入数据以计算七牛 Etag
/// @note
///   * 调用 `qiniu_ng_etag_new()` 函数创建 `qiniu_ng_etag_t` 实例。
///   * 随即可以多次调用 `qiniu_ng_etag_update()` 函数输入数据。
///   * 最终调用 `qiniu_ng_etag_result()` 函数获取计算结果。
///   * 当 `qiniu_ng_etag_t` 使用完毕后，请务必调用 `qiniu_ng_etag_free()` 方法释放内存。
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_etag_t(*mut c_void);

impl Default for qiniu_ng_etag_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_etag_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_etag_t> for Option<Box<etag::Etag>> {
    fn from(etag: qiniu_ng_etag_t) -> Self {
        if etag.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(etag)) })
        }
    }
}

impl From<Box<etag::Etag>> for qiniu_ng_etag_t {
    fn from(etag: Box<etag::Etag>) -> Self {
        unsafe { transmute(Box::into_raw(etag)) }
    }
}

impl From<Option<Box<etag::Etag>>> for qiniu_ng_etag_t {
    fn from(etag: Option<Box<etag::Etag>>) -> Self {
        etag.map(|etag| etag.into()).unwrap_or_default()
    }
}

/// @brief 创建 七牛 Etag 计算器实例
/// @retval qiniu_ng_etag_t 获取创建的七牛 Etag 计算器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_etag_free()` 方法释放 `qiniu_ng_etag_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_new() -> qiniu_ng_etag_t {
    Box::new(etag::new()).into()
}

/// @brief 向七牛 Etag 计算器实例输入数据
/// @param[in] etag 七牛 Etag 计算器实例
/// @param[in] data 输入数据地址
/// @param[in] data_len 输入数据长度
/// @note 多次调用该方法可以多次输入数据
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_update(etag: qiniu_ng_etag_t, data: *const c_void, data_len: size_t) {
    let mut etag = Option::<Box<etag::Etag>>::from(etag).unwrap();
    etag.input(unsafe { from_raw_parts(data.cast(), data_len) });
    let _ = qiniu_ng_etag_t::from(etag);
}

/// @brief 向七牛 Etag 计算器实例输入数据
/// @param[in] etag 七牛 Etag 计算器实例
/// @param[out] result_ptr 用于返回 etag 的内存地址
/// @warning 保证提供给 `result_ptr` 至少 ETAG_SIZE 长度的内存
/// @note 该函数总是返回正确的结果
/// @note 该方法调用后，七牛 Etag 计算器实例将被自动重置，可以重新输入新的数据
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_result(etag: qiniu_ng_etag_t, result_ptr: *mut c_void) {
    let mut etag = Option::<Box<etag::Etag>>::from(etag).unwrap();
    let result = replace(&mut *etag, etag::new()).fixed_result();
    unsafe { copy_nonoverlapping(result.as_ptr(), result_ptr.cast(), ETAG_SIZE) };
    let _ = qiniu_ng_etag_t::from(etag);
}

/// @brief 重置七牛 Etag 计算器实例
/// @param[in] etag 七牛 Etag 计算器实例
/// @note 该函数总是返回正确的结果
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_reset(etag: qiniu_ng_etag_t) {
    let mut etag = Option::<Box<etag::Etag>>::from(etag).unwrap();
    etag.reset();
    let _ = qiniu_ng_etag_t::from(etag);
}

/// @brief 释放 七牛 Etag 计算器实例
/// @param[in,out] etag 七牛 Etag 计算器实例地址，释放完毕后该计算器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_free(etag: *mut qiniu_ng_etag_t) {
    if let Some(etag) = unsafe { etag.as_mut() } {
        let _ = Option::<Box<etag::Etag>>::from(*etag);
        *etag = qiniu_ng_etag_t::default();
    }
}

/// @brief 判断 七牛 Etag 计算器实例是否已经被释放
/// @param[in] etag 七牛 Etag 计算器实例
/// @retval bool 如果返回 `true` 则表示七牛 Etag 计算器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_is_freed(etag: qiniu_ng_etag_t) -> bool {
    etag.is_null()
}
