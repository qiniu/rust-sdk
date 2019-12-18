use cfg_if::cfg_if;
use libc::{c_char, c_void, size_t, strlen};
use std::{
    borrow::Cow,
    boxed::Box,
    collections::{hash_map::RandomState, HashMap},
    ffi::{CStr, CString},
    mem::transmute,
    path::PathBuf,
    ptr::{copy_nonoverlapping, null, null_mut},
    slice::from_raw_parts,
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

pub(crate) fn make_string(s: impl Into<Vec<u8>>) -> qiniu_ng_string_t {
    CString::new(s).unwrap().into_boxed_c_str().into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_new(s: *const c_char) -> qiniu_ng_string_t {
    Cow::Borrowed(unsafe { CStr::from_ptr(s) })
        .into_owned()
        .into_boxed_c_str()
        .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_new_with_len(s: *const c_char, len: usize) -> qiniu_ng_string_t {
    let mut vec: Vec<u8> = Vec::with_capacity(len + 1);
    unsafe {
        vec.set_len(len);
        copy_nonoverlapping(s.cast(), vec.as_mut_ptr(), len);
        CString::from_vec_unchecked(vec).into_boxed_c_str().into()
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_get_ptr(s: qiniu_ng_string_t) -> *const c_char {
    let s: Box<CStr> = s.into();
    s.as_ptr().tap(|_| {
        let _: qiniu_ng_string_t = s.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_get_len(s: qiniu_ng_string_t) -> usize {
    let s: Box<CStr> = s.into();
    s.to_bytes().len().tap(|_| {
        let _: qiniu_ng_string_t = s.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_free(s: qiniu_ng_string_t) {
    let _: Box<CStr> = s.into();
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_optional_string_t(*mut c_char, *mut c_char);

impl Default for qiniu_ng_optional_string_t {
    fn default() -> Self {
        qiniu_ng_optional_string_null()
    }
}

impl From<Option<Box<CStr>>> for qiniu_ng_optional_string_t {
    fn from(s: Option<Box<CStr>>) -> Self {
        if let Some(s) = s {
            unsafe { transmute(Box::into_raw(s)) }
        } else {
            qiniu_ng_optional_string_t(null_mut(), null_mut())
        }
    }
}

impl From<qiniu_ng_optional_string_t> for Option<Box<CStr>> {
    fn from(s: qiniu_ng_optional_string_t) -> Self {
        if s.0.is_null() && s.1.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(s)) })
        }
    }
}

impl From<qiniu_ng_string_t> for qiniu_ng_optional_string_t {
    fn from(s: qiniu_ng_string_t) -> Self {
        let s: Box<CStr> = s.into();
        Some(s).into()
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_null() -> qiniu_ng_optional_string_t {
    qiniu_ng_optional_string_t(null_mut(), null_mut())
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_is_null(s: qiniu_ng_optional_string_t) -> bool {
    s.0.is_null() && s.1.is_null()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_new(s: *const c_char) -> qiniu_ng_optional_string_t {
    if s.is_null() {
        qiniu_ng_optional_string_null()
    } else {
        Some(
            Cow::Borrowed(unsafe { CStr::from_ptr(s) })
                .into_owned()
                .into_boxed_c_str(),
        )
        .into()
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_new_with_len(s: *const c_char, len: usize) -> qiniu_ng_optional_string_t {
    if s.is_null() {
        qiniu_ng_optional_string_null()
    } else {
        let mut vec: Vec<u8> = Vec::with_capacity(len + 1);
        unsafe {
            vec.set_len(len);
            copy_nonoverlapping(s.cast(), vec.as_mut_ptr(), len);
            Some(CString::from_vec_unchecked(vec).into_boxed_c_str()).into()
        }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_get_ptr(s: qiniu_ng_optional_string_t) -> *const c_char {
    let s: Option<Box<CStr>> = s.into();
    s.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null).tap(|_| {
        let _: qiniu_ng_optional_string_t = s.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_get_len(s: qiniu_ng_optional_string_t) -> usize {
    let s: Option<Box<CStr>> = s.into();
    s.as_ref().map(|s| s.to_bytes().len()).unwrap_or(0).tap(|_| {
        let _: qiniu_ng_optional_string_t = s.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_free(s: qiniu_ng_optional_string_t) {
    let _: Option<Box<CStr>> = s.into();
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
pub extern "C" fn qiniu_ng_string_list_new(strlist: *const *const c_char, len: usize) -> qiniu_ng_string_list_t {
    let mut vec: Vec<Box<CStr>> = Vec::with_capacity(len);
    for i in 0..len {
        vec.push(qiniu_ng_string_new(unsafe { *strlist.add(i) }).into());
    }
    vec.into_boxed_slice().into()
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
pub extern "C" fn qiniu_ng_string_map_new(capacity: usize) -> qiniu_ng_string_map_t {
    let hashmap: Box<HashMap<Box<CStr>, Box<CStr>, RandomState>> = Box::new(HashMap::with_capacity(capacity));
    hashmap.into()
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
pub extern "C" fn qiniu_ng_string_map_each_entry(
    hashmap: qiniu_ng_string_map_t,
    handler: fn(key: *const c_char, value: *const c_char, data: *mut c_void) -> bool,
    data: *mut c_void,
) {
    let hashmap: Box<HashMap<Box<CStr>, Box<CStr>, RandomState>> = hashmap.into();
    for (key, value) in hashmap.iter() {
        if !handler(key.as_ptr(), value.as_ptr(), data) {
            break;
        }
    }
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
pub extern "C" fn qiniu_ng_string_map_len(hashmap: qiniu_ng_string_map_t) -> usize {
    let hashmap: Box<HashMap<Box<CStr>, Box<CStr>, RandomState>> = hashmap.into();
    hashmap.len().tap(|_| {
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
            let buf = unsafe { from_raw_parts(path.cast(), strlen(path)) };
            PathBuf::from(OsStr::from_bytes(buf))
        }
    } else if #[cfg(windows)] {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        pub(crate) fn make_path_buf(path: *const c_char) -> PathBuf {
            let buf = unsafe { from_raw_parts(path.cast(), strlen(path)) };
            PathBuf::from(OsStr::from_wide(buf))
        }
    } else {
        panic!("Unsupported platform");
    }
}

pub(crate) fn make_optional_path_buf(path: *const c_char) -> Option<PathBuf> {
    if path.is_null() {
        None
    } else {
        Some(make_path_buf(path))
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
