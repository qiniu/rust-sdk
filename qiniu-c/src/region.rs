use crate::{
    config::qiniu_ng_config_t,
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr},
    utils::qiniu_ng_str_list_t,
};
use libc::{c_char, c_void, size_t};
use qiniu_ng::storage::region::{Region, RegionBuilder, RegionId};
use std::{borrow::Cow, ffi::CStr, mem::transmute, ptr::null_mut};
use tap::TapOps;

/// 存储区域 ID
///
/// 枚举类，仅包含七牛公有云的所有存储区域 ID。
/// 对于私有云，则应采用其他方案替代
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_region_id_t {
    /// 华东区区域 ID
    qiniu_ng_region_z0,
    /// 华北区区域 ID
    qiniu_ng_region_z1,
    /// 华南区区域 ID
    qiniu_ng_region_z2,
    /// 东南亚地区区域 ID
    qiniu_ng_region_as0,
    /// 北美地区区域 ID
    qiniu_ng_region_na0,
}

impl qiniu_ng_region_id_t {
    pub fn as_cstr(self) -> &'static CStr {
        match self {
            qiniu_ng_region_id_t::qiniu_ng_region_z0 => unsafe { CStr::from_bytes_with_nul_unchecked(b"z0\0") },
            qiniu_ng_region_id_t::qiniu_ng_region_z1 => unsafe { CStr::from_bytes_with_nul_unchecked(b"z1\0") },
            qiniu_ng_region_id_t::qiniu_ng_region_z2 => unsafe { CStr::from_bytes_with_nul_unchecked(b"z2\0") },
            qiniu_ng_region_id_t::qiniu_ng_region_as0 => unsafe { CStr::from_bytes_with_nul_unchecked(b"as0\0") },
            qiniu_ng_region_id_t::qiniu_ng_region_na0 => unsafe { CStr::from_bytes_with_nul_unchecked(b"na0\0") },
        }
    }
}

impl AsRef<str> for qiniu_ng_region_id_t {
    fn as_ref(&self) -> &str {
        match self {
            qiniu_ng_region_id_t::qiniu_ng_region_z0 => "z0",
            qiniu_ng_region_id_t::qiniu_ng_region_z1 => "z1",
            qiniu_ng_region_id_t::qiniu_ng_region_z2 => "z2",
            qiniu_ng_region_id_t::qiniu_ng_region_as0 => "as0",
            qiniu_ng_region_id_t::qiniu_ng_region_na0 => "na0",
        }
    }
}

impl From<RegionId> for qiniu_ng_region_id_t {
    fn from(region_id: RegionId) -> Self {
        match region_id {
            RegionId::Z0 => qiniu_ng_region_id_t::qiniu_ng_region_z0,
            RegionId::Z1 => qiniu_ng_region_id_t::qiniu_ng_region_z1,
            RegionId::Z2 => qiniu_ng_region_id_t::qiniu_ng_region_z2,
            RegionId::AS0 => qiniu_ng_region_id_t::qiniu_ng_region_as0,
            RegionId::NA0 => qiniu_ng_region_id_t::qiniu_ng_region_na0,
        }
    }
}

impl From<qiniu_ng_region_id_t> for RegionId {
    fn from(region_id: qiniu_ng_region_id_t) -> Self {
        match region_id {
            qiniu_ng_region_id_t::qiniu_ng_region_z0 => RegionId::Z0,
            qiniu_ng_region_id_t::qiniu_ng_region_z1 => RegionId::Z1,
            qiniu_ng_region_id_t::qiniu_ng_region_z2 => RegionId::Z2,
            qiniu_ng_region_id_t::qiniu_ng_region_as0 => RegionId::AS0,
            qiniu_ng_region_id_t::qiniu_ng_region_na0 => RegionId::NA0,
        }
    }
}

impl From<qiniu_ng_region_id_t> for *const c_char {
    fn from(region_id: qiniu_ng_region_id_t) -> Self {
        region_id.as_cstr().as_ptr()
    }
}

/// @brief 获取区域 ID 的名称
/// @param[in] region_id 区域 ID
/// @retval *char 区域名称
/// @warning 对于返回的区域名称字符串，请勿释放其内存
#[no_mangle]
pub extern "C" fn qiniu_ng_region_id_name(region_id: qiniu_ng_region_id_t) -> *const c_char {
    region_id.into()
}

