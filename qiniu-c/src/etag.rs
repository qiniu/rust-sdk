use super::{result::qiniu_ng_err, utils::make_path_buf};
use crypto::digest::Digest;
use libc::{c_char, c_void, size_t};
use qiniu_ng::utils::etag;
use std::{mem::transmute, slice};

pub const ETAG_SIZE: usize = 28;

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_from_file_path(
    path: *const c_char,
    path_len: size_t,
    result_ptr: *mut c_char,
    error: *mut qiniu_ng_err,
) -> bool {
    match etag::from_file(make_path_buf(path, path_len)) {
        Ok(etag_string) => {
            write_string_to_ptr(&etag_string, result_ptr);
            true
        }
        Err(err) => {
            if !error.is_null() {
                unsafe { *error = (&err).into() };
            }
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_from_buffer(buffer: *const c_char, buffer_len: size_t, result: *mut c_char) {
    write_string_to_ptr(
        unsafe { &etag::from_bytes(slice::from_raw_parts(buffer.cast(), buffer_len)) },
        result,
    );
}

#[repr(C)]
pub struct qiniu_ng_etag_t(*mut c_void);

impl From<qiniu_ng_etag_t> for Box<etag::Etag> {
    fn from(etag: qiniu_ng_etag_t) -> Self {
        unsafe { Box::from_raw(transmute::<_, *mut etag::Etag>(etag)) }
    }
}

impl From<Box<etag::Etag>> for qiniu_ng_etag_t {
    fn from(etag: Box<etag::Etag>) -> Self {
        unsafe { transmute(Box::into_raw(etag)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_new() -> qiniu_ng_etag_t {
    Box::new(etag::new()).into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_update(etag: qiniu_ng_etag_t, data: *mut c_char, data_len: size_t) {
    let mut etag: Box<etag::Etag> = etag.into();
    etag.input(unsafe { slice::from_raw_parts(data.cast(), data_len) });
    let _: qiniu_ng_etag_t = etag.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_result(etag: qiniu_ng_etag_t, result_ptr: *mut c_char) {
    let mut etag: Box<etag::Etag> = etag.into();
    etag.result(unsafe { slice::from_raw_parts_mut(result_ptr.cast(), ETAG_SIZE) });
    let _: qiniu_ng_etag_t = etag.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_reset(etag: qiniu_ng_etag_t) {
    let mut etag: Box<etag::Etag> = etag.into();
    etag.reset();
    let _: qiniu_ng_etag_t = etag.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_free(etag: qiniu_ng_etag_t) {
    let _: Box<etag::Etag> = etag.into();
}

fn write_string_to_ptr(src: &str, dst: *mut c_char) {
    unsafe {
        dst.copy_from_nonoverlapping(src.as_ptr().cast(), src.len());
    }
}
