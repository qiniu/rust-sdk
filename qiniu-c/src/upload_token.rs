use crate::{
    config::qiniu_ng_config_t,
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr},
    utils::{qiniu_ng_str_list_t, qiniu_ng_str_t},
};
use libc::{c_void, size_t};
use qiniu_ng::{
    storage::uploader::{UploadPolicy, UploadPolicyBuilder, UploadToken},
    Config, Credential,
};
use std::{
    mem::transmute,
    ptr::null_mut,
    time::{Duration, SystemTime},
};
use tap::TapOps;

/// @brief 上传策略生成器
/// @note
///   * 调用任一 `qiniu_ng_upload_policy_builder_new_for_` 系列函数创建 `qiniu_ng_upload_policy_builder_t` 实例。
///   * 调用一系列方法修改 `qiniu_ng_upload_policy_builder_t` 实例的数据。
///   * 调用 `qiniu_ng_upload_policy_build()` 生成 `qiniu_ng_upload_policy_t` 实例。
///   * 当通过 `qiniu_ng_upload_policy_builder_t` 生成 `qiniu_ng_upload_policy_t` 完毕后
///     - 当需要继续生成其他客户端配置实例时，可以调用 `qiniu_ng_upload_policy_builder_reset()` 方法重置生成器。
///     - 当没有其他生成需求时，请务必调用 `qiniu_ng_upload_policy_builder_free()` 方法释放内存。
/// @note
///   该结构体不可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_upload_policy_builder_t(*mut c_void);

impl Default for qiniu_ng_upload_policy_builder_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_upload_policy_builder_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_upload_policy_builder_t> for Option<Box<UploadPolicyBuilder<'_>>> {
    fn from(builder: qiniu_ng_upload_policy_builder_t) -> Self {
        if builder.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(builder)) })
        }
    }
}

impl From<Option<Box<UploadPolicyBuilder<'_>>>> for qiniu_ng_upload_policy_builder_t {
    fn from(builder: Option<Box<UploadPolicyBuilder>>) -> Self {
        builder.map(|builder| builder.into()).unwrap_or_default()
    }
}

impl From<Box<UploadPolicyBuilder<'_>>> for qiniu_ng_upload_policy_builder_t {
    fn from(builder: Box<UploadPolicyBuilder>) -> Self {
        unsafe { transmute(Box::into_raw(builder)) }
    }
}

/// @brief 为指定的存储空间生成的上传策略
/// @details
///     允许用户上传文件到指定的存储空间，不限制上传客户端指定对象名称。
///     且这种模式下生成的上传策略将被自动设置 `qiniu_ng_upload_policy_builder_set_insert_only()`，且不允许设置 `qiniu_ng_upload_policy_builder_set_overwritable()`，
///     因此上传时不能通过覆盖的方式修改同名对象。
/// @details
///     上传策略根据给出的客户端配置指定上传凭证有效期
/// @param[in] bucket_name 存储空间名称
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_upload_policy_builder_t 获取创建的上传策略生成器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_upload_policy_builder_free()` 方法释放 `qiniu_ng_upload_policy_builder_t`
/// @note 创建实例时，SDK 客户端会复制并存储输入的 `bucket_name`，因此 `bucket_name` 在使用完毕后即可释放
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_new_for_bucket(
    bucket_name: *const qiniu_ng_char_t,
    config: qiniu_ng_config_t,
) -> qiniu_ng_upload_policy_builder_t {
    let config = Option::<Config>::from(config).unwrap();
    Box::new(UploadPolicyBuilder::new_policy_for_bucket(
        unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap(),
        &config,
    ))
    .tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
    .into()
}

/// @brief 为指定的存储空间和对象名称生成的上传策略
/// @details
///     允许用户以指定的对象名称上传文件到指定的存储空间。
///     上传客户端不能指定与上传策略冲突的对象名称。
///     且这种模式下生成的上传策略将被自动指定 `qiniu_ng_upload_policy_builder_set_overwritable()`，
///     如果不希望允许同名对象被覆盖和修改，则应该调用 `qiniu_ng_upload_policy_builder_set_insert_only()`。
/// @details
///     上传策略根据给出的客户端配置指定上传凭证有效期
/// @param[in] bucket_name 存储空间名称
/// @param[in] key 对象名称
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_upload_policy_builder_t 获取创建的上传策略生成器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_upload_policy_builder_free()` 方法释放 `qiniu_ng_upload_policy_builder_t`
/// @note 创建实例时，SDK 客户端会复制并存储输入的 `bucket_name` 和 `key`，因此 `bucket_name` 和 `key` 在使用完毕后即可释放
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_new_for_object(
    bucket_name: *const qiniu_ng_char_t,
    key: *const qiniu_ng_char_t,
    config: qiniu_ng_config_t,
) -> qiniu_ng_upload_policy_builder_t {
    let config = Option::<Config>::from(config).unwrap();
    Box::new(UploadPolicyBuilder::new_policy_for_object(
        unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap(),
        unsafe { ucstr::from_ptr(key) }.to_string().unwrap(),
        &config,
    ))
    .tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
    .into()
}

