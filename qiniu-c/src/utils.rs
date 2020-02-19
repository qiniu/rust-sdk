use crate::string::{qiniu_ng_char_t, ucstr, UCString};
use libc::{c_void, size_t};
use std::{
    boxed::Box,
    collections::{hash_map::RandomState, HashMap},
    io::{Error, ErrorKind, Read, Result},
    mem::transmute,
    ptr::{null, null_mut},
};
use tap::TapOps;

/// @brief 字符串
/// @details 封装一个 C 字符串，字符类型为 `qiniu_ng_char_t`
/// @note
///   * 当 SDK 的 API 返回字符串实例后，调用 `qiniu_ng_str_get_ptr()` 获取内部的 C 字符串。
///   * 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存。
/// @note 该结构体内部状态不可变，因此可以跨线程使用
/// @note 目前在 UNIX 平台上编码为 UTF-8 而 Windows 编码为 UTF-16，但不同编译条件下字符串的编码可能有所不同
/// @warning `qiniu_ng_str_t` 中的字符串有可能是 `NULL`，这种情况下，可以通过 `qiniu_ng_str_is_null()` 判定
/// @warning `qiniu_ng_str_get_ptr()` 将会返回存储的 C 字符串内存地址，请勿修改其存储的字符串内容
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_str_t(*mut c_void, *mut c_void);

impl qiniu_ng_str_t {
    pub(crate) unsafe fn from_str_unchecked(s: &str) -> Self {
        UCString::from_str_unchecked(s).into()
    }

    pub(crate) unsafe fn from_optional_str_unchecked(s: Option<&str>) -> Self {
        s.map(|s| UCString::from_str_unchecked(s).into()).unwrap_or_default()
    }

    pub(crate) unsafe fn from_string_unchecked(s: String) -> Self {
        UCString::from_string_unchecked(s).into_boxed_ucstr().into()
    }

    pub(crate) unsafe fn from_optional_string_unchecked(s: Option<String>) -> Self {
        s.map(|s| UCString::from_string_unchecked(s).into_boxed_ucstr().into())
            .unwrap_or_default()
    }

    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null() && self.1.is_null()
    }
}

impl Default for qiniu_ng_str_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut(), null_mut())
    }
}

impl From<Option<Box<ucstr>>> for qiniu_ng_str_t {
    fn from(s: Option<Box<ucstr>>) -> Self {
        s.map(|s| unsafe { transmute(Box::into_raw(s)) }).unwrap_or_default()
    }
}

impl From<Option<UCString>> for qiniu_ng_str_t {
    fn from(s: Option<UCString>) -> Self {
        s.map(|s| unsafe { transmute(Box::into_raw(s.into_boxed_ucstr())) })
            .unwrap_or_default()
    }
}

impl From<Box<ucstr>> for qiniu_ng_str_t {
    #[inline]
    fn from(s: Box<ucstr>) -> Self {
        Some(s).into()
    }
}

impl From<UCString> for qiniu_ng_str_t {
    #[inline]
    fn from(s: UCString) -> Self {
        Some(s).into()
    }
}

impl From<qiniu_ng_str_t> for Option<Box<ucstr>> {
    fn from(s: qiniu_ng_str_t) -> Self {
        if s.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(s)) })
        }
    }
}

