use crate::{
    bucket::qiniu_ng_bucket_t,
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr},
    utils::qiniu_ng_str_t,
};
use libc::{c_void, size_t};
use qiniu_ng::storage::{bucket::Bucket, object::Object, resource::ObjectInfo};
use std::{
    mem::transmute,
    ptr::{copy_nonoverlapping, null_mut},
    time::SystemTime,
};
use tap::TapOps;

/// @brief 对象实例
/// @details 用于表示存储空间中的一个对象，可用来获取对象信息或对对象进行操作
/// @note
///   * 调用 `qiniu_ng_object_new()` 函数创建 `qiniu_ng_object_t` 实例。
///   * 当 `qiniu_ng_object_t` 使用完毕后，请务必调用 `qiniu_ng_object_free()` 方法释放内存。
/// @note
///   该结构体可以跨线程使用，SDK 确保其使用的线程安全
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_object_t(*mut c_void);

impl Default for qiniu_ng_object_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_object_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_object_t> for Option<Box<Object>> {
    fn from(object: qiniu_ng_object_t) -> Self {
        if object.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(object)) })
        }
    }
}

impl From<Option<Box<Object>>> for qiniu_ng_object_t {
    fn from(object: Option<Box<Object>>) -> Self {
        object.map(|object| object.into()).unwrap_or_default()
    }
}

impl From<Box<Object>> for qiniu_ng_object_t {
    fn from(object: Box<Object>) -> Self {
        unsafe { transmute(Box::into_raw(object)) }
    }
}

/// @brief 创建对象实例
/// @param[in] bucket 存储空间实例
/// @param[in] key 对象名称
/// @retval qiniu_ng_object_t 获取创建的对象实例
/// @note 注意，该方法仅用于在 SDK 中创建对象实例，而非在七牛云服务器上创建新的存储空间
/// @warning 务必在使用 `qiniu_ng_object_t` 完毕后调用 `qiniu_ng_object_free()` 方法释放 `qiniu_ng_object_t`
/// @warning 在调用完毕后 `qiniu_ng_bucket_t` 依然需要被 `qiniu_ng_bucket_free()` 释放
#[no_mangle]
pub extern "C" fn qiniu_ng_object_new(bucket: qiniu_ng_bucket_t, key: *const qiniu_ng_char_t) -> qiniu_ng_object_t {
    let bucket = Option::<Bucket>::from(bucket).unwrap();
    let key = unsafe { ucstr::from_ptr(key) }.to_string().unwrap();
    qiniu_ng_object_t::from(Box::new(bucket.object(key))).tap(|_| {
        let _ = qiniu_ng_bucket_t::from(bucket);
    })
}

/// @brief 释放对象实例
/// @param[in,out] object 对象实例地址，释放完毕后该对象实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_object_free(object: *mut qiniu_ng_object_t) {
    if let Some(object) = unsafe { object.as_mut() } {
        let _ = Option::<Box<Object>>::from(*object);
        *object = qiniu_ng_object_t::default();
    }
}

/// @brief 判断对象实例是否已经被释放
/// @param[in] object 对象实例
/// @retval bool 如果返回 `true` 则表示对象实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_object_is_freed(object: qiniu_ng_object_t) -> bool {
    object.is_null()
}

/// @brief 获取对象所在存储空间实例
/// @param[in] object 对象实例
/// @retval qiniu_ng_bucket_t 返回对象所在存储空间实例
/// @warning 务必记得 `qiniu_ng_bucket_t` 需要在使用完毕后调用 `qiniu_ng_bucket_free()` 释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_object_get_bucket(object: qiniu_ng_object_t) -> qiniu_ng_bucket_t {
    let object = Option::<Box<Object>>::from(object).unwrap();
    object
        .bucket()
        .to_owned()
        .tap(|_| {
            let _ = qiniu_ng_object_t::from(object);
        })
        .into()
}

/// @brief 获取对象名称
/// @param[in] object 对象实例
/// @retval qiniu_ng_str_t 返回对象的名称
/// @warning 务必记得 `qiniu_ng_str_t` 需要在使用完毕后调用 `qiniu_ng_str_free()` 释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_object_get_key(object: qiniu_ng_object_t) -> qiniu_ng_str_t {
    let object = Option::<Box<Object>>::from(object).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(object.key()) }.tap(|_| {
        let _ = qiniu_ng_object_t::from(object);
    })
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_object_info_t(*mut c_void);

impl Default for qiniu_ng_object_info_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_object_info_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_object_info_t> for Option<Box<ObjectInfo>> {
    fn from(object: qiniu_ng_object_info_t) -> Self {
        if object.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(object)) })
        }
    }
}

impl From<Option<Box<ObjectInfo>>> for qiniu_ng_object_info_t {
    fn from(object: Option<Box<ObjectInfo>>) -> Self {
        object.map(|object| object.into()).unwrap_or_default()
    }
}

impl From<Box<ObjectInfo>> for qiniu_ng_object_info_t {
    fn from(object: Box<ObjectInfo>) -> Self {
        unsafe { transmute(Box::into_raw(object)) }
    }
}

