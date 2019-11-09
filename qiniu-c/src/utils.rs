use cfg_if::cfg_if;
use libc::{c_char, c_void, size_t};
use std::{
    borrow::Cow,
    boxed::Box,
    ffi::{CStr, CString},
    mem::transmute,
    path::PathBuf,
    slice,
};
use tap::TapOps;

#[repr(C)]
pub struct qiniu_ng_string_t(*mut c_char);

impl From<CString> for qiniu_ng_string_t {
    fn from(s: CString) -> Self {
        unsafe { transmute(s.into_raw()) }
    }
}

impl From<qiniu_ng_string_t> for CString {
    fn from(s: qiniu_ng_string_t) -> Self {
        unsafe { CString::from_raw(transmute(s)) }
    }
}

pub(crate) fn make_string(s: &str) -> qiniu_ng_string_t {
    CString::new(s).unwrap().into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_get_ptr(s: qiniu_ng_string_t) -> *const c_char {
    s.0
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_free(s: qiniu_ng_string_t) {
    let _: CString = s.into();
}

#[repr(C)]
pub struct qiniu_ng_string_list_t(*mut c_void, *mut c_void);

impl From<Box<[CString]>> for qiniu_ng_string_list_t {
    fn from(strlist: Box<[CString]>) -> Self {
        unsafe { transmute(Box::into_raw(strlist)) }
    }
}

impl From<qiniu_ng_string_list_t> for Box<[CString]> {
    fn from(strlist: qiniu_ng_string_list_t) -> Self {
        unsafe { Box::from_raw(transmute(strlist)) }
    }
}

pub(crate) fn make_string_list(list: &[impl AsRef<str>]) -> qiniu_ng_string_list_t {
    list.iter()
        .map(|s| CString::new(s.as_ref()).unwrap())
        .collect::<Box<[CString]>>()
        .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_list_len(strlist: qiniu_ng_string_list_t) -> size_t {
    let strlist: Box<[CString]> = strlist.into();
    strlist.len().tap(|_| {
        let _: qiniu_ng_string_list_t = strlist.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_list_get(
    strlist: qiniu_ng_string_list_t,
    index: size_t,
    str_ptr: *mut *const c_char,
) -> bool {
    let strlist: Box<[CString]> = strlist.into();
    let mut got = false;
    if let Some(s) = strlist.get(index) {
        if !str_ptr.is_null() {
            unsafe { *str_ptr = s.as_ptr() };
        }
        got = true;
    }
    let _: qiniu_ng_string_list_t = strlist.into();
    got
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_list_free(strlist: qiniu_ng_string_list_t) {
    let _: Box<[CString]> = strlist.into();
}

cfg_if! {
    if #[cfg(unix)] {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        pub(crate) fn make_path_buf(path: *const c_char, path_len: size_t) -> PathBuf {
            let buf = unsafe { slice::from_raw_parts(path.cast(), path_len) };
            PathBuf::from(OsStr::from_bytes(buf))
        }
    } else if #[cfg(windows)] {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        pub(crate) fn make_path_buf(path: *const c_char, path_len: size_t) -> PathBuf {
            let buf = unsafe { slice::from_raw_parts(path.cast(), path_len) };
            PathBuf::from(OsStr::from_wide(buf))
        }
    } else {
        panic!("Unsupported platform");
    }
}

pub(crate) fn convert_c_char_pointer_to_boxed_cstr(p: *const c_char) -> Option<Box<CStr>> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(p) }.into())
    }
}

pub(crate) fn convert_c_char_to_string<'a>(p: *const c_char) -> Cow<'a, str> {
    unsafe { CStr::from_ptr(p) }.to_string_lossy()
}

pub(crate) fn convert_str_to_boxed_cstr(s: &str) -> Box<CStr> {
    let mut v = vec![0u8; s.len() + 1];
    v[..s.len()].copy_from_slice(s.as_bytes());
    unsafe { CString::from_vec_unchecked(v) }.into()
}

pub(crate) fn convert_string_to_boxed_cstr(s: String) -> Box<CStr> {
    let mut b = s.into_bytes();
    b.push(0);
    unsafe { CString::from_vec_unchecked(b) }.into()
}