impl From<qiniu_ng_str_t> for Option<UCString> {
    fn from(s: qiniu_ng_str_t) -> Self {
        Option::<Box<ucstr>>::from(s).map(|s| s.into())
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

/// @brief 获取被封装的 C 字符串指针
/// @param[in] s 字符串实例
/// @retval *qiniu_ng_char_t 返回 C 字符串指针
/// @note 如果封装的 C 字符串地址为 `NULL`，该函数也将返回 `NULL`
/// @warning 该方法将会返回存储的 C 字符串内存地址，请勿修改其存储的字符串内容
#[no_mangle]
pub extern "C" fn qiniu_ng_str_get_ptr(s: qiniu_ng_str_t) -> *const qiniu_ng_char_t {
    let s = Option::<Box<ucstr>>::from(s);
    s.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null).tap(|_| {
        let _ = qiniu_ng_str_t::from(s);
    })
}

/// @brief 获取被封装的 C 字符串长度
/// @param[in] s 字符串实例
/// @retval size_t 返回 C 字符串长度，单位为字节。因此对于 Unicode 编码字符而言，一个字符可能有多个字节组成。
/// @note 如果封装的 C 字符串地址为 `NULL`，该函数将返回 `0`
#[no_mangle]
pub extern "C" fn qiniu_ng_str_get_len(s: qiniu_ng_str_t) -> size_t {
    let s = Option::<Box<ucstr>>::from(s);
    s.as_ref().map(|s| s.len()).unwrap_or(0).tap(|_| {
        let _ = qiniu_ng_str_t::from(s);
    })
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
        *s = qiniu_ng_str_t::default();
    }
}

/// @brief 判断字符串实例是否已经被释放
/// @param[in] s 字符串实例
/// @retval bool 如果返回 `true` 则表示字符串实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_str_is_freed(s: qiniu_ng_str_t) -> bool {
    s.is_null()
}

/// @brief 字符串列表
/// @details 封装一个 C 字符串列表
/// @note
///   * 当 SDK 的 API 返回字符串列表实例后，调用 `qiniu_ng_str_list_len()` 获取字符串列表长度。
///   * 逐一调用 `qiniu_ng_str_list_get()` 获取字符串列表中每个字符串实例的地址。
///   * 当 `qiniu_ng_str_list_t` 使用完毕后，请务必调用 `qiniu_ng_str_list_free()` 方法释放内存。
/// @note 该结构体内部状态不可变，因此可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_str_list_t(*mut c_void, *mut c_void);

impl Default for qiniu_ng_str_list_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut(), null_mut())
    }
}

impl qiniu_ng_str_list_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null() && self.1.is_null()
    }
}

impl qiniu_ng_str_list_t {
    #[allow(dead_code)]
    pub(crate) unsafe fn from_str_slice_unchecked(list: &[&str]) -> Self {
        list.iter()
            .map(|s| UCString::from_str_unchecked(s).into_boxed_ucstr())
            .collect::<Box<[_]>>()
            .into()
    }

    #[allow(dead_code)]
    pub(crate) unsafe fn from_optional_str_slice_unchecked(list: Option<&[&str]>) -> Self {
        list.map(|list| Self::from_str_slice_unchecked(list))
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub(crate) unsafe fn from_string_vec_unchecked(list: Vec<String>) -> Self {
        list.into_iter()
            .map(|s| UCString::from_string_unchecked(s).into_boxed_ucstr())
            .collect::<Box<[_]>>()
            .into()
    }

    #[allow(dead_code)]
    pub(crate) unsafe fn from_optional_string_vec_unchecked(list: Option<Vec<String>>) -> Self {
        list.map(|list| Self::from_string_vec_unchecked(list))
            .unwrap_or_default()
    }
}

impl From<Box<[Box<ucstr>]>> for qiniu_ng_str_list_t {
    fn from(strlist: Box<[Box<ucstr>]>) -> Self {
        unsafe { transmute(Box::into_raw(strlist)) }
    }
}

impl From<Option<Box<[Box<ucstr>]>>> for qiniu_ng_str_list_t {
    fn from(strlist: Option<Box<[Box<ucstr>]>>) -> Self {
        strlist.map(|strlist| strlist.into()).unwrap_or_default()
    }
}

impl From<qiniu_ng_str_list_t> for Option<Box<[Box<ucstr>]>> {
    fn from(strlist: qiniu_ng_str_list_t) -> Self {
        if strlist.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(strlist)) })
        }
    }
}

