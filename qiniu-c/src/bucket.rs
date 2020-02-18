use crate::{
    client::qiniu_ng_client_t,
    region::{qiniu_ng_region_id_t, qiniu_ng_region_t, qiniu_ng_regions_t},
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr},
    utils::{qiniu_ng_str_list_t, qiniu_ng_str_t},
};
use libc::c_void;
use qiniu_ng::{
    storage::{
        bucket::{Bucket, BucketBuilder},
        region::Region,
    },
    Client,
};
use std::{borrow::Cow, mem::transmute, ptr::null_mut};
use tap::TapOps;

/// @brief 存储空间生成器
/// @details 用于配置并生成存储空间实例
/// @note
///   * 调用 `qiniu_ng_bucket_builder_new()` 函数创建 `qiniu_ng_bucket_builder_t` 实例。
///   * 当 `qiniu_ng_bucket_builder_t` 生成 `qiniu_ng_bucket_t` 完毕后
///     - 当需要继续生成其他存储空间实例时，可以调用 `qiniu_ng_bucket_builder_reset()` 方法重置生成器。
///     - 当没有其他生成需求时，请务必调用 `qiniu_ng_bucket_builder_free()` 方法释放内存。
/// @note
///   该结构体不可以跨线程使用
/// @note
///   注意，该结构体仅用于在 SDK 中配置生成存储空间实例，而非在七牛云服务器上创建新的存储空间。
///   事实上，除非您使用了私有云，或七牛以外的 CDN 服务商，否则您总是可以直接构建调用 `qiniu_ng_bucket_new()` 存储空间，存储空间为以懒加载的方式从七牛服务器获取区域信息和下载域名，SDK 确保懒加载的线程安全。
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_bucket_builder_t(*mut c_void);

impl Default for qiniu_ng_bucket_builder_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_bucket_builder_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl<'r> From<qiniu_ng_bucket_builder_t> for Option<Box<BucketBuilder<'r>>> {
    fn from(builder: qiniu_ng_bucket_builder_t) -> Self {
        if builder.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(builder)) })
        }
    }
}

impl<'r> From<Option<Box<BucketBuilder<'r>>>> for qiniu_ng_bucket_builder_t {
    fn from(builder: Option<Box<BucketBuilder>>) -> Self {
        builder.map(|builder| builder.into()).unwrap_or_default()
    }
}

impl<'r> From<Box<BucketBuilder<'r>>> for qiniu_ng_bucket_builder_t {
    fn from(builder: Box<BucketBuilder>) -> Self {
        unsafe { transmute(Box::into_raw(builder)) }
    }
}

/// @brief 创建存储空间生成器实例
/// @param[in] client 七牛 SDK 客户端实例
/// @param[in] bucket_name 存储空间名称
/// @retval qiniu_ng_bucket_t 获取创建的存储空间实例
/// @note 创建实例时，SDK 客户端会复制并存储输入的 `bucket_name`，因此 `bucket_name` 的使用完毕后即可释放
/// @warning 务必在使用完毕后调用 `qiniu_ng_bucket_builder_free()` 方法释放 `qiniu_ng_bucket_builder_t`
/// @warning 务必保证在 `qiniu_ng_bucket_builder_t` 和其构建的 `qiniu_ng_bucket_t` 没有被释放之前，不要释放传入的 `qiniu_ng_client_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_builder_new(
    client: qiniu_ng_client_t,
    bucket_name: *const qiniu_ng_char_t,
) -> qiniu_ng_bucket_builder_t {
    let client = Option::<Box<Client>>::from(client).unwrap();
    let bucket_name = unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap();
    qiniu_ng_bucket_builder_t::from(Box::new(client.storage().bucket(bucket_name))).tap(|_| {
        let _ = qiniu_ng_client_t::from(client);
    })
}

