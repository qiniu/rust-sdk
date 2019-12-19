use crate::{
    bucket::qiniu_ng_bucket_t,
    config::qiniu_ng_config_t,
    result::qiniu_ng_err,
    upload_token::{qiniu_ng_upload_token_get_token, qiniu_ng_upload_token_t},
    utils::{
        convert_c_char_to_optional_string, convert_c_char_to_string, make_optional_string, make_path_buf, make_string,
        qiniu_ng_optional_string_t, qiniu_ng_string_map_t, qiniu_ng_string_t,
    },
};
use libc::{c_char, c_uint, c_ulonglong, c_void, ferror, fread, size_t, FILE};
use mime::Mime;
use qiniu_ng::storage::{
    bucket::Bucket,
    upload_token::UploadToken,
    uploader::{
        BucketUploader, FileUploaderBuilder, UploadManager, UploadResponse as QiniuUploadResponse,
        UploadResult as QiniuUploadResult,
    },
};
use std::{
    collections::{hash_map::RandomState, HashMap},
    ffi::CStr,
    io::{Error, ErrorKind, Read, Result},
    mem::{drop, transmute},
};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
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
            convert_c_char_to_string(bucket_name),
            convert_c_char_to_string(access_key),
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
pub enum qiniu_ng_resumable_policy_e {
    Default = 0,
    Threshold,
    AlwaysBeResumable,
    NeverBeResumable,
}

