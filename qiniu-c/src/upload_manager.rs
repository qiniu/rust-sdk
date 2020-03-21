use crate::config::qiniu_ng_config_t;
use libc::c_void;
use qiniu_ng::storage::uploader::UploadManager;
use std::{mem::transmute, ptr::null_mut};

/// @brief 上传管理器
/// @details 上传管理器可以用于构建存储空间上传器，或直接上传单个文件
/// @note
///   * 调用 `qiniu_ng_upload_manager_new()` 函数创建 `qiniu_ng_upload_manager_t` 实例。
///   * 当 `qiniu_ng_upload_manager_t` 使用完毕后，请务必调用 `qiniu_ng_upload_manager_free()` 方法释放内存。
/// @note
///   该结构体内部状态不可变，因此可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_upload_manager_t(*mut c_void);

impl Default for qiniu_ng_upload_manager_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_upload_manager_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_upload_manager_t> for Option<Box<UploadManager>> {
    fn from(upload_manager: qiniu_ng_upload_manager_t) -> Self {
        if upload_manager.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(upload_manager)) })
        }
    }
}

impl From<Option<Box<UploadManager>>> for qiniu_ng_upload_manager_t {
    fn from(upload_manager: Option<Box<UploadManager>>) -> Self {
        upload_manager
            .map(|upload_manager| upload_manager.into())
            .unwrap_or_default()
    }
}

impl From<Box<UploadManager>> for qiniu_ng_upload_manager_t {
    fn from(upload_manager: Box<UploadManager>) -> Self {
        unsafe { transmute(Box::into_raw(upload_manager)) }
    }
}

/// @brief 创建上传管理器实例
/// @param[in] config 七牛客户端配置
/// @retval qiniu_ng_upload_manager_t 获取创建的上传管理器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_upload_manager_free()` 方法释放 `qiniu_ng_upload_manager_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_new(config: qiniu_ng_config_t) -> qiniu_ng_upload_manager_t {
    Box::new(UploadManager::new(config.get_clone().unwrap())).into()
}

/// @brief 释放上传管理器实例
/// @param[in,out] upload_manager 上传管理器实例地址，释放完毕后该上传管理器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_free(upload_manager: *mut qiniu_ng_upload_manager_t) {
    if let Some(upload_manager) = unsafe { upload_manager.as_mut() } {
        let _ = Option::<Box<UploadManager>>::from(*upload_manager);
        *upload_manager = qiniu_ng_upload_manager_t::default();
    }
}

/// @brief 判断上传管理器实例是否已经被释放
/// @param[in] upload_manager 上传管理器实例
/// @retval bool 如果返回 `true` 则表示上传管理器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_is_freed(upload_manager: qiniu_ng_upload_manager_t) -> bool {
    upload_manager.is_null()
}