/// @brief 指定存储空间区域
/// @param[in] builder 存储空间生成器
/// @param[in] region 区域实例
/// @note
///     对于之前尚未指定过存储空间区域的情况，该方法将为存储空间指定区域。
///     而一旦指定过，之后调用该方法则表示指定备用区域。
/// @warning 务必保证在 `qiniu_ng_bucket_builder_t` 和其构建的 `qiniu_ng_bucket_t` 没有被释放之前，不要释放传入的 `qiniu_ng_region_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_builder_set_region(builder: qiniu_ng_bucket_builder_t, region: qiniu_ng_region_t) {
    let mut builder = Option::<Box<BucketBuilder>>::from(builder).unwrap();
    let region = Option::<Box<Cow<Region>>>::from(region).unwrap();
    let region = Box::leak(region);
    builder.region(region.as_ref());
    let _ = qiniu_ng_bucket_builder_t::from(builder);
}

/// @brief 指定存储空间区域
/// @param[in] builder 存储空间生成器
/// @param[in] region_id 区域 ID
/// @note
///     该方法仅适用于指定七牛公有云区域。
///     如果使用的是私有云，则请调用 `qiniu_ng_bucket_builder_set_region()` 方法。
/// @note
///     对于之前尚未指定过存储空间区域的情况，该方法将为存储空间指定区域。
///     而一旦指定过，之后调用该方法则表示指定备用区域。
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_builder_set_region_id(
    builder: qiniu_ng_bucket_builder_t,
    region_id: qiniu_ng_region_id_t,
) {
    let mut builder = Option::<Box<BucketBuilder>>::from(builder).unwrap();
    builder.region_id(region_id.into());
    let _ = qiniu_ng_bucket_builder_t::from(builder);
}

/// @brief 自动检测区域
/// @details 将连接七牛服务器查询当前存储空间所在区域和备用区域
/// @param[in] builder 存储空间生成器
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @note
///     注意，如果调用了该方法，则不应该再调用 `qiniu_ng_bucket_builder_set_region()` 或 `qiniu_ng_bucket_builder_set_region_id()` 方法。
///     除非有特殊需求，否则不建议您调用该方法，而是尽量使用懒加载的方式在必要时自动检测区域
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_builder_auto_detect_region(
    builder: qiniu_ng_bucket_builder_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let mut builder = Option::<Box<BucketBuilder>>::from(builder).unwrap();
    let mut result = true;
    if let Err(err) = &builder.auto_detect_region() {
        if let Some(error) = unsafe { error.as_mut() } {
            *error = err.into();
        }
        result = false;
    }
    let _ = qiniu_ng_bucket_builder_t::from(builder);
    result
}

/// @brief 新增下载域名
/// @param[in] builder 存储空间生成器
/// @param[in] domain 下载域名
/// @note 可以先调用 `qiniu_ng_bucket_builder_auto_detect_domains()` 方法然后再调用该方法，SDK 将优先使用最后新增的域名
/// @note 调用该方法时，输入的 `domain` 将被复制并存储，因此 `domain` 的调用完毕后即可释放
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_builder_prepend_domain(
    builder: qiniu_ng_bucket_builder_t,
    domain: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<BucketBuilder>>::from(builder).unwrap();
    builder.prepend_domain(unsafe { ucstr::from_ptr(domain) }.to_string().unwrap());
    let _ = qiniu_ng_bucket_builder_t::from(builder);
}

/// @brief 自动检测下载域名
/// @details 将连接七牛服务器查询当前存储空间的下载域名列表
/// @param[in] builder 存储空间生成器
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `false`，则表示可以读取 `error` 获得错误信息
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_builder_auto_detect_domains(
    builder: qiniu_ng_bucket_builder_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let mut builder = Option::<Box<BucketBuilder>>::from(builder).unwrap();
    let mut result = true;
    if let Err(err) = &builder.auto_detect_domains() {
        if let Some(error) = unsafe { error.as_mut() } {
            *error = err.into();
        }
        result = false;
    }
    let _ = qiniu_ng_bucket_builder_t::from(builder);
    result
}

