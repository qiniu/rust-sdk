use super::{
    result::{make_qiniu_ng_result_from_io_error, qiniu_ng_result, QINIU_NG_RESULT_OK},
    utils::{make_path_buf, write_string_to_ptr},
};
use crypto::digest::Digest;
use libc::{c_char, c_void, size_t};
use qiniu::utils::etag;
use std::slice;

pub const ETAG_SIZE: usize = 28;

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_from_file_path(
    path: *const c_char,
    path_len: size_t,
    result_ptr: *mut c_char,
) -> qiniu_ng_result {
    match etag::from_file(make_path_buf(path as *const u8, path_len)) {
        Ok(etag_string) => {
            write_string_to_ptr(etag_string, result_ptr);
            QINIU_NG_RESULT_OK
        }
        Err(err) => make_qiniu_ng_result_from_io_error(err),
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_from_buffer(
    buffer: *const c_char,
    buffer_len: size_t,
    result_ptr: *mut c_char,
) -> qiniu_ng_result {
    let etag_string = etag::from_bytes(unsafe { slice::from_raw_parts(buffer as *const u8, buffer_len) });
    write_string_to_ptr(etag_string, result_ptr);
    QINIU_NG_RESULT_OK
}

#[repr(C)]
pub struct qiniu_ng_etag_t(pub(crate) *mut c_void);

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_new() -> qiniu_ng_etag_t {
    let etag = Box::new(etag::new());
    qiniu_ng_etag_t(Box::into_raw(etag) as usize as *mut c_void)
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_update(etag: qiniu_ng_etag_t, data: *mut c_void, data_len: size_t) {
    let mut boxed_etag = unsafe { Box::from_raw(etag.0 as usize as *mut etag::Etag) };
    boxed_etag.input(unsafe { slice::from_raw_parts(data as *const u8, data_len) });
    Box::into_raw(boxed_etag);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_result(etag: qiniu_ng_etag_t, result_ptr: *mut c_char) {
    let mut boxed_etag = unsafe { Box::from_raw(etag.0 as usize as *mut etag::Etag) };
    boxed_etag.result(unsafe { slice::from_raw_parts_mut(result_ptr as *mut u8, ETAG_SIZE) });
    Box::into_raw(boxed_etag);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_reset(etag: qiniu_ng_etag_t) {
    let mut boxed_etag = unsafe { Box::from_raw(etag.0 as usize as *mut etag::Etag) };
    boxed_etag.reset();
    Box::into_raw(boxed_etag);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_free(etag: qiniu_ng_etag_t) {
    unsafe {
        Box::from_raw(etag.0 as usize as *mut etag::Etag);
    }
}