/// @brief 创建新的字符串列表实例
/// @param[in] strlist 封装的 C 字符串列表地址
/// @param[in] len 封装的 C 字符串列表长度
/// @retval qiniu_ng_str_list_t 返回字符串列表实例
#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_new(strlist: *const *const qiniu_ng_char_t, len: size_t) -> qiniu_ng_str_list_t {
    (0..len)
        .map(|i| unsafe { UCString::from_ptr(*strlist.add(i)) }.into_boxed_ucstr())
        .collect::<Box<[_]>>()
        .into()
}

/// @brief 获取字符串列表长度
/// @param[in] strlist 字符串列表实例
/// @retval size_t 返回字符串列表长度
#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_len(strlist: qiniu_ng_str_list_t) -> size_t {
    let strlist = Option::<Box<[Box<ucstr>]>>::from(strlist);
    strlist.as_ref().map(|list| list.len()).unwrap_or(0).tap(|_| {
        let _ = qiniu_ng_str_list_t::from(strlist);
    })
}

/// @brief 获取字符串列表中的 C 字符串
/// @param[in] strlist 字符串列表实例
/// @param[in] index 字符串列表索引
/// @retval *qiniu_ng_char_t 返回对应的 C 字符串地址
/// @warning 该方法将会返回存储的 C 字符串内存地址，请勿修改其存储的字符串内容
#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_get(strlist: qiniu_ng_str_list_t, index: size_t) -> *const qiniu_ng_char_t {
    let strlist = Option::<Box<[Box<ucstr>]>>::from(strlist);
    strlist
        .as_ref()
        .and_then(|list| list.get(index))
        .map(|s| s.as_ptr())
        .unwrap_or_else(null)
        .tap(|_| {
            let _ = qiniu_ng_str_list_t::from(strlist);
        })
        .cast()
}

/// @brief 判定字符串列表中所有 C 字符串是否相同
/// @param[in] strlist1 字符串列表实例
/// @param[in] strlist2 字符串列表实例
/// @retval bool 如果两个字符串列表的长度均一致，其中每个相应位置字符串内容均相同，则返回 `true`
#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_eql(strlist1: qiniu_ng_str_list_t, strlist2: qiniu_ng_str_list_t) -> bool {
    let strlist1 = Option::<Box<[Box<ucstr>]>>::from(strlist1);
    let strlist2 = Option::<Box<[Box<ucstr>]>>::from(strlist2);
    let is_eql = strlist1 == strlist2;
    let _ = qiniu_ng_str_list_t::from(strlist1);
    let _ = qiniu_ng_str_list_t::from(strlist2);
    is_eql
}

/// @brief 复制字符串列表实例
/// @param[in] strlist 字符串列表实例
/// @retval qiniu_ng_str_list_t 返回复制的字符串列表实例。在使用完毕后，两个字符串列表实例必须分别调用 `qiniu_ng_str_list_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_clone(strlist: qiniu_ng_str_list_t) -> qiniu_ng_str_list_t {
    let strlist = Option::<Box<[Box<ucstr>]>>::from(strlist);
    qiniu_ng_str_list_t::from(strlist.clone()).tap(|_| {
        let _ = qiniu_ng_str_list_t::from(strlist);
    })
}

/// @brief 释放字符串列表实例
/// @param[in,out] strlist 字符串列表实例地址，释放完毕后该字符串列表实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_free(strlist: *mut qiniu_ng_str_list_t) {
    if let Some(strlist) = unsafe { strlist.as_mut() } {
        let _ = Option::<Box<[Box<ucstr>]>>::from(*strlist);
        *strlist = qiniu_ng_str_list_t::default();
    }
}

/// @brief 判断字符串列表实例是否已经被释放
/// @param[in] strlist 字符串列表实例
/// @retval bool 如果返回 `true` 则表示字符串列表实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_is_freed(strlist: qiniu_ng_str_list_t) -> bool {
    strlist.is_null()
}

