use crate::{
    config::qiniu_ng_config_t,
    string::{qiniu_ng_char_t, ucstr},
    upload::qiniu_ng_upload_manager_t,
};
use libc::c_void;
use qiniu_ng::Client;
use std::{mem::transmute, ptr::null_mut};
use tap::TapOps;

/// @brief 七牛 SDK 客户端
/// @note
///     这里的客户端是针对七牛服务器而言，而并非指该结构体是运行在客户端应用程序上。
///     实际上，该结构体由于会存储用户的 SecretKey，因此不推荐在客户端应用程序上使用，而应该只在服务器端应用程序上使用。
/// @details 除了 Etag 和 上传功能外，`qiniu_ng_client_t` 是七牛大多数 API 调用的入口。
/// @note
///   * 调用 `qiniu_ng_client_new()` 或 `qiniu_ng_client_new_default()` 函数创建 `qiniu_ng_client_t` 实例。
///   * 当 `qiniu_ng_client_t` 使用完毕后，请务必调用 `qiniu_ng_client_free()` 方法释放内存。
/// @note
///   该结构体内部状态不可变，因此可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_client_t(*mut c_void);

impl Default for qiniu_ng_client_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_client_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_client_t> for Option<Box<Client>> {
    fn from(client: qiniu_ng_client_t) -> Self {
        if client.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(client)) })
        }
    }
}

impl From<Option<Box<Client>>> for qiniu_ng_client_t {
    fn from(client: Option<Box<Client>>) -> Self {
        client.map(|client| client.into()).unwrap_or_default()
    }
}

impl From<Box<Client>> for qiniu_ng_client_t {
    fn from(client: Box<Client>) -> Self {
        unsafe { transmute(Box::into_raw(client)) }
    }
}

/// @brief 创建 七牛 SDK 客户端实例
/// @param[in] access_key 七牛 Access Key
/// @param[in] secret_key 七牛 Secret Key
/// @param[in] config 七牛客户端配置
/// @retval qiniu_ng_client_t 获取创建的七牛 SDK 客户端实例
/// @note 创建实例时，SDK 客户端会复制并存储输入的 `access_key` 和 `secret_key`，因此 `access_key` 和 `secret_key` 在使用完毕后即可释放
/// @warning 务必在使用完毕后调用 `qiniu_ng_client_free()` 方法释放 `qiniu_ng_client_t`
/// @warning 务必在 `config` 被使用完毕后调用 `qiniu_ng_config_free()` 方法释放 `qiniu_ng_config_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_client_new(
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
    config: qiniu_ng_config_t,
) -> qiniu_ng_client_t {
    Box::new(Client::new(
        unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
        unsafe { ucstr::from_ptr(secret_key) }.to_string().unwrap(),
        config.get_clone().unwrap(),
    ))
    .into()
}

/// @brief 使用默认七牛客户端配置创建 七牛 SDK 客户端实例
/// @param[in] access_key 七牛 Access Key
/// @param[in] secret_key 七牛 Secret Key
/// @retval qiniu_ng_client_t 获取创建的七牛 SDK 客户端实例
/// @note 创建实例时，SDK 客户端会复制并存储输入的 `access_key` 和 `secret_key`，因此 `access_key` 和 `secret_key` 在使用完毕后即可释放
/// @warning 务必在使用完毕后调用 `qiniu_ng_client_free()` 方法释放 `qiniu_ng_client_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_client_new_default(
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
) -> qiniu_ng_client_t {
    Box::new(Client::new(
        unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
        unsafe { ucstr::from_ptr(secret_key) }.to_string().unwrap(),
        Default::default(),
    ))
    .into()
}

/// @brief 释放 七牛 SDK 客户端实例
/// @param[in,out] client 七牛 SDK 客户端实例地址，释放完毕后该客户端实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_client_free(client: *mut qiniu_ng_client_t) {
    if let Some(client) = unsafe { client.as_mut() } {
        let _ = Option::<Box<Client>>::from(*client);
        *client = qiniu_ng_client_t::default();
    }
}

/// @brief 判断 七牛 SDK 客户端实例是否已经被释放
/// @param[in] client 七牛 SDK 客户端实例
/// @retval bool 如果返回 `true` 则表示七牛 SDK 客户端实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_client_is_freed(client: qiniu_ng_client_t) -> bool {
    client.is_null()
}

/// @brief 创建上传管理器
/// @param[in] client 七牛 SDK 客户端实例
/// @retval qiniu_ng_upload_manager_t 获取创建的上传管理器
/// @note `qiniu_ng_upload_manager_new()` 可能是更简单的创建上传管理器的方法，仅需要客户端配置即可创建。
#[no_mangle]
pub extern "C" fn qiniu_ng_client_get_upload_manager(client: qiniu_ng_client_t) -> qiniu_ng_upload_manager_t {
    let client = Option::<Box<Client>>::from(client).unwrap();
    Box::new(client.upload().to_owned())
        .tap(|_| {
            let _ = qiniu_ng_client_t::from(client);
        })
        .into()
}