/// @brief 区域生成器
/// @details 用于配置并生成区域实例
/// @note
///   * 调用 `qiniu_ng_region_builder_new()` 函数创建 `qiniu_ng_region_builder_t` 实例。
///   * 当 `qiniu_ng_region_builder_t` 生成 `qiniu_ng_region_t` 完毕后
///     - 当需要继续生成其他存储空间实例时，可以调用 `qiniu_ng_region_builder_reset()` 方法重置生成器。
///     - 当没有其他生成需求时，请务必调用 `qiniu_ng_region_builder_free()` 方法释放内存。
/// @note
///   该结构体不可以跨线程使用
/// @note
///   注意，该结构体仅用于在 SDK 中配置生成区域实例，而非在七牛云服务器上创建新的区域。
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_region_builder_t(*mut c_void);

impl Default for qiniu_ng_region_builder_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_region_builder_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_region_builder_t> for Option<Box<RegionBuilder>> {
    fn from(builder: qiniu_ng_region_builder_t) -> Self {
        if builder.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(builder)) })
        }
    }
}

impl From<Option<Box<RegionBuilder>>> for qiniu_ng_region_builder_t {
    fn from(builder: Option<Box<RegionBuilder>>) -> Self {
        builder.map(|builder| builder.into()).unwrap_or_default()
    }
}

impl From<Box<RegionBuilder>> for qiniu_ng_region_builder_t {
    fn from(builder: Box<RegionBuilder>) -> Self {
        unsafe { transmute(Box::into_raw(builder)) }
    }
}

/// @brief 创建区域生成器实例
/// @retval qiniu_ng_bucket_t 获取创建的存储空间实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_region_builder_free()` 方法释放 `qiniu_ng_region_builder_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_new() -> qiniu_ng_region_builder_t {
    qiniu_ng_region_builder_t::from(Box::new(RegionBuilder::default()))
}

/// @brief 指定区域 ID
/// @param[in] builder 区域生成器
/// @param[in] region_id 区域 ID
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_set_region_id(
    builder: qiniu_ng_region_builder_t,
    region_id: qiniu_ng_region_id_t,
) {
    let mut builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    builder.region_id(Some(region_id.into()));
    let _ = qiniu_ng_region_builder_t::from(builder);
}

/// @brief 追加上传服务器（HTTP 协议）URL
/// @param[in] builder 区域生成器
/// @param[in] up_http_url 上传服务器（HTTP 协议）URL
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_append_up_http_url(
    builder: qiniu_ng_region_builder_t,
    up_http_url: *const c_char,
) {
    let mut builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    builder.append_up_http_url(unsafe { ucstr::from_ptr(up_http_url) }.to_string().unwrap());
    let _ = qiniu_ng_region_builder_t::from(builder);
}

/// @brief 追加上传服务器（HTTPS 协议）URL
/// @param[in] builder 区域生成器
/// @param[in] up_https_url 上传服务器（HTTPS 协议）URL
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_append_up_https_url(
    builder: qiniu_ng_region_builder_t,
    up_https_url: *const c_char,
) {
    let mut builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    builder.append_up_https_url(unsafe { ucstr::from_ptr(up_https_url) }.to_string().unwrap());
    let _ = qiniu_ng_region_builder_t::from(builder);
}

/// @brief 追加 IO 服务器（HTTP 协议）URL
/// @param[in] builder 区域生成器
/// @param[in] io_http_url IO 服务器（HTTP 协议）URL
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_append_io_http_url(
    builder: qiniu_ng_region_builder_t,
    io_http_url: *const c_char,
) {
    let mut builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    builder.append_io_http_url(unsafe { ucstr::from_ptr(io_http_url) }.to_string().unwrap());
    let _ = qiniu_ng_region_builder_t::from(builder);
}

/// @brief 追加 IO 服务器（HTTPS 协议）URL
/// @param[in] builder 区域生成器
/// @param[in] io_https_url IO 服务器（HTTPS 协议）URL
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_append_io_https_url(
    builder: qiniu_ng_region_builder_t,
    io_https_url: *const c_char,
) {
    let mut builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    builder.append_io_https_url(unsafe { ucstr::from_ptr(io_https_url) }.to_string().unwrap());
    let _ = qiniu_ng_region_builder_t::from(builder);
}