/// @brief 为指定的存储空间和对象名称前缀生成的上传策略
/// @details
///     允许用户以指定的对象名称前缀上传文件到指定的存储空间。
///     上传客户端指定包含该前缀的对象名称。
///     且这种模式下生成的上传策略将被自动指定 `qiniu_ng_upload_policy_builder_set_overwritable()`，
///     如果不希望允许同名对象被覆盖和修改，则应该调用 `qiniu_ng_upload_policy_builder_set_insert_only()`。
/// @details
///     上传策略根据给出的客户端配置指定上传凭证有效期
/// @param[in] bucket_name 存储空间名称
/// @param[in] prefix 对象名称前缀
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_upload_policy_builder_t 获取创建的上传策略生成器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_upload_policy_builder_free()` 方法释放 `qiniu_ng_upload_policy_builder_t`
/// @note 创建实例时，SDK 客户端会复制并存储输入的 `bucket_name` 和 `prefix`，因此 `bucket_name` 和 `prefix` 在使用完毕后即可释放
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_new_for_objects_with_prefix(
    bucket_name: *const qiniu_ng_char_t,
    prefix: *const qiniu_ng_char_t,
    config: qiniu_ng_config_t,
) -> qiniu_ng_upload_policy_builder_t {
    let config = Option::<Config>::from(config).unwrap();
    Box::new(UploadPolicyBuilder::new_policy_for_objects_with_prefix(
        unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap(),
        unsafe { ucstr::from_ptr(prefix) }.to_string().unwrap(),
        &config,
    ))
    .tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
    .into()
}

/// @brief 指定上传凭证有效期
/// @param[in] builder 客户端配置生成器实例
/// @param[in] lifetime 上传凭证有效期，单位为秒
/// @note 默认将会使用客户端配置指定的上传凭证有效期
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_token_lifetime(
    builder: qiniu_ng_upload_policy_builder_t,
    lifetime: u64,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.token_lifetime(Duration::from_secs(lifetime));
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 指定上传凭证过期时间
/// @param[in] builder 客户端配置生成器实例
/// @param[in] deadline 过期时间，使用以秒为单位的 UNIX 时间戳表示
/// @note 默认将会使用客户端配置指定的上传凭证有效期
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_token_deadline(
    builder: qiniu_ng_upload_policy_builder_t,
    deadline: u64,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.token_deadline(
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(deadline))
            .unwrap(),
    );
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 仅允许创建新的对象，不允许覆盖和修改同名对象
/// @param[in] builder 客户端配置生成器实例
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_insert_only(builder: qiniu_ng_upload_policy_builder_t) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.insert_only();
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 允许覆盖和修改同名对象
/// @param[in] builder 客户端配置生成器实例
/// @retval 如果上传策略仅指定来存储空间而不指定对象名称或前缀，则调用该方法不会起效，将返回 `false`
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_overwritable(builder: qiniu_ng_upload_policy_builder_t) -> bool {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.overwritable().is_ok().tap(|_| {
        let _ = qiniu_ng_upload_policy_builder_t::from(builder);
    })
}

/// @brief 启用 MIME 类型自动检测
/// @param[in] builder 客户端配置生成器实例
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_enable_mime_detection(builder: qiniu_ng_upload_policy_builder_t) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.enable_mime_detection();
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 禁用 MIME 类型自动检测
/// @param[in] builder 客户端配置生成器实例
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_disable_mime_detection(builder: qiniu_ng_upload_policy_builder_t) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.disable_mime_detection();
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 使用低频存储
/// @param[in] builder 客户端配置生成器实例
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_use_infrequent_storage(builder: qiniu_ng_upload_policy_builder_t) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.infrequent_storage();
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 使用标准存储
/// @param[in] builder 客户端配置生成器实例
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_use_normal_storage(builder: qiniu_ng_upload_policy_builder_t) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.normal_storage();
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 指定 Web 端文件上传成功后，浏览器执行 303 跳转的 URL
/// @details
///     通常用于表单上传。
///     文件上传成功后会跳转到 `<return_url>?upload_ret=<queryString>`，
///     `<queryString>` 包含 `return_body()` 内容。
///     如不设置 `return_url`，则直接将 `return_body()` 的内容返回给客户端
/// @param[in] builder 客户端配置生成器实例
/// @param[in] return_url 跳转 URL
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_return_url(
    builder: qiniu_ng_upload_policy_builder_t,
    return_url: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.return_url(unsafe { ucstr::from_ptr(return_url) }.to_string().unwrap());
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 上传成功后，自定义七牛云最终返回给上传端（在指定 `qiniu_ng_upload_policy_builder_set_return_url()` 时是携带在跳转路径参数中）的数据
/// @details
///     支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)。
///     `return_body` 要求是合法的 JSON 文本。
///     例如 `{"key": $(key), "hash": $(etag), "w": $(imageInfo.width), "h": $(imageInfo.height)}`
/// @param[in] builder 客户端配置生成器实例
/// @param[in] return_body 返回数据
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_return_body(
    builder: qiniu_ng_upload_policy_builder_t,
    return_body: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.return_body(unsafe { ucstr::from_ptr(return_body) }.to_string().unwrap());
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 上传成功后，七牛云向业务服务器发送 POST 请求的 URL 列表，`Host`，回调请求的内容以及其 `Content-Type`
/// @details 七牛服务器会在上传成功后逐一回调 URL 直到有一个成功为止
/// @param[in] builder 客户端配置生成器实例
/// @param[in] callback_urls 回调 URL 列表
/// @param[in] callback_urls_size 回调 URL 列表长度
/// @param[in] callback_host 回调时的 `Host`，如果传入 `NULL` 则将使用默认的 `Host`
/// @param[in] body 回调请求体，必须不能为 `NULL`，支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
/// @param[in] body_type 回调请求体的 `Content-Type`，如果传入 `NULL` 则表示为默认的 `application/x-www-form-urlencoded`
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_callback(
    builder: qiniu_ng_upload_policy_builder_t,
    callback_urls: *const *const qiniu_ng_char_t,
    callback_urls_size: size_t,
    callback_host: *const qiniu_ng_char_t,
    body: *const qiniu_ng_char_t,
    body_type: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.callback(
        Vec::<String>::with_capacity(callback_urls_size)
            .tap(|urls| {
                for i in 0..callback_urls_size {
                    urls.push(unsafe { ucstr::from_ptr(*callback_urls.add(i)) }.to_string().unwrap());
                }
            })
            .iter()
            .map(|url| url.as_ref())
            .collect::<Box<[_]>>(),
        unsafe { callback_host.as_ref() }
            .map(|callback_host| unsafe { ucstr::from_ptr(callback_host) }.to_string().unwrap())
            .unwrap_or_else(String::new),
        unsafe { ucstr::from_ptr(body) }.to_string().unwrap(),
        unsafe { body_type.as_ref() }
            .map(|body_type| unsafe { ucstr::from_ptr(body_type) }.to_string().unwrap())
            .unwrap_or_else(String::new),
    );
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 设置自定义对象名称
/// @details
///     支持支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
/// @param[in] builder 客户端配置生成器实例
/// @param[in] save_as 对象名称
/// @param[in] force 当它为 `false` 时，`save_as` 字段仅当用户上传的时候没有主动指定对象名时起作用。当它为 `true` 时，将强制按 `save_as` 字段的格式命名
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_save_as_key(
    builder: qiniu_ng_upload_policy_builder_t,
    save_as: *const qiniu_ng_char_t,
    force: bool,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.save_as(unsafe { ucstr::from_ptr(save_as) }.to_string().unwrap(), force);
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 限定上传文件尺寸的范围
/// @param[in] builder 客户端配置生成器实例
/// @param[in] min_file_size 上传文件尺寸下限，单位为字节，如果为 `0` 则表示不限制
/// @param[in] max_file_size 上传文件尺寸上限，单位为字节，如果为 `0` 则表示不限制
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_file_size_limitation(
    builder: qiniu_ng_upload_policy_builder_t,
    min_file_size: size_t,
    max_file_size: size_t,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    match (min_file_size, max_file_size) {
        (0, 0) => {
            builder.file_size_limitation(..);
        }
        (0, max_file_size) => {
            builder.file_size_limitation(..=max_file_size);
        }
        (min_file_size, 0) => {
            builder.file_size_limitation(min_file_size..);
        }
        (min_file_size, max_file_size) => {
            builder.file_size_limitation(min_file_size..=max_file_size);
        }
    };
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 限定用户上传的文件类型
/// @details
///     指定本字段值，七牛服务器会侦测文件内容以判断 MIME 类型，再用判断值跟指定值进行匹配，
///     匹配成功则允许上传，匹配失败则返回 403 状态码
/// @param[in] builder 客户端配置生成器实例
/// @param[in] mime_types MIME 类型列表
/// @param[in] mime_types_size MIME 类型列表尺寸
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_mime_types(
    builder: qiniu_ng_upload_policy_builder_t,
    mime_types: *const *const qiniu_ng_char_t,
    mime_types_size: size_t,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.mime_types(
        Vec::<String>::with_capacity(mime_types_size)
            .tap(|m| {
                for i in 0..mime_types_size {
                    m.push(unsafe { ucstr::from_ptr(*mime_types.add(i)) }.to_string().unwrap());
                }
            })
            .iter()
            .map(|m| m.as_ref())
            .collect::<Box<[&str]>>(),
    );
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 指定对象生命周期
/// @param[in] builder 客户端配置生成器实例
/// @param[in] lifetime 对象生命周期，单位为秒，但只能精确到天
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_object_lifetime(
    builder: qiniu_ng_upload_policy_builder_t,
    lifetime: u64,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.object_lifetime(Duration::from_secs(lifetime));
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 指定对象生命到期时间
/// @param[in] builder 客户端配置生成器实例
/// @param[in] deadline 过期时间，使用以秒为单位的 UNIX 时间戳表示，但只能精确到天
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_object_deadline(
    builder: qiniu_ng_upload_policy_builder_t,
    deadline: u64,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.object_deadline(
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(deadline))
            .unwrap(),
    );
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 上传策略
/// @details 可以点击[这里](https://developer.qiniu.com/kodo/manual/1206/put-policy)了解七牛安全机制
/// @note
///   * 调用 `qiniu_ng_upload_policy_builder_t` 生成器生成 `qiniu_ng_upload_policy_t` 实例。
///   * 当 `qiniu_ng_upload_policy_t` 使用完毕后，请务必调用 `qiniu_ng_upload_policy_free()` 方法释放内存。
/// @note
///   所有上传策略均为只读，因此可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_upload_policy_t(*mut c_void);

impl Default for qiniu_ng_upload_policy_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_upload_policy_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_upload_policy_t> for Option<Box<UploadPolicy<'_>>> {
    fn from(upload_policy: qiniu_ng_upload_policy_t) -> Self {
        if upload_policy.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(upload_policy)) })
        }
    }
}

impl From<Option<Box<UploadPolicy<'_>>>> for qiniu_ng_upload_policy_t {
    fn from(upload_policy: Option<Box<UploadPolicy>>) -> Self {
        upload_policy
            .map(|upload_policy| upload_policy.into())
            .unwrap_or_default()
    }
}

impl From<Box<UploadPolicy<'_>>> for qiniu_ng_upload_policy_t {
    fn from(upload_policy: Box<UploadPolicy>) -> Self {
        unsafe { transmute(Box::into_raw(upload_policy)) }
    }
}

/// @brief 生成上传策略实例
/// @param[in] builder 上传策略生成器实例
/// @retval qiniu_ng_upload_policy_t 返回创建的上传策略实例
/// @warning 务必在使用 `qiniu_ng_upload_policy_t` 完毕后调用 `qiniu_ng_upload_policy_free()` 方法释放 `qiniu_ng_upload_policy_t`
/// @warning 在调用完毕后 `qiniu_ng_upload_policy_builder_t` 依然需要被 `qiniu_ng_upload_policy_builder_free()` 释放
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_build(builder: qiniu_ng_upload_policy_builder_t) -> qiniu_ng_upload_policy_t {
    let builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    qiniu_ng_upload_policy_t::from(Box::new(builder.build())).tap(|_| {
        let _ = qiniu_ng_upload_policy_builder_t::from(builder);
    })
}

/// @brief 重置上传策略生成器实例
/// @details 调用该方法使生成器可以被多次复用
/// @param[in] builder 上传策略生成器
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_reset(builder: qiniu_ng_upload_policy_builder_t) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    builder.reset();
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

/// @brief 释放上传策略生成器实例
/// @param[in,out] builder 上传策略生成器实例地址，释放完毕后该生成器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_free(builder: *mut qiniu_ng_upload_policy_builder_t) {
    if let Some(builder) = unsafe { builder.as_mut() } {
        let _ = Option::<Box<UploadPolicyBuilder>>::from(*builder);
        *builder = qiniu_ng_upload_policy_builder_t::default();
    }
}

/// @brief 判断上传策略生成器实例是否已经被释放
/// @param[in] builder 上传策略生成器实例
/// @retval bool 如果返回 `true` 则表示上传策略生成器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_is_freed(builder: qiniu_ng_upload_policy_builder_t) -> bool {
    builder.is_null()
}

/// @brief 获取上传策略的存储空间约束
/// @param[in] upload_policy 上传策略实例
/// @retval qiniu_ng_str_t 存储空间约束
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_bucket(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.bucket()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 获取上传策略的对象名称约束或对象名称前缀约束
/// @param[in] upload_policy 上传策略实例
/// @retval qiniu_ng_str_t 对象名称约束或对象名称前缀约束
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_key(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.key()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 上传策略是否是对象名称前缀约束
/// @param[in] upload_policy 上传策略实例
/// @retval bool 是否是对象名称前缀约束
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_use_prefixal_object_key(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.use_prefixal_object_key().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 是否仅允许新增对象，不允许覆盖对象
/// @param[in] upload_policy 上传策略实例
/// @retval bool 是否仅允许新增对象，不允许覆盖对象
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_insert_only(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.is_insert_only().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 是否允许覆盖对象
/// @param[in] upload_policy 上传策略实例
/// @retval bool 是否允许覆盖对象
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_overwritable(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.is_overwritable().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 是否启用 MIME 类型自动检测
/// @param[in] upload_policy 上传策略实例
/// @retval bool 是否启用 MIME 类型自动检测
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_mime_detection_enabled(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.mime_detection_enabled().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 获取上传策略的上传凭证有效期
/// @param[in] upload_policy 上传策略实例
/// @param[out] lifetime 用于获取上传凭证有效期，单位为秒，如果传入 `NULL` 表示不获取 `lifetime`，但如果上传凭证有效期存在，返回值依然是 `true`
/// @retval bool 如果上传凭证有效期存在，则返回 `true`，否则返回 `false`
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_token_lifetime(
    upload_policy: qiniu_ng_upload_policy_t,
    lifetime: *mut u64,
) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    if let Some(token_lifetime) = upload_policy.token_lifetime() {
        if let Some(lifetime) = unsafe { lifetime.as_mut() } {
            *lifetime = token_lifetime.as_secs()
        }
        true
    } else {
        false
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 获取上传策略的上传凭证过期时间
/// @param[in] upload_policy 上传策略实例
/// @param[out] lifetime 用于获取上传凭证过期时间，使用以秒为单位的 UNIX 时间戳表示，如果传入 `NULL` 表示不获取 `deadline`，但如果上传凭证过期时间存在，返回值依然是 `true`
/// @retval bool 如果上传凭证过期时间存在，则返回 `true`，否则返回 `false`
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_token_deadline(
    upload_policy: qiniu_ng_upload_policy_t,
    deadline: *mut u64,
) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    if let Some(token_deadline) = upload_policy.token_deadline() {
        if let Some(deadline) = unsafe { deadline.as_mut() } {
            *deadline = token_deadline.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        }
        true
    } else {
        false
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief Web 端文件上传成功后，浏览器执行 303 跳转的 URL
/// @param[in] upload_policy 上传策略实例
/// @retval qiniu_ng_str_t 浏览器执行 303 跳转的 URL
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_return_url(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.return_url()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 上传成功后，自定义七牛云最终返回给上传端的数据
/// @param[in] upload_policy 上传策略实例
/// @retval qiniu_ng_str_t 返回给上传端的数据
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_return_body(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.return_body()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 上传成功后，七牛云向业务服务器发送 POST 请求的 URL 列表
/// @param[in] upload_policy 上传策略实例
/// @retval qiniu_ng_str_list_t 回调请求的 URL 列表
/// @note 这里返回的 `qiniu_ng_str_list_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_list_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_list_t` 使用完毕后，请务必调用 `qiniu_ng_str_list_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_callback_urls(
    upload_policy: qiniu_ng_upload_policy_t,
) -> qiniu_ng_str_list_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe {
        qiniu_ng_str_list_t::from_optional_str_slice_unchecked(
            upload_policy
                .callback_urls()
                .map(|urls| urls.collect::<Box<[&str]>>())
                .as_ref()
                .map(|urls| urls.as_ref()),
        )
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 上传成功后，七牛云向业务服务器发送回调请求时的 `Host`
/// @param[in] upload_policy 上传策略实例
/// @retval qiniu_ng_str_t 返回回调请求的 `Host`
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_callback_host(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.callback_host()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 上传成功后，七牛云向业务服务器发送回调请求时的内容
/// @details
///     支持[魔法变量](https://developer.qiniu.com/kodo/manual/1235/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
/// @param[in] upload_policy 上传策略实例
/// @retval qiniu_ng_str_t 返回回调请求的请求体
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_callback_body(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.callback_body()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 上传成功后，七牛云向业务服务器发送回调请求时的 `Content-Type`
/// @param[in] upload_policy 上传策略实例
/// @retval qiniu_ng_str_t 返回回调请求的 `Content-Type`
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_callback_body_type(
    upload_policy: qiniu_ng_upload_policy_t,
) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.callback_body_type()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 获取上传策略中的自定义对象名称
/// @param[in] upload_policy 上传策略实例
/// @retval qiniu_ng_str_t 返回自定义对象名称
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_save_key(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.save_key()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 是否忽略客户端指定的对象名称
/// @param[in] upload_policy 上传策略实例
/// @retval bool 是否忽略客户端指定的对象名称
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_save_key_forced(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.is_save_key_forced().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 获取上传策略中的上传文件尺寸的范围限定
/// @param[in] upload_policy 上传策略实例
/// @param[out] min_file_size 用于返回上传文件尺寸上限，如果传入 `NULL` 表示不获取 `min_file_size`，但不影响其他字段的获取
/// @param[out] max_file_size 用于返回上传文件尺寸下限，如果传入 `NULL` 表示不获取 `max_file_size`，但不影响其他字段的获取
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_file_size_limitation(
    upload_policy: qiniu_ng_upload_policy_t,
    min_file_size: *mut size_t,
    max_file_size: *mut size_t,
) {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    let (file_size_min, file_size_max) = upload_policy.file_size_limitation();
    unsafe {
        if let Some(min_file_size) = min_file_size.as_mut() {
            *min_file_size = file_size_min.unwrap_or(0);
        }
        if let Some(max_file_size) = max_file_size.as_mut() {
            *max_file_size = file_size_max.unwrap_or(0);
        }
    }
    let _ = qiniu_ng_upload_policy_t::from(upload_policy);
}

/// @brief 获取上传策略中的 MIME 类型限定
/// @param[in] upload_policy 上传策略实例
/// @retval qiniu_ng_str_list_t 返回 MIME 类型限定
/// @note 这里返回的 `qiniu_ng_str_list_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_list_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_list_t` 使用完毕后，请务必调用 `qiniu_ng_str_list_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_mime_types(
    upload_policy: qiniu_ng_upload_policy_t,
) -> qiniu_ng_str_list_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe {
        qiniu_ng_str_list_t::from_optional_str_slice_unchecked(
            upload_policy
                .mime_types()
                .map(|types| types.collect::<Box<[&str]>>())
                .as_ref()
                .map(|types| types.as_ref()),
        )
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 是否会使用标准存储
/// @param[in] upload_policy 上传策略实例
/// @retval bool 是否会使用标准存储
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_normal_storage_used(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.is_normal_storage_used().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 是否会使用低频存储
/// @param[in] upload_policy 上传策略实例
/// @retval bool 是否会使用低频存储
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_infrequent_storage_used(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.is_infrequent_storage_used().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 获取上传策略的对象生命周期
/// @param[in] upload_policy 上传策略实例
/// @param[out] lifetime 用于获取对象生命周期，单位为秒，但只能精确到天，如果传入 `NULL` 表示不获取 `lifetime`，但如果对象生命周期存在，返回值依然是 `true`
/// @retval bool 如果对象生命周期存在，则返回 `true`，否则返回 `false`
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_object_lifetime(
    upload_policy: qiniu_ng_upload_policy_t,
    lifetime: *mut u64,
) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    if let Some(object_lifetime) = upload_policy.object_lifetime() {
        if let Some(lifetime) = unsafe { lifetime.as_mut() } {
            *lifetime = object_lifetime.as_secs()
        }
        true
    } else {
        false
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 获取上传策略的对象生命结束时间
/// @param[in] upload_policy 上传策略实例
/// @param[out] lifetime 用于获取对象生命结束时间，使用以秒为单位的 UNIX 时间戳表示，如果传入 `NULL` 表示不获取 `deadline`，但如果对象生命结束时间存在，返回值依然是 `true`
/// @retval bool 如果对象生命结束时间存在，则返回 `true`，否则返回 `false`
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_object_deadline(
    upload_policy: qiniu_ng_upload_policy_t,
    deadline: *mut u64,
) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    if let Some(object_deadline) = upload_policy.object_deadline() {
        if let Some(deadline) = unsafe { deadline.as_mut() } {
            *deadline = object_deadline
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
        true
    } else {
        false
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 以 JSON 格式获取上传策略
/// @param[in] upload_policy 上传策略实例
/// @retval qiniu_ng_str_t 返回 JSON 格式的上传策略
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_as_json(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(upload_policy.as_json()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

/// @brief 以 JSON 格式生成上传策略
/// @param[in] json JSON 格式的上传策略
/// @param[out] upload_policy 用于返回上传策略实例
/// @param[out] err 用于返回错误，如果传入 `NULL` 表示不获取 `err`。但如果解析 JSON 发生错误，返回值将依然是 `false`
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_from_json(
    json: *const qiniu_ng_char_t,
    upload_policy: *mut qiniu_ng_upload_policy_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    match UploadPolicy::from_json(&unsafe { ucstr::from_ptr(json) }.to_string().unwrap()) {
        Ok(policy) => {
            if let Some(upload_policy) = unsafe { upload_policy.as_mut() } {
                *upload_policy = Box::new(policy).into();
            }
            true
        }
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            false
        }
    }
}

/// @brief 释放上传策略实例
/// @param[in,out] policy 上传策略实例地址，释放完毕后该实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_free(policy: *mut qiniu_ng_upload_policy_t) {
    if let Some(policy) = unsafe { policy.as_mut() } {
        let _ = Option::<Box<UploadPolicy>>::from(*policy);
        *policy = qiniu_ng_upload_policy_t::default();
    }
}

/// @brief 判断上传策略实例是否已经被释放
/// @param[in] config 上传策略实例
/// @retval bool 如果返回 `true` 则表示上传策略实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_freed(policy: qiniu_ng_upload_policy_t) -> bool {
    policy.is_null()
}

/// @brief 上传凭证
/// @details 可以点击[这里](https://developer.qiniu.com/kodo/manual/1208/upload-token)了解七牛安全机制
/// @warning 当 `qiniu_ng_upload_token_t` 使用完毕后，请务必调用 `qiniu_ng_upload_token_free()` 方法释放内存。
/// @note 所有上传凭证均为只读，因此可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_upload_token_t(*mut c_void);

impl Default for qiniu_ng_upload_token_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_upload_token_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_upload_token_t> for Option<Box<UploadToken<'_>>> {
    fn from(upload_token: qiniu_ng_upload_token_t) -> Self {
        if upload_token.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(upload_token)) })
        }
    }
}

impl From<Option<Box<UploadToken<'_>>>> for qiniu_ng_upload_token_t {
    fn from(upload_token: Option<Box<UploadToken>>) -> Self {
        upload_token.map(|upload_token| upload_token.into()).unwrap_or_default()
    }
}

impl From<Box<UploadToken<'_>>> for qiniu_ng_upload_token_t {
    fn from(upload_token: Box<UploadToken>) -> Self {
        unsafe { transmute(Box::into_raw(upload_token)) }
    }
}

/// @brief 根据上传策略生成器生成上传凭证实例
/// @param[in] policy_builder 上传凭证生成器实例
/// @param[out] access_key 用于签发上传凭证的七牛 Access Key
/// @param[out] secret_key 用于签发上传凭证的七牛 Secret Key
/// @retval qiniu_ng_upload_token_t 返回创建的上传凭证实例
/// @warning 务必在使用 `qiniu_ng_upload_token_t` 完毕后调用 `qiniu_ng_upload_token_free()` 方法释放 `qiniu_ng_upload_token_t`
/// @warning 在调用完毕后 `qiniu_ng_upload_policy_builder_t` 依然需要被 `qiniu_ng_upload_policy_builder_free()` 释放
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_new_from_policy_builder(
    policy_builder: qiniu_ng_upload_policy_builder_t,
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
) -> qiniu_ng_upload_token_t {
    let policy_builder = Option::<Box<UploadPolicyBuilder>>::from(policy_builder).unwrap();
    qiniu_ng_upload_token_t::from(Box::new(UploadToken::new(
        policy_builder.to_owned().build(),
        Credential::new(
            unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
            unsafe { ucstr::from_ptr(secret_key) }.to_string().unwrap(),
        ),
    )))
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_builder_t::from(policy_builder);
    })
}

