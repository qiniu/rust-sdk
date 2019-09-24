use super::{
    result::{make_qiniu_ng_err_from_io_error, qiniu_ng_err},
    utils::{make_path_buf, write_string_to_ptr},
};
use crypto::digest::Digest;
use libc::{c_char, c_void, size_t};
use qiniu::utils::etag;
use std::{mem, slice};

pub const ETAG_SIZE: usize = 28;

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_etag_from_file_path(
    path: *const c_char,
    path_len: size_t,
    result_ptr: *mut c_char,
    error: *mut qiniu_ng_err,
) -> bool {
    match etag::from_file(make_path_buf(mem::transmute(path), path_len)) {
        Ok(etag_string) => {
            write_string_to_ptr(etag_string, result_ptr);
            true
        }
        Err(err) => {
            if !error.is_null() {
                *error = make_qiniu_ng_err_from_io_error(&err);
            }
            false
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_etag_from_buffer(buffer: *const c_char, buffer_len: size_t, result_ptr: *mut c_char) {
    let etag_string = etag::from_bytes(slice::from_raw_parts(mem::transmute(buffer), buffer_len));
    write_string_to_ptr(etag_string, result_ptr);
}

#[repr(C)]
pub struct qiniu_ng_etag_t(*mut c_void);

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_etag_new() -> qiniu_ng_etag_t {
    let etag = Box::new(etag::new());
    mem::transmute(Box::into_raw(etag))
}

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_etag_update(etag: qiniu_ng_etag_t, data: *mut c_void, data_len: size_t) {
    let mut boxed_etag = Box::from_raw(mem::transmute::<_, *mut etag::Etag>(etag));
    boxed_etag.input(slice::from_raw_parts(mem::transmute(data), data_len));
    Box::into_raw(boxed_etag);
}

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_etag_result(etag: qiniu_ng_etag_t, result_ptr: *mut c_char) {
    let mut boxed_etag = Box::from_raw(mem::transmute::<_, *mut etag::Etag>(etag));
    boxed_etag.result(slice::from_raw_parts_mut(mem::transmute(result_ptr), ETAG_SIZE));
    Box::into_raw(boxed_etag);
}

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_etag_reset(etag: qiniu_ng_etag_t) {
    let mut boxed_etag = Box::from_raw(mem::transmute::<_, *mut etag::Etag>(etag));
    boxed_etag.reset();
    Box::into_raw(boxed_etag);
}

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_etag_free(etag: qiniu_ng_etag_t) {
    Box::from_raw(mem::transmute::<_, *mut etag::Etag>(etag));
}
