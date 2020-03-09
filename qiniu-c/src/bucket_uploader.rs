use crate::{
    bucket::qiniu_ng_bucket_t,
    string::{qiniu_ng_char_t, ucstr},
    upload_manager::qiniu_ng_upload_manager_t,
};
use libc::{c_void, size_t};
use qiniu_ng::storage::{
    bucket::Bucket,
    uploader::{BucketUploader, UploadManager},
};
use std::{mem::transmute, ptr::null_mut};
use tap::TapOps;

/// @brief 存储空间上传器
/// @details 为指定存储空间的上传准备初始化数据，可以反复使用以上传多个文件
/// @note
///   * 调用 `qiniu_ng_bucket_uploader_new_from_bucket()` 或 `qiniu_ng_bucket_uploader_new_from_bucket_name()` 函数创建 `qiniu_ng_bucket_uploader_t` 实例。
///   * 当 `qiniu_ng_bucket_uploader_t` 使用完毕后，请务必调用 `qiniu_ng_bucket_uploader_free()` 方法释放内存。
/// @note
///   该结构体内部状态不可变，因此可以跨线程使用，但由于可能会自带线程池，所以不要跨进程使用，否则可能会发生线程池无法使用的问题
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_bucket_uploader_t(*mut c_void);

impl Default for qiniu_ng_bucket_uploader_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_bucket_uploader_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_bucket_uploader_t> for Option<BucketUploader> {
    fn from(bucket_uploader: qiniu_ng_bucket_uploader_t) -> Self {
        if bucket_uploader.is_null() {
            None
        } else {
            Some(unsafe { BucketUploader::from_raw(transmute(bucket_uploader)) })
        }
    }
}

impl From<Option<BucketUploader>> for qiniu_ng_bucket_uploader_t {
    fn from(bucket_uploader: Option<BucketUploader>) -> Self {
        bucket_uploader
            .map(|bucket_uploader| bucket_uploader.into())
            .unwrap_or_default()
    }
}

impl From<BucketUploader> for qiniu_ng_bucket_uploader_t {
    fn from(bucket_uploader: BucketUploader) -> Self {
        unsafe { transmute(bucket_uploader.into_raw()) }
    }
}

/// @brief 创建存储空间上传器实例
/// @param[in] upload_manager 上传管理器实例
/// @param[in] bucket 存储空间实例
/// @param[in] thread_pool_size 上传线程池大小，如果传入 `0`，则使用默认的线程池策略
/// @retval qiniu_ng_bucket_uploader_t 获取创建的存储空间上传器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_bucket_uploader_free()` 方法释放 `qiniu_ng_bucket_uploader_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_new_from_bucket(
    upload_manager: qiniu_ng_upload_manager_t,
    bucket: qiniu_ng_bucket_t,
    thread_pool_size: size_t,
) -> qiniu_ng_bucket_uploader_t {
    let upload_manager = Option::<Box<UploadManager>>::from(upload_manager).unwrap();
    let bucket = Option::<Box<Bucket>>::from(bucket).unwrap();
    let mut bucket_uploader_builder = upload_manager.for_bucket(&bucket).tap(|_| {
        let _ = qiniu_ng_bucket_t::from(bucket);
        let _ = qiniu_ng_upload_manager_t::from(upload_manager);
    });
    if thread_pool_size > 0 {
        bucket_uploader_builder = bucket_uploader_builder.thread_pool_size(thread_pool_size);
    }
    bucket_uploader_builder.build().into()
}

/// @brief 创建存储空间上传器实例
/// @param[in] upload_manager 上传管理器实例
/// @param[in] bucket_name 存储空间名称
/// @param[in] access_key 七牛 Access Key
/// @param[in] thread_pool_size 上传线程池大小，如果传入 `0`，则使用默认的线程池策略
/// @retval qiniu_ng_bucket_uploader_t 获取创建的存储空间上传器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_bucket_uploader_free()` 方法释放 `qiniu_ng_bucket_uploader_t`
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_new_from_bucket_name(
    upload_manager: qiniu_ng_upload_manager_t,
    bucket_name: *const qiniu_ng_char_t,
    access_key: *const qiniu_ng_char_t,
    thread_pool_size: size_t,
) -> qiniu_ng_bucket_uploader_t {
    let upload_manager = Option::<Box<UploadManager>>::from(upload_manager).unwrap();
    let mut bucket_uploader_builder = upload_manager
        .for_bucket_name(
            unsafe { ucstr::from_ptr(bucket_name) }.to_string().unwrap(),
            unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
        )
        .tap(|_| {
            let _ = qiniu_ng_upload_manager_t::from(upload_manager);
        });
    if thread_pool_size > 0 {
        bucket_uploader_builder = bucket_uploader_builder.thread_pool_size(thread_pool_size);
    }
    bucket_uploader_builder.build().into()
}

/// @brief 释放存储空间上传器实例
/// @param[in,out] bucket_uploader 存储空间上传器实例地址，释放完毕后该上传器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_free(bucket_uploader: *mut qiniu_ng_bucket_uploader_t) {
    if let Some(bucket_uploader) = unsafe { bucket_uploader.as_mut() } {
        let _ = Option::<BucketUploader>::from(*bucket_uploader);
        *bucket_uploader = qiniu_ng_bucket_uploader_t::default();
    }
}

/// @brief 判断存储空间上传器实例是否已经被释放
/// @param[in] bucket_uploader 存储空间上传器实例
/// @retval bool 如果返回 `true` 则表示存储空间上传器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_is_freed(bucket_uploader: qiniu_ng_bucket_uploader_t) -> bool {
    bucket_uploader.is_null()
}
