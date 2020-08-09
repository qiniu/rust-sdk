use super::ucstring::{qiniu_ng_char_t, ucstr, UCString};
use libc::{c_void, size_t};
use qiniu_ffi_struct_macros::FFIStruct;
use std::{
    boxed::Box,
    fmt,
    mem::{transmute, ManuallyDrop},
    ptr::{copy_nonoverlapping, null},
};
use tap::TapOps;

/// @brief 字符串
/// @details 封装一个 C 字符串，字符类型为 `qiniu_ng_char_t`
/// @note
///   * 当 SDK 的 API 返回字符串实例后，调用 `qiniu_ng_str_get_cstr()` 获取内部的 C 字符串。
///   * 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存。
/// @note 该结构体内部状态不可变，因此可以跨线程使用
/// @note 目前在 UNIX 平台上编码为 UTF-8 而 Windows 编码为 UTF-16，但不同编译条件下字符串的编码可能有所不同
/// @warning `qiniu_ng_str_t` 中的字符串有可能是 `NULL`，这种情况下，可以通过 `qiniu_ng_str_is_null()` 判定
/// @warning `qiniu_ng_str_get_cstr()` 将会返回存储的 C 字符串内存地址，请勿修改其存储的字符串内容
#[repr(C)]
#[derive(Copy, Clone, PartialEq, FFIStruct)]
#[ffi_wrap(Box, ucstr)]
pub struct qiniu_ng_str_t(*mut c_void, *mut c_void);

impl From<Option<UCString>> for qiniu_ng_str_t {
    fn from(s: Option<UCString>) -> Self {
        s.map(|s| unsafe { transmute(Box::into_raw(s.into_boxed_ucstr())) })
            .unwrap_or_default()
    }
}

impl From<UCString> for qiniu_ng_str_t {
    #[inline]
    fn from(s: UCString) -> Self {
        Some(s).into()
    }
}

impl From<qiniu_ng_str_t> for Option<UCString> {
    fn from(s: qiniu_ng_str_t) -> Self {
        Option::<Box<ucstr>>::from(s).map(|s| s.into())
    }
}

impl fmt::Debug for qiniu_ng_str_t {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = Option::<Box<ucstr>>::from(*self);
        s.fmt(f).tap(|_| {
            let _ = qiniu_ng_str_t::from(s);
        })
    }
}

/// @brief 判断字符串是否是 NULL
/// @param[in] s 字符串实例
/// @retval bool 如果返回 `true` 则表示字符串是 `NULL`
#[no_mangle]
pub extern "C" fn qiniu_ng_str_is_null(s: qiniu_ng_str_t) -> bool {
    s.is_null()
}

/// @brief 创建新的字符串实例
/// @param[in] ptr 封装的 C 字符串
/// @retval qiniu_ng_str_t 返回字符串实例
#[no_mangle]
pub extern "C" fn qiniu_ng_str_new(ptr: *const qiniu_ng_char_t) -> qiniu_ng_str_t {
    unsafe { ptr.as_ref() }
        .map(|ptr| unsafe { UCString::from_ptr(ptr) }.into_boxed_ucstr().into())
        .unwrap_or_default()
}

/// @brief 推送字符串到字符串实例中
/// @param[in,out] s 字符串实例的指针，可以为空字符串实例，但必须不为 `NULL`
/// @param[in] cs 推送的 C 字符串指针，必须不为 `NULL`
#[no_mangle]
pub extern "C" fn qiniu_ng_str_push_cstr(s: *mut qiniu_ng_str_t, cs: *const qiniu_ng_char_t) {
    let ss = Option::<Box<ucstr>>::from(unsafe { s.read() });
    if let Some(ss) = &ss {
        let mut bytes = ss.as_slice().to_owned();
        bytes.extend_from_slice(unsafe { ucstr::from_ptr(cs) }.as_slice());
        let new_s = unsafe { UCString::from_vec_unchecked(bytes) }.into_boxed_ucstr();
        unsafe { s.write(new_s.into()) };
    } else {
        let ss = qiniu_ng_str_new(cs);
        unsafe { s.write(ss) };
    }
}