/// @brief 生成存储空间实例
/// @param[in] builder 存储空间生成器
/// @retval qiniu_ng_bucket_t 获取生成的存储空间实例
/// @note 注意，该方法仅用于在 SDK 中配置生成存储空间实例，而非在七牛云服务器上创建新的存储空间
/// @warning 务必在使用 `qiniu_ng_bucket_t` 完毕后调用 `qiniu_ng_bucket_free()` 方法释放 `qiniu_ng_bucket_t`
/// @warning 在调用完毕后 `qiniu_ng_bucket_builder_t` 依然需要被 `qiniu_ng_bucket_builder_free()` 释放
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_build(builder: qiniu_ng_bucket_builder_t) -> qiniu_ng_bucket_t {
    let builder = Option::<Box<BucketBuilder>>::from(builder).unwrap();
    qiniu_ng_bucket_t::from(Box::new(builder.build())).tap(|_| {
        let _ = qiniu_ng_bucket_builder_t::from(builder);
    })
}

/// @brief 重置存储空间生成器实例
/// @details 调用该方法使生成器可以被多次复用
/// @param[in] builder 存储空间生成器
/// @param[in] new_bucket_name 新的存储空间名称
/// @note 重置实例时，SDK 客户端会复制并存储输入的 `bucket_name`，因此 `bucket_name` 的使用完毕后即可释放
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_builder_reset(
    builder: qiniu_ng_bucket_builder_t,
    new_bucket_name: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<BucketBuilder>>::from(builder).unwrap();
    let new_bucket_name = unsafe { ucstr::from_ptr(new_bucket_name) }.to_string().unwrap();
    builder.reset(new_bucket_name);
    let _ = qiniu_ng_bucket_builder_t::from(builder);
}

/// @brief 释放存储空间生成器实例
/// @param[in,out] builder 存储空间生成器实例地址，释放完毕后该生成器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_builder_free(builder: *mut qiniu_ng_bucket_builder_t) {
    if let Some(builder) = unsafe { builder.as_mut() } {
        let _ = Option::<Box<BucketBuilder>>::from(*builder);
        *builder = qiniu_ng_bucket_builder_t::default();
    }
}

/// @brief 判断存储空间生成器实例是否已经被释放
/// @param[in] builder 存储空间生成器实例
/// @retval bool 如果返回 `true` 则表示存储空间生成器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_builder_is_freed(builder: qiniu_ng_bucket_builder_t) -> bool {
    builder.is_null()
}

/// @brief 存储空间实例
/// @details 封装存储空间相关数据，例如配置，区域，下载域名等
/// @note
///   * 调用 `qiniu_ng_bucket_new()` 函数创建 `qiniu_ng_bucket_t` 实例。
///   * 当 `qiniu_ng_bucket_t` 使用完毕后，请务必调用 `qiniu_ng_bucket_free()` 方法释放内存。
/// @note
///   该结构体可以跨线程使用，SDK 确保其使用的线程安全
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_bucket_t(*mut c_void);

impl Default for qiniu_ng_bucket_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_bucket_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl<'r> From<qiniu_ng_bucket_t> for Option<Box<Bucket<'r>>> {
    fn from(bucket: qiniu_ng_bucket_t) -> Self {
        if bucket.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(bucket)) })
        }
    }
}

impl<'r> From<Option<Box<Bucket<'r>>>> for qiniu_ng_bucket_t {
    fn from(bucket: Option<Box<Bucket>>) -> Self {
        bucket.map(|bucket| bucket.into()).unwrap_or_default()
    }
}

impl<'r> From<Box<Bucket<'r>>> for qiniu_ng_bucket_t {
    fn from(bucket: Box<Bucket>) -> Self {
        unsafe { transmute(Box::into_raw(bucket)) }
    }
}

/// @brief 创建存储空间实例
/// @param[in] client 七牛 SDK 客户端实例
/// @param[in] bucket_name 存储空间名称
/// @retval qiniu_ng_bucket_t 获取创建的存储空间实例
/// @note 注意，该方法仅用于在 SDK 中配置生成存储空间实例，而非在七牛云服务器上创建新的存储空间
/// @note 创建实例时，SDK 客户端会复制并存储输入的 `bucket_name`，因此 `bucket_name` 的使用完毕后即可释放
/// @warning 务必在使用完毕后调用 `qiniu_ng_bucket_free()` 方法释放 `qiniu_ng_bucket_t`
/// @warning 务必保证在 `qiniu_ng_bucket_t` 没有被释放之前，不要释放传入的 `qiniu_ng_client_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_new(
    client: qiniu_ng_client_t,
    bucket_name: *const qiniu_ng_char_t,
) -> qiniu_ng_bucket_t {
    let client = Option::<Box<Client>>::from(client).unwrap();
    let bucket_name = unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap();
    qiniu_ng_bucket_t::from(Box::new(client.storage().bucket(bucket_name).build())).tap(|_| {
        let _ = qiniu_ng_client_t::from(client);
    })
}

