use crate::config::qiniu_ng_config_t;
use libc::c_void;
use qiniu_ng::storage::uploader::UploadManager;
use std::mem::transmute;

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