/// @brief 根据上传策略生成上传凭证实例
/// @param[in] policy 上传凭证实例
/// @param[out] access_key 用于签发上传凭证的七牛 Access Key
/// @param[out] secret_key 用于签发上传凭证的七牛 Secret Key
/// @retval qiniu_ng_upload_token_t 返回创建的上传凭证实例
/// @warning 务必在使用 `qiniu_ng_upload_token_t` 完毕后调用 `qiniu_ng_upload_token_free()` 方法释放 `qiniu_ng_upload_token_t`
/// @warning 在调用完毕后 `qiniu_ng_upload_policy_t` 依然需要被 `qiniu_ng_upload_policy_free()` 释放
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_new_from_policy(
    policy: qiniu_ng_upload_policy_t,
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
) -> qiniu_ng_upload_token_t {
    let policy = Option::<Box<UploadPolicy>>::from(policy).unwrap();
    Box::new(UploadToken::new(
        policy.as_ref().to_owned(),
        Credential::new(
            unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
            unsafe { ucstr::from_ptr(secret_key) }.to_string().unwrap(),
        ),
    ))
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(policy);
    })
    .into()
}

/// @brief 将上传凭证字符串封装成上传凭证实例
/// @param[in] s 上传凭证字符串
/// @retval qiniu_ng_upload_token_t 返回创建的上传凭证实例
/// @warning 务必在使用 `qiniu_ng_upload_token_t` 完毕后调用 `qiniu_ng_upload_token_free()` 方法释放 `qiniu_ng_upload_token_t`
/// @warning 在调用完毕后 `qiniu_ng_upload_policy_t` 依然需要被 `qiniu_ng_upload_policy_free()` 释放
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_new_from(s: *const qiniu_ng_char_t) -> qiniu_ng_upload_token_t {
    Box::new(UploadToken::from(unsafe { ucstr::from_ptr(s) }.to_string().unwrap())).into()
}