/// @brief 释放存储空间实例
/// @param[in,out] bucket 存储空间实例地址，释放完毕后该存储空间实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_free(bucket: *mut qiniu_ng_bucket_t) {
    if let Some(bucket) = unsafe { bucket.as_mut() } {
        let _ = Option::<Box<Bucket>>::from(*bucket);
        *bucket = qiniu_ng_bucket_t::default();
    }
}

/// @brief 判断存储空间实例是否已经被释放
/// @param[in] bucket 存储空间实例
/// @retval bool 如果返回 `true` 则表示存储空间实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_is_freed(bucket: qiniu_ng_bucket_t) -> bool {
    bucket.is_null()
}

/// @brief 获取存储空间名称
/// @param[in] bucket 存储空间实例
/// @retval qiniu_ng_str_t 返回存储空间的名称
/// @warning 务必记得 `qiniu_ng_str_t` 需要在使用完毕后调用 `qiniu_ng_str_free()` 释放内存。
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_get_name(bucket: qiniu_ng_bucket_t) -> qiniu_ng_str_t {
    let bucket = Option::<Box<Bucket>>::from(bucket).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(bucket.name()) }.tap(|_| {
        let _ = qiniu_ng_bucket_t::from(bucket);
    })
}

/// @brief 获取存储空间区域
/// @param[in] bucket 存储空间实例
/// @param[out] region 用于返回区域的内存地址，如果传入 `NULL` 表示不获取 `region`。但如果运行正常，返回值将依然是 `true`
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `region` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `region` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_get_region(
    bucket: qiniu_ng_bucket_t,
    region: *mut qiniu_ng_region_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let bucket = Option::<Box<Bucket>>::from(bucket).unwrap();
    match bucket.region().map(|region| region.to_owned()).tap(|_| {
        let _ = qiniu_ng_bucket_t::from(bucket);
    }) {
        Ok(r) => {
            if let Some(region) = unsafe { region.as_mut() } {
                *region = Box::<Cow<Region>>::new(Cow::Owned(r)).into();
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

/// @brief 获取存储空间区域列表
/// @param[in] bucket 存储空间实例
/// @param[out] regions 用于返回区域列表的内存地址，区域列表中第一个区域是当前存储空间所在区域，之后的区域则是备用区域。如果传入 `NULL` 表示不获取 `regions`。但如果运行正常，返回值将依然是 `true`
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `regions` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `regions` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_get_regions(
    bucket: qiniu_ng_bucket_t,
    regions: *mut qiniu_ng_regions_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let bucket = Option::<Box<Bucket>>::from(bucket).unwrap();
    match bucket
        .regions()
        .map(|iter| iter.map(|r| r.to_owned()).collect::<Box<[Region]>>())
        .tap(|_| {
            let _ = qiniu_ng_bucket_t::from(bucket);
        }) {
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

/// @brief 获取存储空间下载域名列表
/// @param[in] bucket 存储空间实例
/// @param[out] domains 用于返回下载域名列表的内存地址。如果传入 `NULL` 表示不获取 `domains`。但如果运行正常，返回值将依然是 `true`
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示可以读取 `domains` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `domains` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_get_domains(
    bucket: qiniu_ng_bucket_t,
    domains: *mut qiniu_ng_str_list_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let bucket = Option::<Box<Bucket>>::from(bucket).unwrap();
    match bucket
        .domains()
        .map(|domains| unsafe { qiniu_ng_str_list_t::from_str_slice_unchecked(&domains) })
        .tap(|_| {
            let _ = qiniu_ng_bucket_t::from(bucket);
        }) {
        Ok(ds) => {
            if let Some(domains) = unsafe { domains.as_mut() } {
                *domains = ds;
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