/// @brief 追加 RS 服务器（HTTP 协议）URL
/// @param[in] builder 区域生成器
/// @param[in] rs_http_url RS 服务器（HTTP 协议）URL
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_append_rs_http_url(
    builder: qiniu_ng_region_builder_t,
    rs_http_url: *const c_char,
) {
    let mut builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    builder.append_rs_http_url(unsafe { ucstr::from_ptr(rs_http_url) }.to_string().unwrap());
    let _ = qiniu_ng_region_builder_t::from(builder);
}

/// @brief 追加 RS 服务器（HTTPS 协议）URL
/// @param[in] builder 区域生成器
/// @param[in] rs_https_url RS 服务器（HTTPS 协议）URL
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_append_rs_https_url(
    builder: qiniu_ng_region_builder_t,
    rs_https_url: *const c_char,
) {
    let mut builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    builder.append_rs_https_url(unsafe { ucstr::from_ptr(rs_https_url) }.to_string().unwrap());
    let _ = qiniu_ng_region_builder_t::from(builder);
}

/// @brief 追加 RSF 服务器（HTTP 协议）URL
/// @param[in] builder 区域生成器
/// @param[in] rsf_http_url RSF 服务器（HTTP 协议）URL
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_append_rsf_http_url(
    builder: qiniu_ng_region_builder_t,
    rsf_http_url: *const c_char,
) {
    let mut builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    builder.append_rsf_http_url(unsafe { ucstr::from_ptr(rsf_http_url) }.to_string().unwrap());
    let _ = qiniu_ng_region_builder_t::from(builder);
}

/// @brief 追加 RSF 服务器（HTTPS 协议）URL
/// @param[in] builder 区域生成器
/// @param[in] rsf_https_url RSF 服务器（HTTPS 协议）URL
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_append_rsf_https_url(
    builder: qiniu_ng_region_builder_t,
    rsf_https_url: *const c_char,
) {
    let mut builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    builder.append_rsf_https_url(unsafe { ucstr::from_ptr(rsf_https_url) }.to_string().unwrap());
    let _ = qiniu_ng_region_builder_t::from(builder);
}

/// @brief 追加 API 服务器（HTTP 协议）URL
/// @param[in] builder 区域生成器
/// @param[in] api_http_url API 服务器（HTTP 协议）URL
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_append_api_http_url(
    builder: qiniu_ng_region_builder_t,
    api_http_url: *const c_char,
) {
    let mut builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    builder.append_api_http_url(unsafe { ucstr::from_ptr(api_http_url) }.to_string().unwrap());
    let _ = qiniu_ng_region_builder_t::from(builder);
}

/// @brief 追加 API 服务器（HTTPS 协议）URL
/// @param[in] builder 区域生成器
/// @param[in] api_https_url API 服务器（HTTPS 协议）URL
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_append_api_https_url(
    builder: qiniu_ng_region_builder_t,
    api_https_url: *const c_char,
) {
    let mut builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    builder.append_api_https_url(unsafe { ucstr::from_ptr(api_https_url) }.to_string().unwrap());
    let _ = qiniu_ng_region_builder_t::from(builder);
}

/// @brief 生成区域实例
/// @param[in] builder 区域生成器
/// @retval qiniu_ng_region_t 获取生成的区域实例
/// @warning 务必在使用 `qiniu_ng_region_t` 完毕后调用 `qiniu_ng_region_free()` 方法释放 `qiniu_ng_region_t`
/// @warning 在调用完毕后 `qiniu_ng_region_builder_t` 依然需要被 `qiniu_ng_region_builder_free()` 释放
#[no_mangle]
pub extern "C" fn qiniu_ng_region_build(builder: qiniu_ng_region_builder_t) -> qiniu_ng_region_t {
    let builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    qiniu_ng_region_t::from(Box::new(Cow::Owned(builder.build()))).tap(|_| {
        let _ = qiniu_ng_region_builder_t::from(builder);
    })
}

/// @brief 重置区域生成器实例
/// @details 调用该方法使生成器可以被多次复用
/// @param[in] builder 区域生成器
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_reset(builder: qiniu_ng_region_builder_t) {
    let mut builder = Option::<Box<RegionBuilder>>::from(builder).unwrap();
    builder.reset();
    let _ = qiniu_ng_region_builder_t::from(builder);
}

/// @brief 释放区域生成器实例
/// @param[in,out] builder 区域生成器实例地址，释放完毕后该生成器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_free(builder: *mut qiniu_ng_region_builder_t) {
    if let Some(builder) = unsafe { builder.as_mut() } {
        let _ = Option::<Box<RegionBuilder>>::from(*builder);
        *builder = qiniu_ng_region_builder_t::default();
    }
}