/// @brief 释放上传凭证实例
/// @param[in,out] policy 上传凭证实例地址，释放完毕后该实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_free(token: *mut qiniu_ng_upload_token_t) {
    if let Some(token) = unsafe { token.as_mut() } {
        let _ = Option::<Box<UploadToken>>::from(*token);
        *token = qiniu_ng_upload_token_t::default();
    }
}

/// @brief 判断上传凭证实例是否已经被释放
/// @param[in] config 上传凭证实例
/// @retval bool 如果返回 `true` 则表示上传凭证实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_is_freed(token: qiniu_ng_upload_token_t) -> bool {
    token.is_null()
}

/// @brief 获取上传凭证实例中的七牛 Access Key
/// @param[in] upload_token 上传凭证实例
/// @param[out] access_key 用于返回七牛 Access Key
/// @param[out] err 用于返回错误，如果传入 `NULL` 表示不获取 `err`。但如果解析上传凭证发生错误，返回值将依然是 `false`
/// @retval bool 是否上传凭证解析正常，如果返回 `true`，则表示可以读取 `access_key` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 对于获取的 `access_key` 或 `error`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_get_access_key(
    upload_token: qiniu_ng_upload_token_t,
    access_key: *mut qiniu_ng_str_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let upload_token = Option::<Box<UploadToken>>::from(upload_token).unwrap();
    match upload_token.access_key() {
        Ok(ak) => {
            if let Some(access_key) = unsafe { access_key.as_mut() } {
                *access_key = unsafe { qiniu_ng_str_t::from_str_unchecked(ak) };
            }
            true
        }
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            false
        }
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_token_t::from(upload_token);
    })
}

