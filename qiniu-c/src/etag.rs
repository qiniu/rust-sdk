use super::{
    result::qiniu_ng_err,
    string::{qiniu_ng_char_t, UCString},
};
use digest::{FixedOutput, Input, Reset};
use libc::{c_char, c_void, size_t};
use qiniu_ng::utils::etag;
use std::{
    mem::{replace, transmute},
    ptr::copy_nonoverlapping,
    slice::from_raw_parts,
};

pub const ETAG_SIZE: usize = 28;

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_from_file_path(
    path: *const qiniu_ng_char_t,
    result_ptr: *mut c_char,
    error: *mut qiniu_ng_err,
) -> bool {
    match etag::from_file(unsafe { UCString::from_ptr(path) }.into_path_buf()) {
        Ok(etag_string) => {
            unsafe { write_string_to_ptr(&etag_string, result_ptr) };
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
pub extern "C" fn qiniu_ng_etag_from_buffer(buffer: *const c_void, buffer_len: size_t, result: *mut c_char) {
    unsafe {
        write_string_to_ptr(&etag::from_bytes(from_raw_parts(buffer.cast(), buffer_len)), result);
    }
}

unsafe fn write_string_to_ptr(src: &str, dst: *mut c_char) {
    dst.copy_from_nonoverlapping(src.as_ptr().cast(), src.len());
}

#[repr(C)]
#[derive(Copy, Clone)]
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
pub extern "C" fn qiniu_ng_etag_update(etag: qiniu_ng_etag_t, data: *mut c_void, data_len: size_t) {
    let mut etag: Box<etag::Etag> = etag.into();
    etag.input(unsafe { from_raw_parts(data.cast(), data_len) });
    let _: qiniu_ng_etag_t = etag.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_result(etag: qiniu_ng_etag_t, result_ptr: *mut c_void) {
    let mut etag: Box<etag::Etag> = etag.into();
    let result = replace(&mut *etag, etag::new()).fixed_result();
    unsafe { copy_nonoverlapping(result.as_ptr(), result_ptr.cast(), ETAG_SIZE) };
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