/// @brief 判断区域生成器实例是否已经被释放
/// @param[in] builder 区域生成器实例
/// @retval bool 如果返回 `true` 则表示区域生成器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_region_builder_is_freed(builder: qiniu_ng_region_builder_t) -> bool {
    builder.is_null()
}

/// @brief 区域
/// @details 区域实例负责管理七牛多个服务器的 URL，用于为存储管理器或上传管理器提供 URL。
/// @note
///   * 调用 `qiniu_ng_region_builder_t` 的方法生成 `qiniu_ng_region_t` 实例。
///   * 当 `qiniu_ng_region_t` 使用完毕后，请务必调用 `qiniu_ng_region_free()` 方法释放内存。
/// @note
///   该结构体内部状态不可变，因此可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_region_t(*mut c_void);

impl Default for qiniu_ng_region_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_region_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

// TODO: 设计一个新的枚举类替代 `Region`，可以在引用的情况下不使用堆内存

impl From<qiniu_ng_region_t> for Option<Box<Cow<'static, Region>>> {
    fn from(region: qiniu_ng_region_t) -> Self {
        if region.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(region)) })
        }
    }
}

impl From<Option<Box<Cow<'static, Region>>>> for qiniu_ng_region_t {
    fn from(region: Option<Box<Cow<'static, Region>>>) -> Self {
        region.map(|region| region.into()).unwrap_or_default()
    }
}

impl From<Box<Cow<'static, Region>>> for qiniu_ng_region_t {
    fn from(region: Box<Cow<'static, Region>>) -> Self {
        unsafe { transmute(Box::into_raw(region)) }
    }
}

/// @brief 获取区域 ID
/// @param[in] region 区域实例
/// @param[out] region_id 用于返回区域 ID 的内存地址，如果传入 `NULL` 表示不获取 `region_id`。但如果区域 ID 存在，返回值将依然是 `true`
/// @retval bool 是否存在区域 ID，如果返回 `true`，则表示可以读取 `region_id` 获得结果
/// @note 通过七牛服务器查询获得的区域实例，区域 ID 通常不存在
#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_region_id(
    region: qiniu_ng_region_t,
    region_id: *mut qiniu_ng_region_id_t,
) -> bool {
    let region = Option::<Box<Cow<'static, Region>>>::from(region).unwrap();
    match region.region_id().tap(|_| {
        let _ = qiniu_ng_region_t::from(region);
    }) {
        Some(rid) => {
            if let Some(region_id) = unsafe { region_id.as_mut() } {
                *region_id = rid.into();
            }
            true
        }
        None => false,
    }
}

/// @brief 通过区域 ID 获取区域实例
/// @param[in] region_id 区域 ID
/// @retval qiniu_ng_region_t 区域实例
/// @warning 当 `qiniu_ng_region_t` 使用完毕后，请务必调用 `qiniu_ng_region_free()` 方法释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_by_id(region_id: qiniu_ng_region_id_t) -> qiniu_ng_region_t {
    match region_id {
        qiniu_ng_region_id_t::qiniu_ng_region_z0 => Box::new(Cow::Borrowed(Region::z0())).into(),
        qiniu_ng_region_id_t::qiniu_ng_region_z1 => Box::new(Cow::Borrowed(Region::z1())).into(),
        qiniu_ng_region_id_t::qiniu_ng_region_z2 => Box::new(Cow::Borrowed(Region::z2())).into(),
        qiniu_ng_region_id_t::qiniu_ng_region_as0 => Box::new(Cow::Borrowed(Region::as0())).into(),
        qiniu_ng_region_id_t::qiniu_ng_region_na0 => Box::new(Cow::Borrowed(Region::na0())).into(),
    }
}

/// @brief 获取区域上传服务器 URL 列表
/// @param[in] region 区域实例
/// @param[in] use_https 是否返回 HTTPS 协议的 URL
/// @retval qiniu_ng_str_list_t 返回上传服务器 URL 列表
/// @warning 当 `qiniu_ng_str_list_t` 使用完毕后，请务必调用 `qiniu_ng_str_list_free()` 方法释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_up_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_str_list_t {
    let region = Option::<Box<Cow<'static, Region>>>::from(region).unwrap();
    unsafe { qiniu_ng_str_list_t::from_str_slice_unchecked(&region.up_urls_ref(use_https)) }.tap(|_| {
        let _ = qiniu_ng_region_t::from(region);
    })
}