/// @brief 从上传凭证实例中取出上传凭证字符串
/// @param[in] upload_token 上传凭证实例
/// @retval qiniu_ng_str_t 返回上传凭证字符串
/// @warning 在 `qiniu_ng_str_t` 被使用完毕后，必须调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_get_string(upload_token: qiniu_ng_upload_token_t) -> qiniu_ng_str_t {
    let upload_token = Option::<Box<UploadToken>>::from(upload_token).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(upload_token.to_string()) }.tap(|_| {
        let _ = qiniu_ng_upload_token_t::from(upload_token);
    })
}

/// @brief 从上传凭证实例中解析出上传策略
/// @param[in] upload_token 上传凭证实例
/// @param[out] policy 用于返回上传策略
/// @param[out] err 用于返回错误，如果传入 `NULL` 表示不获取 `err`。但如果解析上传凭证发生错误，返回值将依然是 `false`
/// @retval bool 是否上传凭证解析正常，如果返回 `true`，则表示可以读取 `policy` 获得结果，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 对于获取的 `policy` 或 `err`，一旦使用完毕，应该调用各自的内存释放方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_get_policy(
    upload_token: qiniu_ng_upload_token_t,
    policy: *mut qiniu_ng_upload_policy_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let upload_token = Option::<Box<UploadToken>>::from(upload_token).unwrap();
    match upload_token.policy() {
        Ok(upload_policy) => {
            if let Some(policy) = unsafe { policy.as_mut() } {
                *policy = Box::new(upload_policy.into_owned()).into();
            }
            true
        }
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            false
        }
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_token_t::from(upload_token);
    })
}
