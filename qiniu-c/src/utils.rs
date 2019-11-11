use cfg_if::cfg_if;
use libc::{c_char, c_void, size_t, strlen};
use std::{
    borrow::Cow,
    boxed::Box,
    collections::{hash_map::RandomState, HashMap},
    ffi::{CStr, CString},
    mem::transmute,
    path::PathBuf,
    ptr::null,
    slice,
};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_string_t(*mut c_char, *mut c_char);

impl From<Box<CStr>> for qiniu_ng_string_t {
    fn from(s: Box<CStr>) -> Self {
        unsafe { transmute(Box::into_raw(s)) }
    }
}

impl From<qiniu_ng_string_t> for Box<CStr> {
    fn from(s: qiniu_ng_string_t) -> Self {
        unsafe { Box::from_raw(transmute(s)) }
    }
}

pub(crate) fn make_string(s: &str) -> qiniu_ng_string_t {
    CString::new(s).unwrap().into_boxed_c_str().into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_get_ptr(s: qiniu_ng_string_t) -> *const c_char {
    s.0
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_free(s: qiniu_ng_string_t) {
    let _: Box<CStr> = s.into();
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_string_list_t(*mut c_void, *mut c_void);

impl From<Box<[Box<CStr>]>> for qiniu_ng_string_list_t {
    fn from(strlist: Box<[Box<CStr>]>) -> Self {
        unsafe { transmute(Box::into_raw(strlist)) }
    }
}

impl From<qiniu_ng_string_list_t> for Box<[Box<CStr>]> {
    fn from(strlist: qiniu_ng_string_list_t) -> Self {
        unsafe { Box::from_raw(transmute(strlist)) }
    }
}

pub(crate) fn make_string_list(list: &[impl AsRef<str>]) -> qiniu_ng_string_list_t {
    list.iter()
        .map(|s| CString::new(s.as_ref()).unwrap().into_boxed_c_str())
        .collect::<Box<[Box<CStr>]>>()
        .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_list_len(strlist: qiniu_ng_string_list_t) -> size_t {
    let strlist: Box<[Box<CStr>]> = strlist.into();
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
    let strlist: Box<[Box<CStr>]> = strlist.into();
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
    let _: Box<[Box<CStr>]> = strlist.into();
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_string_map_t(*mut c_void);

impl From<Box<HashMap<Box<CStr>, Box<CStr>, RandomState>>> for qiniu_ng_string_map_t {
    fn from(hashmap: Box<HashMap<Box<CStr>, Box<CStr>, RandomState>>) -> Self {
        unsafe { transmute(Box::into_raw(hashmap)) }
    }
}

impl From<qiniu_ng_string_map_t> for Box<HashMap<Box<CStr>, Box<CStr>, RandomState>> {
    fn from(hashmap: qiniu_ng_string_map_t) -> Self {
        unsafe { Box::from_raw(transmute(hashmap)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_map_set(hashmap: qiniu_ng_string_map_t, key: *const c_char, value: *const c_char) {
    let mut hashmap: Box<HashMap<Box<CStr>, Box<CStr>, RandomState>> = hashmap.into();
    hashmap.insert(
        unsafe { CStr::from_ptr(key) }.into(),
        unsafe { CStr::from_ptr(value) }.into(),
    );
    let _: qiniu_ng_string_map_t = hashmap.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_map_get(hashmap: qiniu_ng_string_map_t, key: *const c_char) -> *const c_char {
    let hashmap: Box<HashMap<Box<CStr>, Box<CStr>, RandomState>> = hashmap.into();
    hashmap
        .get(unsafe { CStr::from_ptr(key) })
        .map(|val| val.as_ptr())
        .unwrap_or_else(null)
        .tap(|_| {
            let _: qiniu_ng_string_map_t = hashmap.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_map_free(hashmap: qiniu_ng_string_map_t) {
    let _: Box<HashMap<Box<CStr>, Box<CStr>, RandomState>> = hashmap.into();
}

cfg_if! {
    if #[cfg(unix)] {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        pub(crate) fn make_path_buf(path: *const c_char) -> PathBuf {
            let buf = unsafe { slice::from_raw_parts(path.cast(), strlen(path)) };
            PathBuf::from(OsStr::from_bytes(buf))
        }
    } else if #[cfg(windows)] {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        pub(crate) fn make_path_buf(path: *const c_char) -> PathBuf {
            let buf = unsafe { slice::from_raw_parts(path.cast(), strlen(path)) };
            PathBuf::from(OsStr::from_wide(buf))
        }
    } else {
        panic!("Unsupported platform");
    }
}

pub(crate) fn convert_c_char_pointer_to_boxed_cstr(p: *const c_char) -> Box<CStr> {
    unsafe { CStr::from_ptr(p) }.into()
}

pub(crate) fn convert_c_char_pointer_to_optional_boxed_cstr(p: *const c_char) -> Option<Box<CStr>> {
    if p.is_null() {
        None
    } else {
        Some(convert_c_char_pointer_to_boxed_cstr(p))
    }
}

pub(crate) fn convert_c_char_to_string<'a>(p: *const c_char) -> Cow<'a, str> {
    unsafe { CStr::from_ptr(p) }.to_string_lossy()
}

pub(crate) fn convert_c_char_to_optional_string<'a>(p: *const c_char) -> Option<Cow<'a, str>> {
    if p.is_null() {
        None
    } else {
        Some(convert_c_char_to_string(p))
    }
}

pub(crate) fn convert_str_to_boxed_cstr(s: impl Into<Vec<u8>>) -> Box<CStr> {
    CString::new(s).unwrap().into_boxed_c_str()
}