/// @brief 字符串映射
/// @details 封装一个 C 字符串映射
/// @note
///   * 当 SDK 的 API 返回字符串映射实例后，调用 `qiniu_ng_str_map_each_entry()` 逐一获得字符串映射的键值对。
///   * 当 `qiniu_ng_str_map_t` 使用完毕后，请务必调用 `qiniu_ng_str_map_free()` 方法释放内存。
/// @note
///   * 调用 `qiniu_ng_str_map_new()` 创建一个字符串映射实例
///   * 多次调用 `qiniu_ng_str_map_set()` 向字符串映射实例输入键值对
///   * 当 `qiniu_ng_str_map_t` 使用完毕后，请务必调用 `qiniu_ng_str_map_free()` 方法释放内存。
/// @note 对该结构体不可跨线程修改数据，这一过程不保证线程安全性。
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_str_map_t(*mut c_void);

impl Default for qiniu_ng_str_map_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_str_map_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>> for qiniu_ng_str_map_t {
    fn from(hashmap: Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>) -> Self {
        unsafe { transmute(Box::into_raw(hashmap)) }
    }
}

impl From<Option<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>> for qiniu_ng_str_map_t {
    fn from(hashmap: Option<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>) -> Self {
        hashmap.map(|hashmap| hashmap.into()).unwrap_or_default()
    }
}

impl From<qiniu_ng_str_map_t> for Option<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>> {
    fn from(hashmap: qiniu_ng_str_map_t) -> Self {
        if hashmap.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(hashmap)) })
        }
    }
}

/// @brief 创建新的字符串映射实例
/// @param[in] capacity 预先分配的字符串映射容量
/// @retval qiniu_ng_str_map_t 返回字符串映射实例
#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_new(capacity: size_t) -> qiniu_ng_str_map_t {
    Box::<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>::new(HashMap::with_capacity(capacity)).into()
}

/// @brief 向字符串映射实例输入键值对
/// @param[in] hashmap 字符串映射实例
/// @param[in] key 输入的字符串键
/// @param[in] value 输入的字符串值
#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_set(
    hashmap: qiniu_ng_str_map_t,
    key: *const qiniu_ng_char_t,
    value: *const qiniu_ng_char_t,
) {
    let mut hashmap = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(hashmap).unwrap();
    hashmap.insert(
        unsafe { ucstr::from_ptr(key) }.to_owned().into_boxed_ucstr(),
        unsafe { ucstr::from_ptr(value) }.to_owned().into_boxed_ucstr(),
    );
    let _ = qiniu_ng_str_map_t::from(hashmap);
}

/// @brief 对字符串映射实例的每一个键值对分别调用回调函数
/// @param[in] hashmap 字符串映射实例
/// @param[in] handler 回调函数，对每一个键值对都会调用该函数，直到该函数返回 `false` 为止。回调函数的前两个参数分别是键值对的键和值字符串地址，而 `data` 则是调用该函数时给出的 `data` 参数，您可以根据您的需要将上下文参数传入。
/// @param[in] data 传入回调函数的数据，可以作为上下文参数使用
#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_each_entry(
    hashmap: qiniu_ng_str_map_t,
    handler: fn(key: *const qiniu_ng_char_t, value: *const qiniu_ng_char_t, data: *mut c_void) -> bool,
    data: *mut c_void,
) {
    let hashmap = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(hashmap);
    if let Some(hashmap) = hashmap.as_ref() {
        for (key, value) in hashmap.iter() {
            if !handler(key.as_ptr(), value.as_ptr(), data) {
                break;
            }
        }
    }
    let _ = qiniu_ng_str_map_t::from(hashmap);
}

