use libc::c_void;
use qiniu::{Client, Config};
use std::{ffi::CStr, os::raw::c_char};

#[no_mangle]
pub extern "C" fn qiniu_ng_client_new(
    access_key: *const c_char,
    secret_key: *const c_char,
    config_ptr: *mut c_void,
) -> *mut c_void {
    let client = Box::new(Client::new(
        unsafe { CStr::from_ptr(access_key) }.to_string_lossy(),
        unsafe { CStr::from_ptr(secret_key) }.to_string_lossy(),
        unsafe { *Box::from_raw(config_ptr as usize as *mut Config) },
    ));
    Box::into_raw(client) as usize as *mut c_void
}

#[no_mangle]
pub extern "C" fn qiniu_ng_client_free(client_ptr: *mut c_void) {
    unsafe { Box::from_raw(client_ptr as usize as *mut Client) };
}