/// @brief 获取区域 IO 服务器 URL 列表
/// @param[in] region 区域实例
/// @param[in] use_https 是否返回 HTTPS 协议的 URL
/// @retval qiniu_ng_str_list_t 返回 IO 服务器 URL 列表
/// @warning 当 `qiniu_ng_str_list_t` 使用完毕后，请务必调用 `qiniu_ng_str_list_free()` 方法释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_io_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_str_list_t {
    let region = Option::<Box<Cow<'static, Region>>>::from(region).unwrap();
    unsafe { qiniu_ng_str_list_t::from_str_slice_unchecked(&region.io_urls_ref(use_https)) }.tap(|_| {
        let _ = qiniu_ng_region_t::from(region);
    })
}

/// @brief 获取区域 RS 服务器 URL 列表
/// @param[in] region 区域实例
/// @param[in] use_https 是否返回 HTTPS 协议的 URL
/// @retval qiniu_ng_str_list_t 返回 RS 服务器 URL 列表
/// @warning 当 `qiniu_ng_str_list_t` 使用完毕后，请务必调用 `qiniu_ng_str_list_free()` 方法释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_rs_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_str_list_t {
    let region = Option::<Box<Cow<'static, Region>>>::from(region).unwrap();
    unsafe { qiniu_ng_str_list_t::from_str_slice_unchecked(&region.rs_urls_ref(use_https)) }.tap(|_| {
        let _ = qiniu_ng_region_t::from(region);
    })
}

/// @brief 获取区域 RSF 服务器 URL 列表
/// @param[in] region 区域实例
/// @param[in] use_https 是否返回 HTTPS 协议的 URL
/// @retval qiniu_ng_str_list_t 返回 RSF 服务器 URL 列表
/// @warning 当 `qiniu_ng_str_list_t` 使用完毕后，请务必调用 `qiniu_ng_str_list_free()` 方法释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_rsf_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_str_list_t {
    let region = Option::<Box<Cow<'static, Region>>>::from(region).unwrap();
    unsafe { qiniu_ng_str_list_t::from_str_slice_unchecked(&region.rsf_urls_ref(use_https)) }.tap(|_| {
        let _ = qiniu_ng_region_t::from(region);
    })
}

/// @brief 获取区域 API 服务器 URL 列表
/// @param[in] region 区域实例
/// @param[in] use_https 是否返回 HTTPS 协议的 URL
/// @retval qiniu_ng_str_list_t 返回 API 服务器 URL 列表
/// @warning 当 `qiniu_ng_str_list_t` 使用完毕后，请务必调用 `qiniu_ng_str_list_free()` 方法释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_region_get_api_urls(region: qiniu_ng_region_t, use_https: bool) -> qiniu_ng_str_list_t {
    let region = Option::<Box<Cow<'static, Region>>>::from(region).unwrap();
    unsafe { qiniu_ng_str_list_t::from_str_slice_unchecked(&region.api_urls_ref(use_https)) }.tap(|_| {
        let _ = qiniu_ng_region_t::from(region);
    })
}

