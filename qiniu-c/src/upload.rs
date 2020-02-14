use crate::{
    bucket::qiniu_ng_bucket_t,
    config::qiniu_ng_config_t,
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr, UCString},
    upload_token::qiniu_ng_upload_token_t,
    utils::{qiniu_ng_readable_t, qiniu_ng_str_map_t, qiniu_ng_str_t},
};
use libc::{c_void, ferror, fread, size_t, FILE};
use mime::Mime;
use qiniu_ng::storage::{
    bucket::Bucket,
    uploader::{
        BucketUploader, FileUploaderBuilder, UploadManager, UploadResponse as QiniuUploadResponse,
        UploadResult as QiniuUploadResult, UploadToken,
    },
};
use std::{
    collections::{hash_map::RandomState, HashMap},
    io::{Error, ErrorKind, Read, Result},
    mem::{drop, transmute},
    ptr::{copy_nonoverlapping, null_mut},
};
use tap::TapOps;

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

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_new(config: qiniu_ng_config_t) -> qiniu_ng_upload_manager_t {
    Box::new(UploadManager::new(config.get_clone().unwrap())).into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_free(upload_manager: *mut qiniu_ng_upload_manager_t) {
    if let Some(upload_manager) = unsafe { upload_manager.as_mut() } {
        let _ = Option::<Box<UploadManager>>::from(*upload_manager);
        *upload_manager = qiniu_ng_upload_manager_t::default();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_is_freed(upload_manager: qiniu_ng_upload_manager_t) -> bool {
    upload_manager.is_null()
}

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

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_new_bucket_uploader_from_bucket(
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

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_manager_new_bucket_uploader_from_bucket_name(
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

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_free(bucket_uploader: *mut qiniu_ng_bucket_uploader_t) {
    if let Some(bucket_uploader) = unsafe { bucket_uploader.as_mut() } {
        let _ = Option::<BucketUploader>::from(*bucket_uploader);
        *bucket_uploader = qiniu_ng_bucket_uploader_t::default();
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[allow(dead_code, non_camel_case_types)]
pub enum qiniu_ng_resumable_policy_e {
    qiniu_ng_resumable_policy_default = 0,
    qiniu_ng_resumable_policy_threshold,
    qiniu_ng_resumable_policy_always_be_resumeable,
    qiniu_ng_resumable_policy_never_be_resumeable,
}

#[repr(C)]
#[derive(Clone)]
pub struct qiniu_ng_upload_params_t {
    key: *const qiniu_ng_char_t,
    file_name: *const qiniu_ng_char_t,
    mime: *const qiniu_ng_char_t,
    vars: qiniu_ng_str_map_t,
    metadata: qiniu_ng_str_map_t,
    checksum_enabled: bool,
    resumable_policy: qiniu_ng_resumable_policy_e,
    on_uploading_progress: Option<fn(uploaded: u64, total: u64)>,
    upload_threshold: u32,
    thread_pool_size: size_t,
    max_concurrency: size_t,
}

#[no_mangle]
pub extern "C" fn qiniu_ng_bucket_uploader_upload_file_path(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
    file_path: *const qiniu_ng_char_t,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
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
pub extern "C" fn qiniu_ng_bucket_uploader_upload_file(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
    file: *mut FILE,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
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
pub extern "C" fn qiniu_ng_bucket_uploader_upload_reader(
    bucket_uploader: qiniu_ng_bucket_uploader_t,
    upload_token: qiniu_ng_upload_token_t,
    reader: qiniu_ng_readable_t,
    params: *const qiniu_ng_upload_params_t,
    response: *mut qiniu_ng_upload_response_t,
    err: *mut qiniu_ng_err_t,
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
    err: *mut qiniu_ng_err_t,
) -> bool {
    let bucket_uploader = Option::<BucketUploader>::from(bucket_uploader).unwrap();
    let upload_token = Option::<Box<UploadToken>>::from(upload_token).unwrap();
    let mut file_uploader = bucket_uploader.upload_token(upload_token.as_ref().to_owned()).tap(|_| {
        let _ = qiniu_ng_upload_token_t::from(upload_token);
    });
    let mut file_name: String = "".into();
    let mut mime: Option<Mime> = None;
    if let Some(params) = unsafe { params.as_ref() } {
        file_uploader = set_params_to_file_uploader(file_uploader, params);
        if let Some(f) = unsafe { params.file_name.as_ref() } {
            file_name = unsafe { UCString::from_ptr(f) }.to_string().unwrap()
        }

        mime = match unsafe { params.mime.as_ref() }
            .map(|mime| unsafe { ucstr::from_ptr(mime) }.to_string().unwrap().parse())
        {
            Some(Ok(mime)) => Some(mime),
            Some(Err(ref e)) => {
                if let Some(err) = unsafe { err.as_mut() } {
                    *err = e.into();
                }
                drop(file_uploader);
                let _ = qiniu_ng_bucket_uploader_t::from(bucket_uploader);
                return false;
            }
            _ => None,
        };
    }
    match upload_file.upload(file_uploader, file_name, mime) {
        Ok(resp) => {
            if let Some(response) = unsafe { response.as_mut() } {
                *response = Box::new(resp).into();
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
        let _ = qiniu_ng_bucket_uploader_t::from(bucket_uploader);
    })
}

fn set_params_to_file_uploader<'n>(
    mut file_uploader: FileUploaderBuilder<'n>,
    params: &qiniu_ng_upload_params_t,
) -> FileUploaderBuilder<'n> {
    if params.thread_pool_size > 0 {
        file_uploader = file_uploader.thread_pool_size(params.thread_pool_size);
    }
    if let Some(key) = unsafe { params.key.as_ref() }.map(|key| unsafe { UCString::from_ptr(key) }.to_string().unwrap())
    {
        file_uploader = file_uploader.key(key);
    }
    {
        let vars = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(params.vars);
        if let Some(vars) = vars.as_ref() {
            for (key, value) in vars.iter() {
                file_uploader = file_uploader.var(key.to_string().unwrap(), value.to_string().unwrap());
            }
        }
        let _ = qiniu_ng_str_map_t::from(vars);
    }
    {
        let metadata = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(params.metadata);
        if let Some(metadata) = metadata.as_ref() {
            for (key, value) in metadata.iter() {
                file_uploader = file_uploader.metadata(key.to_string().unwrap(), value.to_string().unwrap());
            }
        }
        let _ = qiniu_ng_str_map_t::from(metadata);
    }
    file_uploader = if params.checksum_enabled {
        file_uploader.enable_checksum()
    } else {
        file_uploader.disable_checksum()
    };
    match params.resumable_policy {
        qiniu_ng_resumable_policy_e::qiniu_ng_resumable_policy_threshold => {
            file_uploader = file_uploader.upload_threshold(params.upload_threshold);
        }
        qiniu_ng_resumable_policy_e::qiniu_ng_resumable_policy_always_be_resumeable => {
            file_uploader = file_uploader.always_be_resumable();
        }
        qiniu_ng_resumable_policy_e::qiniu_ng_resumable_policy_never_be_resumeable => {
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

impl Default for qiniu_ng_upload_response_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_upload_response_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_upload_response_t> for Option<Box<QiniuUploadResponse>> {
    fn from(upload_response: qiniu_ng_upload_response_t) -> Self {
        if upload_response.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(upload_response)) })
        }
    }
}

impl From<Option<Box<QiniuUploadResponse>>> for qiniu_ng_upload_response_t {
    fn from(upload_response: Option<Box<QiniuUploadResponse>>) -> Self {
        upload_response
            .map(|upload_response| upload_response.into())
            .unwrap_or_default()
    }
}

impl From<Box<QiniuUploadResponse>> for qiniu_ng_upload_response_t {
    fn from(upload_response: Box<QiniuUploadResponse>) -> Self {
        unsafe { transmute(Box::into_raw(upload_response)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_get_key(upload_response: qiniu_ng_upload_response_t) -> qiniu_ng_str_t {
    let upload_response = Option::<Box<QiniuUploadResponse>>::from(upload_response).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_response.key()) }.tap(|_| {
        let _ = qiniu_ng_upload_response_t::from(upload_response);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_get_hash(
    upload_response: qiniu_ng_upload_response_t,
    result_ptr: *mut c_void,
    result_size: *mut size_t,
) {
    let upload_response = Option::<Box<QiniuUploadResponse>>::from(upload_response).unwrap();
    if let Some(hash) = upload_response.hash().map(|hash| hash.as_bytes()) {
        if let Some(result_size) = unsafe { result_size.as_mut() } {
            *result_size = hash.len();
        }
        if let Some(result_ptr) = unsafe { result_ptr.as_mut() } {
            unsafe { copy_nonoverlapping(hash.as_ptr(), result_ptr as *mut c_void as *mut u8, hash.len()) };
        }
    } else if let Some(result_size) = unsafe { result_size.as_mut() } {
        *result_size = 0;
    }
    let _ = qiniu_ng_upload_response_t::from(upload_response);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_get_json_string(
    upload_response: qiniu_ng_upload_response_t,
    json_str: *mut qiniu_ng_str_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let upload_response = Option::<Box<QiniuUploadResponse>>::from(upload_response).unwrap();
    match upload_response.to_string() {
        Ok(s) => {
            if let Some(json_str) = unsafe { json_str.as_mut() } {
                *json_str = unsafe { qiniu_ng_str_t::from_string_unchecked(s) };
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
        let _ = qiniu_ng_upload_response_t::from(upload_response);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_response_free(upload_response: *mut qiniu_ng_upload_response_t) {
    if let Some(upload_response) = unsafe { upload_response.as_mut() } {
        let _ = Option::<Box<QiniuUploadResponse>>::from(*upload_response);
        *upload_response = qiniu_ng_upload_response_t::default();
    }
}

enum UploadFile {
    FilePath(*const qiniu_ng_char_t),
    File(*mut FILE),
    Readable(qiniu_ng_readable_t),
}

impl UploadFile {
    fn upload(self, file_uploader: FileUploaderBuilder, file_name: String, mime: Option<Mime>) -> QiniuUploadResult {
        match self {
            UploadFile::FilePath(file_path) => {
                file_uploader.upload_file(unsafe { UCString::from_ptr(file_path) }.to_path_buf(), file_name, mime)
            }
            UploadFile::File(file) => file_uploader.upload_stream(FileReader(file), file_name, mime),
            UploadFile::Readable(reader) => file_uploader.upload_stream(reader, file_name, mime),
        }
    }
}

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