/// @brief 获取被封装的 C 字符串指针
/// @param[in] s 字符串实例
/// @retval *qiniu_ng_char_t 返回 C 字符串指针
/// @note 如果封装的 C 字符串地址为 `NULL`，该函数也将返回 `NULL`
/// @warning 该方法将会返回存储的 C 字符串内存地址，请勿修改其存储的字符串内容
#[no_mangle]
pub extern "C" fn qiniu_ng_str_get_cstr(s: qiniu_ng_str_t) -> *const qiniu_ng_char_t {
    let s = Option::<Box<ucstr>>::from(s).map(ManuallyDrop::new);
    s.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null)
}

/// @brief 获取字符串的二进制数据
/// @param[in] s 字符串实例
/// @param[in] max_len 最大返回数据长度，单位为字节
/// @param[out] data_ptr 提供内存地址用于返回数据，如果传入 `NULL` 表示不获取 `data_ptr`。且不影响其他字段的获取
/// @param[out] data_len 用于返回数据实际长度，单位为字节，如果传入 `NULL` 表示不获取 `data_size`。且不影响其他字段的获取。如果返回 `0`，则表明数据并不存在
/// @retval bool 是否获取正常，如果返回 `true`，则表示可以读取 `data_ptr` 和 `data_len` 获得结果，如果返回 `false`，获取二进制数据失败
#[no_mangle]
pub extern "C" fn qiniu_ng_str_get_bytes(
    s: qiniu_ng_str_t,
    max_len: size_t,
    data_ptr: *mut c_void,
    data_len: *mut size_t,
) -> bool {
    let s = Option::<Box<ucstr>>::from(s).map(ManuallyDrop::new);
    let mut result = true;
    if let Some(s) = s.as_ref() {
        match s.to_string() {
            Ok(s) => {
                let data = s.as_bytes();
                if let Some(data_ptr) = unsafe { data_ptr.as_mut() } {
                    let actual_data_len = data.len().min(max_len);
                    unsafe {
                        copy_nonoverlapping(
                            data.as_ptr(),
                            data_ptr as *mut c_void as *mut u8,
                            actual_data_len,
                        )
                    };
                    if let Some(data_len) = unsafe { data_len.as_mut() } {
                        *data_len = actual_data_len;
                    }
                } else if let Some(data_len) = unsafe { data_len.as_mut() } {
                    *data_len = data.len();
                }
            }
            Err(_) => {
                result = false;
            }
        }
    } else if let Some(data_len) = unsafe { data_len.as_mut() } {
        *data_len = 0;
    }
    result
}

/// @brief 获取被封装的 C 字符串长度
/// @param[in] s 字符串实例
/// @retval size_t 返回 C 字符串长度，单位为字节。因此对于 Unicode 编码字符而言，一个字符可能有多个字节组成。
/// @note 如果封装的 C 字符串地址为 `NULL`，该函数将返回 `0`
#[no_mangle]
pub extern "C" fn qiniu_ng_str_get_len(s: qiniu_ng_str_t) -> size_t {
    let s = Option::<Box<ucstr>>::from(s).map(ManuallyDrop::new);
    s.as_ref().map(|s| s.len()).unwrap_or(0)
}

/// @brief 复制字符串实例
/// @param[in] s 字符串实例
/// @retval qiniu_ng_str_t 返回复制的字符串实例。在使用完毕后，两个字符串实例必须分别调用 `qiniu_ng_str_free()` 方法释放内存
/// @note 如果封装的 C 字符串地址为 `NULL`，该函数也会复制出相同的字符串实例
#[no_mangle]
pub extern "C" fn qiniu_ng_str_clone(s: qiniu_ng_str_t) -> qiniu_ng_str_t {
    let s = Option::<Box<ucstr>>::from(s);
    qiniu_ng_str_t::from(s.clone()).tap(|_| {
        let _ = qiniu_ng_str_t::from(s);
    })
}

/// @brief 释放字符串实例
/// @param[in,out] s 字符串实例地址，释放完毕后该字符串实例将不再可用
/// @note 如果封装的 C 字符串地址为 `NULL`，可以无需释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_str_free(s: *mut qiniu_ng_str_t) {
    if let Some(s) = unsafe { s.as_mut() } {
        let _ = Option::<Box<ucstr>>::from(*s);
        *s = Default::default();
    }
}

/// @brief 判断字符串实例是否已经被释放
/// @param[in] s 字符串实例
/// @retval bool 如果返回 `true` 则表示字符串实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_str_is_freed(s: qiniu_ng_str_t) -> bool {
    s.is_null()
}
