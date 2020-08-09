use super::{error::qiniu_ng_err_t, qiniu_reader::FileReader};
use libc::{c_void, size_t, FILE};
use qiniu_etag::{
    etag_to_buf, etag_with_parts_to_buf, Etag, FixedOutput, GenericArray, Reset, Update,
};
use qiniu_ffi_struct_macros::FFIStruct;
use std::{mem::ManuallyDrop, slice::from_raw_parts};

/// Etag 字符串固定长度
pub const ETAG_SIZE: usize = 28;

/// @brief 七牛 Etag 计算器
/// @details 可以多次接受输入数据以计算七牛 Etag
/// @note
///   * 调用 `qiniu_ng_etag_new()` 函数创建 `qiniu_ng_etag_t` 实例。
///   * 随即可以多次调用 `qiniu_ng_etag_update()` 函数输入数据。
///   * 最终调用 `qiniu_ng_etag_result()` 函数获取计算结果。
///   * 当 `qiniu_ng_etag_t` 使用完毕后，请务必调用 `qiniu_ng_etag_free()` 方法释放内存。
#[repr(C)]
#[derive(Copy, Clone, PartialEq, FFIStruct)]
#[ffi_wrap(Box, Etag)]
pub struct qiniu_ng_etag_t(*mut c_void);

/// @brief 创建 七牛 Etag 计算器实例
/// @param[in] version 七牛 Etag 计算器版本，可选值为 1 或 2
/// @retval qiniu_ng_etag_t 获取创建的七牛 Etag 计算器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_etag_free()` 方法释放 `qiniu_ng_etag_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_new(version: u8) -> qiniu_ng_etag_t {
    Box::new(Etag::new(version)).into()
}

/// @brief 向七牛 Etag 计算器实例输入数据
/// @param[in] etag 七牛 Etag 计算器实例
/// @param[in] data 输入数据地址
/// @param[in] data_len 输入数据长度
/// @note 多次调用该方法可以多次输入数据
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_update(
    etag: qiniu_ng_etag_t,
    data: *const c_void,
    data_len: size_t,
) {
    let mut etag = ManuallyDrop::new(Option::<Box<Etag>>::from(etag).unwrap());
    etag.update(unsafe { from_raw_parts(data.cast(), data_len) });
}

/// @brief 从七牛 Etag 计算器获取结果
/// @param[in] etag 七牛 Etag 计算器实例
/// @param[out] result_ptr 用于返回 Etag 的内存地址，这里 `result_ptr` 必须不能为 `NULL`
/// @warning 保证提供给 `result_ptr` 至少 ETAG_SIZE 长度的内存
/// @note 该函数总是返回正确的结果
/// @note 该方法调用后，七牛 Etag 计算器实例将被自动重置，可以重新输入新的数据
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_result(etag: qiniu_ng_etag_t, result_ptr: *mut c_void) {
    let mut etag = ManuallyDrop::new(Option::<Box<Etag>>::from(etag).unwrap());
    let result_ptr: &mut [u8; ETAG_SIZE] = unsafe { &mut *(result_ptr as *mut [u8; ETAG_SIZE]) };
    etag.finalize_into_reset(GenericArray::from_mut_slice(result_ptr));
}

/// @brief 重置七牛 Etag 计算器实例
/// @param[in] etag 七牛 Etag 计算器实例
/// @note 该函数总是返回正确的结果
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_reset(etag: qiniu_ng_etag_t) {
    let mut etag = ManuallyDrop::new(Option::<Box<Etag>>::from(etag).unwrap());
    etag.reset();
}

/// @brief 释放 七牛 Etag 计算器实例
/// @param[in,out] etag 七牛 Etag 计算器实例地址，释放完毕后该计算器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_free(etag: *mut qiniu_ng_etag_t) {
    if let Some(etag) = unsafe { etag.as_mut() } {
        let _ = Option::<Box<Etag>>::from(*etag);
        *etag = Default::default();
    }
}

/// @brief 判断 七牛 Etag 计算器实例是否已经被释放
/// @param[in] etag 七牛 Etag 计算器实例
/// @retval bool 如果返回 `true` 则表示七牛 Etag 计算器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_is_freed(etag: qiniu_ng_etag_t) -> bool {
    etag.is_null()
}

/// @brief 计算文件的 七牛 Etag V1
/// @param[in] file 文件
/// @param[out] result_ptr 用于返回 Etag V1 的内存地址，如果传入 `NULL` 表示不获取 `result_ptr`。但如果运行正常，返回值将依然是 `true`
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `result_ptr` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 保证提供给 `result_ptr` 至少 `ETAG_SIZE` 长度的内存，除非 `result_ptr` 为 `NULL`
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v1_of_file(
    file: *mut FILE,
    result_ptr: *mut c_void,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let mut not_used = [0; ETAG_SIZE];
    let result_ptr: &mut [u8; ETAG_SIZE] = if result_ptr.is_null() {
        &mut not_used
    } else {
        unsafe { &mut *(result_ptr as *mut [u8; ETAG_SIZE]) }
    };
    match etag_to_buf(FileReader::new(file), result_ptr) {
        Ok(_) => true,
        Err(err) => {
            if let Some(e) = unsafe { error.as_mut() } {
                *e = err.into();
            }
            false
        }
    }
}

/// @brief 计算文件的 七牛 Etag V2
/// @param[in] file 文件
/// @param[in] parts 数据块尺寸数组地址
/// @param[in] parts_len 数据块尺寸数组大小
/// @param[out] result_ptr 用于返回 Etag V2 的内存地址，如果传入 `NULL` 表示不获取 `result_ptr`。但如果运行正常，返回值将依然是 `true`
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `result_ptr` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 保证提供给 `result_ptr` 至少 `ETAG_SIZE` 长度的内存，除非 `result_ptr` 为 `NULL`
#[no_mangle]
pub extern "C" fn qiniu_ng_etag_v2_of_file(
    file: *mut FILE,
    parts: *const usize,
    parts_len: usize,
    result_ptr: *mut c_void,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let mut not_used = [0; ETAG_SIZE];
    let result_ptr: &mut [u8; ETAG_SIZE] = if result_ptr.is_null() {
        &mut not_used
    } else {
        unsafe { &mut *(result_ptr as *mut [u8; ETAG_SIZE]) }
    };
    let parts = unsafe { from_raw_parts(parts, parts_len) };
    match etag_with_parts_to_buf(FileReader::new(file), parts, result_ptr) {
        Ok(_) => true,
        Err(err) => {
            if let Some(e) = unsafe { error.as_mut() } {
                *e = err.into();
            }
            false
        }
    }
}