/// @brief 查询七牛服务器，根据存储空间名称获取区域列表
/// @param[in] bucket_name 存储空间名称
/// @param[in] access_key 七牛 Access Key
/// @param[in] config 客户端配置
/// @param[out] regions 返回区域列表，区域列表中第一个区域是当前存储空间所在区域，之后的区域则是备用区域。如果传入 `NULL` 表示不获取 `regions`。但如果运行正常，返回值将依然是 `true`
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `regions` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `regions` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_region_query(
    bucket_name: *const qiniu_ng_char_t,
    access_key: *const qiniu_ng_char_t,
    config: qiniu_ng_config_t,
    regions: *mut qiniu_ng_regions_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    match Region::query(
        unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap(),
        unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
        config.get_clone().unwrap(),
    ) {
        Ok(r) => {
            if let Some(regions) = unsafe { regions.as_mut() } {
                *regions = r.into();
            }
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

/// @brief 释放区域实例
/// @param[in,out] region 区域实例地址，释放完毕后该区域实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_region_free(region: *mut qiniu_ng_region_t) {
    if let Some(region) = unsafe { region.as_mut() } {
        let _ = Option::<Box<Cow<'static, Region>>>::from(*region);
        *region = qiniu_ng_region_t::default();
    }
}

/// @brief 判断区域实例是否已经被释放
/// @param[in] region 区域实例
/// @retval bool 如果返回 `true` 则表示区域实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_region_is_freed(region: qiniu_ng_region_t) -> bool {
    region.is_null()
}

/// @brief 区域列表
/// @details 区域列表管理多个区域
/// @note
///   * 调用 `qiniu_ng_regions_len()` 的方法获取列表中 `qiniu_ng_region_t` 实例的数量。
///   * 逐一调用 `qiniu_ng_regions_get()` 的方法获取列表中 `qiniu_ng_region_t` 实例。
///   * 一旦一个 `qiniu_ng_region_t` 实例使用完毕后，请务必调用 `qiniu_ng_region_free()` 方法释放内存。
///   * 当 `qiniu_ng_regions_t` 使用完毕后，请务必调用 `qiniu_ng_regions_free()` 方法释放内存。
/// @note
///   该结构体内部状态不可变，因此可以跨线程使用
/// @warning 请勿在列表中的 `qiniu_ng_region_t` 没有释放区域实例的内存前，就先调用 `qiniu_ng_regions_free()` 方法释放列表的内存。
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_regions_t(*mut c_void, *mut c_void);

impl Default for qiniu_ng_regions_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut(), null_mut())
    }
}

impl qiniu_ng_regions_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null() && self.1.is_null()
    }
}

impl From<qiniu_ng_regions_t> for Option<Box<[Region]>> {
    fn from(regions: qiniu_ng_regions_t) -> Self {
        if regions.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(regions)) })
        }
    }
}

impl From<Option<Box<[Region]>>> for qiniu_ng_regions_t {
    fn from(regions: Option<Box<[Region]>>) -> Self {
        regions.map(|regions| regions.into()).unwrap_or_default()
    }
}

impl From<Box<[Region]>> for qiniu_ng_regions_t {
    fn from(regions: Box<[Region]>) -> Self {
        unsafe { transmute(Box::into_raw(regions)) }
    }
}

/// @brief 获取区域列表中区域的数量
/// @param[in] regions 区域列表
/// @retval size_t 返回区域列表中区域的数量
#[no_mangle]
pub extern "C" fn qiniu_ng_regions_len(regions: qiniu_ng_regions_t) -> size_t {
    let regions = Option::<Box<[Region]>>::from(regions).unwrap();
    regions.len().tap(|_| {
        let _ = qiniu_ng_regions_t::from(regions);
    })
}

/// @brief 获取区域列表中的区域
/// @param[in] regions 区域列表
/// @param[in] index 区域列表索引
/// @param[in] region 返回区域列表中的区域，如果传入 `NULL` 表示不获取 `result`。但如果该区域存在，返回值将依然是 `true`
/// @retval bool 如果区域列表中该索引对应的区域存在，则返回 `true`，否则返回 `false`
/// @warning
///     * 务必记得如果获取了 `qiniu_ng_region_t`，需要在使用完毕后调用 `qiniu_ng_region_free()` 释放内存。
///     * 在所有区域列表中的区域使用完毕前，请不要调用 `qiniu_ng_regions_free()` 释放区域列表的内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_regions_get(
    regions: qiniu_ng_regions_t,
    index: size_t,
    region: *mut qiniu_ng_region_t,
) -> bool {
    let regions = Option::<Box<[Region]>>::from(regions).unwrap();
    let mut got = false;
    if let Some(r) = regions.get(index) {
        if let Some(region) = unsafe { region.as_mut() } {
            *region = Box::<Cow<Region>>::new(Cow::Owned(r.to_owned())).into();
        }
        got = true;
    }
    let _ = qiniu_ng_regions_t::from(regions);
    got
}

/// @brief 判断区域列表是否已经被释放
/// @param[in] regions 区域列表
/// @retval bool 如果返回 `true` 则表示区域列表已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_regions_free(regions: *mut qiniu_ng_regions_t) {
    if let Some(regions) = unsafe { regions.as_mut() } {
        let _ = Option::<Box<[Region]>>::from(*regions);
        *regions = qiniu_ng_regions_t::default();
    }
}

/// @brief 判断区域列表是否已经被释放
/// @param[in] regions 区域列表
/// @retval bool 如果返回 `true` 则表示区域列表已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_regions_is_freed(regions: qiniu_ng_regions_t) -> bool {
    regions.is_null()
}
