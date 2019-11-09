use crate::{
    bucket::qiniu_ng_bucket_t, config::qiniu_ng_config_t, result::qiniu_ng_err, utils::convert_c_char_to_string,
};
use libc::{c_char, c_void, size_t};
use qiniu_ng::storage::{
    bucket::Bucket,
    uploader::{BucketUploader, UploadManager},
};
use std::{ffi::CStr, mem::transmute};
use tap::TapOps;

#[repr(C)]
pub struct qiniu_ng_upload_manager_t(*mut c_void);

impl From<qiniu_ng_upload_manager_t> for Box<UploadManager> {
    fn from(upload_manager: qiniu_ng_upload_manager_t) -> Self {
        unsafe { Box::from_raw(transmute::<_, *mut UploadManager>(upload_manager)) }
    }
}

impl From<Box<UploadManager>> for qiniu_ng_upload_manager_t {
    fn from(upload_manager: Box<UploadManager>) -> Self {
        unsafe { transmute(Box::into_raw(upload_manager)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_new(config: *const qiniu_ng_config_t) -> qiniu_ng_upload_manager_t {
    Box::new(UploadManager::new(unsafe { config.as_ref() }.unwrap().into())).into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_free(upload_manager: qiniu_ng_upload_manager_t) {
    let _: Box<UploadManager> = upload_manager.into();
}

#[repr(C)]
pub struct qiniu_ng_bucket_uploader_t(*mut c_void);

impl From<qiniu_ng_bucket_uploader_t> for BucketUploader {
    fn from(bucket_uploader: qiniu_ng_bucket_uploader_t) -> Self {
        unsafe { Self::from_raw(transmute(bucket_uploader)) }
    }
}

impl From<BucketUploader> for qiniu_ng_bucket_uploader_t {
    fn from(bucket_uploader: BucketUploader) -> Self {
        unsafe { transmute(bucket_uploader.into_raw()) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_new_bucket_uploader_from_bucket(
    upload_manager: qiniu_ng_upload_manager_t,
    bucket: qiniu_ng_bucket_t,
    thread_pool_size: size_t,
) -> qiniu_ng_bucket_uploader_t {
    let upload_manager: Box<UploadManager> = upload_manager.into();
    let bucket: Box<Bucket> = bucket.into();
    let mut bucket_uploader_builder = upload_manager.for_bucket(&bucket).tap(|_| {
        let _: qiniu_ng_upload_manager_t = upload_manager.into();
        let _: qiniu_ng_bucket_t = bucket.into();
    });
    if thread_pool_size > 0 {
        bucket_uploader_builder = bucket_uploader_builder.thread_pool_size(thread_pool_size);
    }
    bucket_uploader_builder.build().into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_new_bucket_uploader_from_bucket_name(
    upload_manager: qiniu_ng_upload_manager_t,
    bucket_name: *const c_char,
    access_key: *const c_char,
    thread_pool_size: size_t,
) -> qiniu_ng_bucket_uploader_t {
    let upload_manager: Box<UploadManager> = upload_manager.into();
    let mut bucket_uploader_builder = upload_manager
        .for_bucket_name(
            convert_c_char_to_string(bucket_name.cast()),
            convert_c_char_to_string(access_key.cast()),
        )
        .tap(|_| {
            let _: qiniu_ng_upload_manager_t = upload_manager.into();
        });
    if thread_pool_size > 0 {
        bucket_uploader_builder = bucket_uploader_builder.thread_pool_size(thread_pool_size);
    }
    bucket_uploader_builder.build().into()
}