/// @brief 根据键获取字符串映射实例的值
/// @param[in] hashmap 字符串映射实例
/// @param[in] key 查找字符串映射实例用的键
/// @retval *qiniu_ng_char_t 字符串映射实例中该键对应的值，如果对应的键找不到，将会返回 `NULL`
#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_get(
    hashmap: qiniu_ng_str_map_t,
    key: *const qiniu_ng_char_t,
) -> *const qiniu_ng_char_t {
    let hashmap = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(hashmap);
    hashmap
        .as_ref()
        .and_then(|hashmap| hashmap.get(unsafe { ucstr::from_ptr(key) }))
        .map(|val| val.as_ptr())
        .unwrap_or_else(null)
        .tap(|_| {
            let _ = qiniu_ng_str_map_t::from(hashmap);
        })
}

/// @brief 获取字符串映射实例中键值对的数量
/// @param[in] hashmap 字符串映射实例
/// @retval size_t 返回字符串映射实例中键值对的数量
#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_len(hashmap: qiniu_ng_str_map_t) -> size_t {
    let hashmap = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(hashmap);
    hashmap.as_ref().map(|hashmap| hashmap.len()).unwrap_or(0).tap(|_| {
        let _ = qiniu_ng_str_map_t::from(hashmap);
    })
}

/// @brief 复制字符串映射实例实例
/// @param[in] hashmap 字符串映射实例
/// @retval qiniu_ng_str_map_t 返回复制的字符串列表实例。在使用完毕后，两个字符串映射实例必须分别调用 `qiniu_ng_str_map_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_clone(hashmap: qiniu_ng_str_map_t) -> qiniu_ng_str_map_t {
    let hashmap = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(hashmap);
    qiniu_ng_str_map_t::from(hashmap.clone()).tap(|_| {
        let _ = qiniu_ng_str_map_t::from(hashmap);
    })
}

/// @brief 释放字符串映射实例
/// @param[in,out] hashmap 字符串映射实例地址，释放完毕后该字符串映射实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_free(hashmap: *mut qiniu_ng_str_map_t) {
    if let Some(hashmap) = unsafe { hashmap.as_mut() } {
        let _ = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(*hashmap);
        *hashmap = qiniu_ng_str_map_t::default();
    }
}

/// @brief 判断字符串列表实例是否已经被释放
/// @param[in] strlist 字符串列表实例
/// @retval bool 如果返回 `true` 则表示字符串列表实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_is_freed(hashmap: qiniu_ng_str_map_t) -> bool {
    hashmap.is_null()
}

/// @brief 数据读取器结构体，用于实现数据读取
/// @details
///   该结构是个简单的开放结构体，其中的 `read_func` 字段需要您用来定义回调函数实现对数据的读取。
///   该函数的第一个参数 `context` 是个上下文参数指针，可以自由定义其数据作为上下文使用。
///   第二个参数 `buf` 是 SDK 提供给回调函数读取数据的缓冲区地址，第三个参数 `count` 则是缓冲区的长度，单位为字节。
///   您需要读取数据并将数据写入 `buf` 缓冲区，且写入数据的尺寸不能超过 `count`。
///   写入完毕后，需要您将实际写入的数据长度填充在第四个参数 `have_read` 内。
///   如果发生无法处理的读取错误，则返回 `false`。
///
///   `context` 字段则是您用来传入到回调函数的上下文参数指针，您可以用来作为回调函数的上下文使用
#[repr(C)]
#[derive(Clone)]
pub struct qiniu_ng_readable_t {
    // TODO: read_func() 改成返回 `qiniu_ng_err_t`
    read_func: fn(context: *mut c_void, buf: *mut c_void, count: size_t, have_read: *mut size_t) -> bool,
    context: *mut c_void,
}

impl Read for qiniu_ng_readable_t {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut have_read: size_t = 0;
        if (self.read_func)(self.context, buf.as_mut_ptr().cast(), buf.len(), &mut have_read) {
            Ok(have_read)
        } else {
            Err(Error::new(ErrorKind::Other, "User callback returns false"))
        }
    }
}
unsafe impl Send for qiniu_ng_readable_t {}
