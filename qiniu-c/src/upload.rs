use crate::{
    bucket::qiniu_ng_bucket_t,
    config::qiniu_ng_config_t,
    result::qiniu_ng_err,
    upload_token::{qiniu_ng_upload_token_get_token, qiniu_ng_upload_token_t},
    utils::{
        convert_c_char_to_optional_string, convert_c_char_to_string, convert_str_to_boxed_cstr, make_path_buf,
        qiniu_ng_string_map_t,
    },
};
use libc::{c_char, c_uint, c_void, size_t};
use mime::Mime;
use qiniu_ng::storage::{
    bucket::Bucket,
    upload_token::UploadToken,
    uploader::{BucketUploader, UploadManager, UploadResponse as QiniuUploadResponse},
};
use std::{
    collections::{hash_map::RandomState, HashMap},
    ffi::CStr,
    mem::transmute,
    ptr::null,
};
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
pub extern "C" fn qiniu_ng_upload_manager_new(config: qiniu_ng_config_t) -> qiniu_ng_upload_manager_t {
    Box::new(UploadManager::new(config.get_clone())).into()
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

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_free(bucket_uploader: qiniu_ng_bucket_uploader_t) {
    let _: BucketUploader = bucket_uploader.into();
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum qiniu_ng_resumeable_policy_e {
    Default = 0,
    Threshold,
    AlwaysBeResumeable,
    NeverBeResumeable,
}

#[repr(C)]
pub struct qiniu_ng_upload_params_t {
    key: *const c_char,
    vars: *const qiniu_ng_string_map_t,
    metadata: *const qiniu_ng_string_map_t,
    checksum_enabled: bool,
    resumeable_policy: qiniu_ng_resumeable_policy_e,
    upload_threshold: c_uint,
    thread_pool_size: size_t,
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_file(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
    file_path: *const c_char,
    file_name: *const c_char,
    mime: *const c_char,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err,
) -> bool {
    let bucket_uploader: BucketUploader = bucket_uploader.into();
    let upload_token = qiniu_ng_upload_token_get_token(upload_token);
    let mut file_uploader =
        bucket_uploader.upload_token(UploadToken::from_token(convert_c_char_to_string(upload_token)));
    if let Some(params) = unsafe { params.as_ref() } {
        if params.thread_pool_size > 0 {
            file_uploader = file_uploader.thread_pool_size(params.thread_pool_size);
        }
        if let Some(key) = convert_c_char_to_optional_string(params.key) {
            file_uploader = file_uploader.key(key);
        }
        if !params.vars.is_null() {
            let vars: Box<HashMap<Box<CStr>, Box<CStr>, RandomState>> = unsafe { *params.vars }.into();
            for (key, value) in vars.iter() {
                file_uploader =
                    file_uploader.var(key.to_string_lossy().into_owned(), value.to_string_lossy().into_owned());
            }
            let _: qiniu_ng_string_map_t = vars.into();
        }
        if !params.metadata.is_null() {
            let metadata: Box<HashMap<Box<CStr>, Box<CStr>, RandomState>> = unsafe { *params.metadata }.into();
            for (key, value) in metadata.iter() {
                file_uploader =
                    file_uploader.metadata(key.to_string_lossy().into_owned(), value.to_string_lossy().into_owned());
            }
            let _: qiniu_ng_string_map_t = metadata.into();
        }
        file_uploader = if params.checksum_enabled {
            file_uploader.enable_checksum()
        } else {
            file_uploader.disable_checksum()
        };
        match params.resumeable_policy {
            qiniu_ng_resumeable_policy_e::Threshold => {
                file_uploader = file_uploader.upload_threshold(params.upload_threshold);
            }
            qiniu_ng_resumeable_policy_e::AlwaysBeResumeable => {
                file_uploader = file_uploader.always_be_resumeable();
            }
            qiniu_ng_resumeable_policy_e::NeverBeResumeable => {
                file_uploader = file_uploader.never_be_resumeable();
            }
            _ => {}
        }
    }
    let mime: Option<Mime> = match convert_c_char_to_optional_string(mime).map(|mime| mime.parse()) {
        Some(Ok(mime)) => Some(mime),
        Some(Err(ref e)) => {
            if !err.is_null() {
                unsafe { *err = e.into() };
            }
            let _: qiniu_ng_bucket_uploader_t = bucket_uploader.into();
            return false;
        }
        _ => None,
    };
    match &file_uploader.upload_file(
        make_path_buf(file_path),
        convert_c_char_to_optional_string(file_name),
        mime,
    ) {
        Ok(resp) => {
            if !response.is_null() {
                let resp: Box<UploadResponse> = Box::new(resp.into());
                unsafe { *response = resp.into() };
            }
            true
        }
        Err(e) => {
            if !err.is_null() {
                unsafe { *err = e.into() };
            }
            false
        }
    }
    .tap(|_| {
        let _: qiniu_ng_bucket_uploader_t = bucket_uploader.into();
    })
}

struct UploadResponse {
    key: Option<Box<CStr>>,
    hash: Option<Box<CStr>>,
}

impl From<&QiniuUploadResponse> for UploadResponse {
    fn from(upload_response: &QiniuUploadResponse) -> Self {
        UploadResponse {
            key: upload_response.key().map(convert_str_to_boxed_cstr),
            hash: upload_response.hash().map(convert_str_to_boxed_cstr),
        }
    }
}

#[repr(C)]
pub struct qiniu_ng_upload_response_t(*mut c_void);

impl From<qiniu_ng_upload_response_t> for Box<UploadResponse> {
    fn from(upload_response: qiniu_ng_upload_response_t) -> Self {
        unsafe { Self::from_raw(transmute(upload_response)) }
    }
}

impl From<Box<UploadResponse>> for qiniu_ng_upload_response_t {
    fn from(upload_response: Box<UploadResponse>) -> Self {
        unsafe { transmute(Box::into_raw(upload_response)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_get_key(upload_response: qiniu_ng_upload_response_t) -> *const c_char {
    let upload_response: Box<UploadResponse> = upload_response.into();
    upload_response
        .key
        .as_ref()
        .map(|key| key.as_ptr())
        .unwrap_or_else(null)
        .tap(|_| {
            let _: qiniu_ng_upload_response_t = upload_response.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_get_hash(upload_response: qiniu_ng_upload_response_t) -> *const c_char {
    let upload_response: Box<UploadResponse> = upload_response.into();
    upload_response
        .hash
        .as_ref()
        .map(|hash| hash.as_ptr())
        .unwrap_or_else(null)
        .tap(|_| {
            let _: qiniu_ng_upload_response_t = upload_response.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_free(upload_response: qiniu_ng_upload_response_t) {
    let _: Box<UploadResponse> = upload_response.into();
}
