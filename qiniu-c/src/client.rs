use crate::config::qiniu_ng_config_t;
use libc::c_void;
use qiniu::{Client, Config};
use std::{ffi::CStr, os::raw::c_char};

#[repr(C)]
pub struct qiniu_ng_client_t(pub(crate) *mut c_void);

#[no_mangle]
pub extern "C" fn qiniu_ng_client_new(
    access_key: *const c_char,
    secret_key: *const c_char,
    config: qiniu_ng_config_t,
) -> qiniu_ng_client_t {
    let client = Box::new(Client::new(
        unsafe { CStr::from_ptr(access_key) }.to_string_lossy(),
        unsafe { CStr::from_ptr(secret_key) }.to_string_lossy(),
        unsafe { *Box::from_raw(config.0 as usize as *mut Config) },
    ));
    qiniu_ng_client_t(Box::into_raw(client) as usize as *mut c_void)
}

#[no_mangle]
pub extern "C" fn qiniu_ng_client_free(client: qiniu_ng_client_t) {
    unsafe { Box::from_raw(client.0 as usize as *mut Client) };
}