#[repr(C)]
#[derive(Clone)]
pub struct qiniu_ng_upload_params_t {
    key: *const c_char,
    file_name: *const c_char,
    mime: *const c_char,
    vars: *const qiniu_ng_string_map_t,
    metadata: *const qiniu_ng_string_map_t,
    checksum_enabled: bool,
    resumable_policy: qiniu_ng_resumable_policy_e,
    on_uploading_progress: Option<fn(uploaded: c_ulonglong, total: c_ulonglong)>,
    upload_threshold: c_uint,
    thread_pool_size: size_t,
    max_concurrency: size_t,
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_file_path(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
    file_path: *const c_char,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err,
) -> bool {
    qiniu_ng_upload(
        bucket_uploader,
        upload_token,
        UploadFile::FilePath(file_path),
        params,
        response,
        err,
    )
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_file(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
    file: *mut FILE,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err,
) -> bool {
    qiniu_ng_upload(
        bucket_uploader,
        upload_token,
        UploadFile::File(file),
        params,
        response,
        err,
    )
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_reader(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
    reader: qiniu_ng_readable_t,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err,
) -> bool {
    qiniu_ng_upload(
        bucket_uploader,
        upload_token,
        UploadFile::Readable(reader),
        params,
        response,
        err,
    )
}

fn qiniu_ng_upload(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
    upload_file: UploadFile,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err,
) -> bool {
    let bucket_uploader: BucketUploader = bucket_uploader.into();
    let upload_token = qiniu_ng_upload_token_get_token(upload_token);
    let mut file_uploader =
        bucket_uploader.upload_token(UploadToken::from_token(convert_c_char_to_string(upload_token)));
    let mut file_name: Option<String> = None;
    let mut mime: Option<Mime> = None;
    if let Some(params) = unsafe { params.as_ref() } {
        file_uploader = set_params_to_file_uploader(file_uploader, params);
        file_name = convert_c_char_to_optional_string(params.file_name).map(|file_name| file_name.into());
        mime = match convert_c_char_to_optional_string(params.mime).map(|mime| mime.parse()) {
            Some(Ok(mime)) => Some(mime),
            Some(Err(ref e)) => {
                if !err.is_null() {
                    unsafe { *err = e.into() };
                }
                drop(file_uploader);
                let _: qiniu_ng_bucket_uploader_t = bucket_uploader.into();
                return false;
            }
            _ => None,
        };
    }
    match upload_file.upload(file_uploader, file_name, mime) {
        Ok(resp) => {
            if !response.is_null() {
                unsafe { *response = Box::new(resp).into() };
            }
            true
        }
        Err(ref e) => {
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

fn set_params_to_file_uploader<'n>(
    mut file_uploader: FileUploaderBuilder<'n>,
    params: &qiniu_ng_upload_params_t,
) -> FileUploaderBuilder<'n> {
    if params.thread_pool_size > 0 {
        file_uploader = file_uploader.thread_pool_size(params.thread_pool_size);
    }
    if let Some(key) = convert_c_char_to_optional_string(params.key) {
        file_uploader = file_uploader.key(key);
    }
    if !params.vars.is_null() {
        let vars: Box<HashMap<Box<CStr>, Box<CStr>, RandomState>> = unsafe { *params.vars }.into();
        for (key, value) in vars.iter() {
            file_uploader = file_uploader.var(key.to_string_lossy().into_owned(), value.to_string_lossy().into_owned());
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
    match params.resumable_policy {
        qiniu_ng_resumable_policy_e::Threshold => {
            file_uploader = file_uploader.upload_threshold(params.upload_threshold);
        }
        qiniu_ng_resumable_policy_e::AlwaysBeResumable => {
            file_uploader = file_uploader.always_be_resumable();
        }
        qiniu_ng_resumable_policy_e::NeverBeResumable => {
            file_uploader = file_uploader.never_be_resumable();
        }
        _ => {}
    }
    if let Some(on_uploading_progress) = params.on_uploading_progress {
        file_uploader = file_uploader.on_progress(move |uploaded: u64, total: Option<u64>| {
            (on_uploading_progress)(uploaded, total.unwrap_or(0))
        });
    }
    file_uploader.max_concurrency(params.max_concurrency)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_upload_response_t(*mut c_void);

impl From<qiniu_ng_upload_response_t> for Box<QiniuUploadResponse> {
    fn from(upload_response: qiniu_ng_upload_response_t) -> Self {
        unsafe { Self::from_raw(transmute(upload_response)) }
    }
}

impl From<Box<QiniuUploadResponse>> for qiniu_ng_upload_response_t {
    fn from(upload_response: Box<QiniuUploadResponse>) -> Self {
        unsafe { transmute(Box::into_raw(upload_response)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_get_key(
    upload_response: qiniu_ng_upload_response_t,
) -> qiniu_ng_optional_string_t {
    let upload_response: Box<QiniuUploadResponse> = upload_response.into();
    make_optional_string(upload_response.key()).tap(|_| {
        let _: qiniu_ng_upload_response_t = upload_response.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_get_hash(
    upload_response: qiniu_ng_upload_response_t,
) -> qiniu_ng_optional_string_t {
    let upload_response: Box<QiniuUploadResponse> = upload_response.into();
    make_optional_string(upload_response.hash()).tap(|_| {
        let _: qiniu_ng_upload_response_t = upload_response.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_get_json(
    upload_response: qiniu_ng_upload_response_t,
    json: *mut qiniu_ng_string_t,
    err: *mut qiniu_ng_err,
) -> bool {
    let upload_response: Box<QiniuUploadResponse> = upload_response.into();
    match upload_response.to_string() {
        Ok(s) => {
            if !json.is_null() {
                unsafe { *json = make_string(s) };
            }
            true
        }
        Err(ref e) => {
            if !err.is_null() {
                unsafe { *err = e.into() };
            }
            false
        }
    }
    .tap(|_| {
        let _: qiniu_ng_upload_response_t = upload_response.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_free(upload_response: qiniu_ng_upload_response_t) {
    let _: Box<QiniuUploadResponse> = upload_response.into();
}

enum UploadFile {
    FilePath(*const c_char),
    File(*mut FILE),
    Readable(qiniu_ng_readable_t),
}

impl UploadFile {
    fn upload(
        self,
        file_uploader: FileUploaderBuilder,
        file_name: Option<String>,
        mime: Option<Mime>,
    ) -> QiniuUploadResult {
        match self {
            UploadFile::FilePath(file_path) => file_uploader.upload_file(make_path_buf(file_path), file_name, mime),
            UploadFile::File(file) => file_uploader.upload_stream(FileReader(file), file_name, mime),
            UploadFile::Readable(reader) => file_uploader.upload_stream(reader, file_name, mime),
        }
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct qiniu_ng_readable_t {
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

struct FileReader(*mut FILE);

impl Read for FileReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let have_read = unsafe { fread(buf.as_mut_ptr().cast(), 1, buf.len(), self.0) };
        if have_read < buf.len() && unsafe { ferror(self.0) } != 0 {
            return Err(Error::new(ErrorKind::Other, "ferror() returns non-zero"));
        }
        Ok(have_read)
    }
}
unsafe impl Send for FileReader {}