/// @brief 获取对象详细信息
/// @param[in] object 对象实例
/// @param[out] object_info 用于返回对象详细信息的内存地址，如果传入 `NULL` 表示不获取 `object_info`。但如果运行正常，返回值将依然是 `true`
/// @param[out] err 用于返回错误，如果传入 `NULL` 表示不获取 `err`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `object_info` 获得结果，如果返回 `false`，则表示可以读取 `err` 获得错误信息
/// @warning 对于获取的 `object_info` 或 `err`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_object_get_info(
    object: qiniu_ng_object_t,
    object_info: *mut qiniu_ng_object_info_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let object = Option::<Box<Object>>::from(object).unwrap();
    let mut result = true;
    match object.get_info() {
        Ok(info) => {
            if let Some(object_info) = unsafe { object_info.as_mut() } {
                *object_info = Box::new(info).into();
            }
        }
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            result = false
        }
    };
    let _ = qiniu_ng_object_t::from(object);
    result
}

/// @brief 获取对象信息中的对象尺寸
/// @param[in] object_info 对象详细信息
/// @retval uint64_t 对象尺寸，单位为字节
#[no_mangle]
pub extern "C" fn qiniu_ng_object_info_get_size(object_info: qiniu_ng_object_info_t) -> u64 {
    let object_info = Option::<Box<ObjectInfo>>::from(object_info).unwrap();
    object_info.size().tap(|_| {
        let _ = qiniu_ng_object_info_t::from(object_info);
    })
}

/// @brief 获取对象信息中的校验和字段
/// @param[in] object_info 对象详细信息
/// @param[out] result_ptr 提供内存地址用于返回校验和字段，如果传入 `NULL` 表示不获取 `result_ptr`。且不影响其他字段的获取
/// @param[out] result_size 用于返回校验和字段长度，如果传入 `NULL` 表示不获取 `result_size`。且不影响其他字段的获取。该字段一般返回的是 Etag，因此长度一般会等于 `ETAG_SIZE`
#[no_mangle]
pub extern "C" fn qiniu_ng_object_info_get_hash(
    object_info: qiniu_ng_object_info_t,
    hash_ptr: *mut c_void,
    hash_size: *mut size_t,
) {
    let object_info = Option::<Box<ObjectInfo>>::from(object_info).unwrap();
    let object_hash = object_info.hash();
    if let Some(hash_size) = unsafe { hash_size.as_mut() } {
        *hash_size = object_hash.len();
    }
    if let Some(hash_ptr) = unsafe { hash_ptr.as_mut() } {
        unsafe {
            copy_nonoverlapping(
                object_hash.as_ptr(),
                hash_ptr as *mut c_void as *mut u8,
                object_hash.len(),
            )
        };
    }
    let _ = qiniu_ng_object_info_t::from(object_info);
}

/// @brief 获取对象信息中的 MIME 类型
/// @param[in] object_info 对象详细信息
/// @retval qiniu_ng_str_t MIME 类型
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_object_info_get_mime_type(object_info: qiniu_ng_object_info_t) -> qiniu_ng_str_t {
    let object_info = Option::<Box<ObjectInfo>>::from(object_info).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(object_info.mime_type()) }.tap(|_| {
        let _ = qiniu_ng_object_info_t::from(object_info);
    })
}

/// @brief 获取对象信息中的创建时间
/// @param[in] object_info 对象详细信息
/// @retval uint64_t 过期时间，使用以秒为单位的 UNIX 时间戳表示
#[no_mangle]
pub extern "C" fn qiniu_ng_object_info_get_put_time(object_info: qiniu_ng_object_info_t) -> u64 {
    let object_info = Option::<Box<ObjectInfo>>::from(object_info).unwrap();
    object_info
        .put_time()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .tap(|_| {
            let _ = qiniu_ng_object_info_t::from(object_info);
        })
}

/// @brief 释放对象详细信息实例
/// @param[in,out] object 对象详细信息实例地址，释放完毕后该对象详细信息实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_object_info_free(object_info: *mut qiniu_ng_object_info_t) {
    if let Some(object_info) = unsafe { object_info.as_mut() } {
        let _ = Option::<Box<ObjectInfo>>::from(*object_info);
        *object_info = qiniu_ng_object_info_t::default();
    }
}

/// @brief 判断对象详细信息实例是否已经被释放
/// @param[in] object 对象详细信息实例
/// @retval bool 如果返回 `true` 则表示对象详细信息实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_object_info_is_freed(object_info: qiniu_ng_object_info_t) -> bool {
    object_info.is_null()
}

/// @brief 删除对象
/// @param[in] object 对象实例
/// @param[out] err 用于返回错误，如果传入 `NULL` 表示不获取 `err`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `false`，则表示可以读取 `err` 获得错误信息
/// @warning 对于运行错误的情况，需要调用 `qiniu_ng_err_t` 系列的函数判定具体错误并释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_object_delete(object: qiniu_ng_object_t, err: *mut qiniu_ng_err_t) -> bool {
    let object = Option::<Box<Object>>::from(object).unwrap();
    let mut result = true;
    if let Err(ref e) = object.delete() {
        if let Some(err) = unsafe { err.as_mut() } {
            *err = e.into();
        }
        result = false;
    }
    let _ = qiniu_ng_object_t::from(object);
    result
}
