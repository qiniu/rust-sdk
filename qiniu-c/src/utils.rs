use crate::string::{qiniu_ng_char_t, ucstr, UCString};
use libc::{c_char, c_void, size_t};
use std::{
    boxed::Box,
    collections::{hash_map::RandomState, HashMap},
    ffi::{CStr, CString},
    io::{Error, ErrorKind, Read, Result},
    mem::transmute,
    ptr::{null, null_mut},
};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_str_t(*mut c_void, *mut c_void);

impl From<Box<CStr>> for qiniu_ng_str_t {
    fn from(s: Box<CStr>) -> Self {
        unsafe { transmute(Box::into_raw(s)) }
    }
}

impl From<qiniu_ng_str_t> for Box<CStr> {
    fn from(s: qiniu_ng_str_t) -> Self {
        unsafe { Box::from_raw(transmute(s)) }
    }
}

impl From<CString> for qiniu_ng_str_t {
    fn from(s: CString) -> Self {
        unsafe { transmute(Box::into_raw(s.into_boxed_c_str())) }
    }
}

impl From<qiniu_ng_str_t> for CString {
    fn from(s: qiniu_ng_str_t) -> Self {
        Box::<CStr>::from(s).into()
    }
}

impl qiniu_ng_str_t {
    pub(crate) unsafe fn from_str_unchecked(s: &str) -> Self {
        CString::from_vec_unchecked(s.to_owned().into_bytes())
            .into_boxed_c_str()
            .into()
    }

    pub(crate) unsafe fn from_string_unchecked(s: String) -> Self {
        CString::from_vec_unchecked(s.into_bytes()).into_boxed_c_str().into()
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_new(ptr: *const c_char) -> qiniu_ng_str_t {
    unsafe { CStr::from_ptr(ptr) }.to_owned().into_boxed_c_str().into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_get_ptr(s: qiniu_ng_str_t) -> *const c_char {
    let s = Box::<CStr>::from(s);
    s.as_ptr().tap(|_| {
        let _ = qiniu_ng_str_t::from(s);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_get_len(s: qiniu_ng_str_t) -> size_t {
    let s = Box::<CStr>::from(s);
    s.to_bytes().len().tap(|_| {
        let _ = qiniu_ng_str_t::from(s);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_free(s: qiniu_ng_str_t) {
    let _ = Box::<CStr>::from(s);
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_string_t(*mut c_void, *mut c_void);

impl From<Box<ucstr>> for qiniu_ng_string_t {
    fn from(s: Box<ucstr>) -> Self {
        unsafe { transmute(Box::into_raw(s)) }
    }
}

impl From<qiniu_ng_string_t> for Box<ucstr> {
    fn from(s: qiniu_ng_string_t) -> Self {
        unsafe { Box::from_raw(transmute(s)) }
    }
}

impl From<UCString> for qiniu_ng_string_t {
    fn from(s: UCString) -> Self {
        unsafe { transmute(Box::into_raw(s.into_boxed_ucstr())) }
    }
}

impl From<qiniu_ng_string_t> for UCString {
    fn from(s: qiniu_ng_string_t) -> Self {
        Box::<ucstr>::from(s).into()
    }
}

impl qiniu_ng_string_t {
    pub(crate) unsafe fn from_str_unchecked(s: &str) -> Self {
        UCString::from_str_unchecked(s).into_boxed_ucstr().into()
    }

    pub(crate) unsafe fn from_string_unchecked(s: String) -> Self {
        UCString::from_string_unchecked(s).into_boxed_ucstr().into()
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_new(ptr: *const qiniu_ng_char_t) -> qiniu_ng_string_t {
    unsafe { UCString::from_ptr(ptr) }.into_boxed_ucstr().into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_get_ptr(s: qiniu_ng_string_t) -> *const qiniu_ng_char_t {
    let s = Box::<ucstr>::from(s);
    s.as_ptr().tap(|_| {
        let _ = qiniu_ng_string_t::from(s);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_get_len(s: qiniu_ng_string_t) -> size_t {
    let s = Box::<ucstr>::from(s);
    s.len().tap(|_| {
        let _ = qiniu_ng_string_t::from(s);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_free(s: qiniu_ng_string_t) {
    let _ = Box::<ucstr>::from(s);
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_optional_str_t(*mut c_void, *mut c_void);

impl qiniu_ng_optional_str_t {
    pub(crate) unsafe fn from_str_unchecked(s: Option<&str>) -> Self {
        s.map(|s| {
            CString::from_vec_unchecked(s.to_owned().into_bytes())
                .into_boxed_c_str()
                .into()
        })
        .unwrap_or_default()
    }

    pub(crate) unsafe fn from_string_unchecked(s: Option<String>) -> Self {
        s.map(|s| CString::from_vec_unchecked(s.into_bytes()).into_boxed_c_str().into())
            .unwrap_or_default()
    }

    #[inline]
    fn is_null(self) -> bool {
        self.0.is_null() && self.1.is_null()
    }
}

impl Default for qiniu_ng_optional_str_t {
    #[inline]
    fn default() -> Self {
        qiniu_ng_optional_str_t(null_mut(), null_mut())
    }
}

impl From<Option<Box<CStr>>> for qiniu_ng_optional_str_t {
    fn from(s: Option<Box<CStr>>) -> Self {
        s.map(|s| unsafe { transmute(Box::into_raw(s)) }).unwrap_or_default()
    }
}

impl From<Option<CString>> for qiniu_ng_optional_str_t {
    fn from(s: Option<CString>) -> Self {
        s.map(|s| unsafe { transmute(Box::into_raw(s.into_boxed_c_str())) })
            .unwrap_or_default()
    }
}

impl From<Box<CStr>> for qiniu_ng_optional_str_t {
    #[inline]
    fn from(s: Box<CStr>) -> Self {
        Some(s).into()
    }
}

impl From<CString> for qiniu_ng_optional_str_t {
    #[inline]
    fn from(s: CString) -> Self {
        Some(s).into()
    }
}

impl From<qiniu_ng_optional_str_t> for Option<Box<CStr>> {
    fn from(s: qiniu_ng_optional_str_t) -> Self {
        if s.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(s)) })
        }
    }
}

impl From<qiniu_ng_optional_str_t> for Option<CString> {
    fn from(s: qiniu_ng_optional_str_t) -> Self {
        Option::<Box<CStr>>::from(s).map(|s| s.into())
    }
}

impl From<qiniu_ng_str_t> for qiniu_ng_optional_str_t {
    fn from(s: qiniu_ng_str_t) -> Self {
        Some(Box::<CStr>::from(s)).into()
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_str_null() -> qiniu_ng_optional_str_t {
    Default::default()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_str_is_null(s: qiniu_ng_optional_str_t) -> bool {
    s.is_null()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_str_new(ptr: *const c_char) -> qiniu_ng_optional_str_t {
    unsafe { ptr.as_ref() }
        .map(|ptr| unsafe { CStr::from_ptr(ptr) }.to_owned().into_boxed_c_str().into())
        .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_str_get_ptr(s: qiniu_ng_optional_str_t) -> *const c_char {
    let s = Option::<Box<CStr>>::from(s);
    s.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null).tap(|_| {
        let _ = qiniu_ng_optional_str_t::from(s);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_str_get_len(s: qiniu_ng_optional_str_t) -> size_t {
    let s = Option::<Box<CStr>>::from(s);
    s.as_ref().map(|s| s.to_bytes().len()).unwrap_or(0).tap(|_| {
        let _ = qiniu_ng_optional_str_t::from(s);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_str_free(s: qiniu_ng_optional_str_t) {
    let _ = Option::<Box<CStr>>::from(s);
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_optional_string_t(*mut c_void, *mut c_void);

impl qiniu_ng_optional_string_t {
    pub(crate) unsafe fn from_str_unchecked(s: Option<&str>) -> Self {
        s.map(|s| UCString::from_str_unchecked(s).into()).unwrap_or_default()
    }

    pub(crate) unsafe fn from_string_unchecked(s: Option<String>) -> Self {
        s.map(|s| UCString::from_string_unchecked(s).into_boxed_ucstr().into())
            .unwrap_or_default()
    }

    #[inline]
    fn is_null(self) -> bool {
        self.0.is_null() && self.1.is_null()
    }
}

impl Default for qiniu_ng_optional_string_t {
    #[inline]
    fn default() -> Self {
        qiniu_ng_optional_string_t(null_mut(), null_mut())
    }
}

impl From<Option<Box<ucstr>>> for qiniu_ng_optional_string_t {
    fn from(s: Option<Box<ucstr>>) -> Self {
        s.map(|s| unsafe { transmute(Box::into_raw(s)) }).unwrap_or_default()
    }
}

impl From<Option<UCString>> for qiniu_ng_optional_string_t {
    fn from(s: Option<UCString>) -> Self {
        s.map(|s| unsafe { transmute(Box::into_raw(s.into_boxed_ucstr())) })
            .unwrap_or_default()
    }
}

impl From<Box<ucstr>> for qiniu_ng_optional_string_t {
    #[inline]
    fn from(s: Box<ucstr>) -> Self {
        Some(s).into()
    }
}

impl From<UCString> for qiniu_ng_optional_string_t {
    #[inline]
    fn from(s: UCString) -> Self {
        Some(s).into()
    }
}

impl From<qiniu_ng_optional_string_t> for Option<Box<ucstr>> {
    fn from(s: qiniu_ng_optional_string_t) -> Self {
        if s.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(s)) })
        }
    }
}

impl From<qiniu_ng_optional_string_t> for Option<UCString> {
    fn from(s: qiniu_ng_optional_string_t) -> Self {
        Option::<Box<ucstr>>::from(s).map(|s| s.into())
    }
}

impl From<qiniu_ng_string_t> for qiniu_ng_optional_string_t {
    fn from(s: qiniu_ng_string_t) -> Self {
        Box::<ucstr>::from(s).into()
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_null() -> qiniu_ng_optional_string_t {
    Default::default()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_is_null(s: qiniu_ng_optional_string_t) -> bool {
    s.is_null()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_new(ptr: *const qiniu_ng_char_t) -> qiniu_ng_optional_string_t {
    unsafe { ptr.as_ref() }
        .map(|ptr| unsafe { UCString::from_ptr(ptr) }.into_boxed_ucstr().into())
        .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_get_ptr(s: qiniu_ng_optional_string_t) -> *const qiniu_ng_char_t {
    let s = Option::<Box<ucstr>>::from(s);
    s.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null).tap(|_| {
        let _ = qiniu_ng_optional_string_t::from(s);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_get_len(s: qiniu_ng_optional_string_t) -> size_t {
    let s = Option::<Box<ucstr>>::from(s);
    s.as_ref().map(|s| s.len()).unwrap_or(0).tap(|_| {
        let _ = qiniu_ng_optional_string_t::from(s);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_optional_string_free(s: qiniu_ng_optional_string_t) {
    let _ = Option::<Box<ucstr>>::from(s);
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_str_list_t(*mut c_void, *mut c_void);

impl qiniu_ng_str_list_t {
    pub(crate) unsafe fn from_str_slice_unchecked(list: &[&str]) -> Self {
        list.iter()
            .map(|s| CString::from_vec_unchecked((*s).to_owned().into_bytes()).into_boxed_c_str())
            .collect::<Box<[_]>>()
            .into()
    }

    pub(crate) unsafe fn from_string_vec_unchecked(list: Vec<String>) -> Self {
        list.into_iter()
            .map(|s| CString::from_vec_unchecked(s.into_bytes()).into_boxed_c_str())
            .collect::<Box<[_]>>()
            .into()
    }
}

impl From<Box<[Box<CStr>]>> for qiniu_ng_str_list_t {
    fn from(strlist: Box<[Box<CStr>]>) -> Self {
        unsafe { transmute(Box::into_raw(strlist)) }
    }
}

impl From<qiniu_ng_str_list_t> for Box<[Box<CStr>]> {
    fn from(strlist: qiniu_ng_str_list_t) -> Self {
        unsafe { Box::from_raw(transmute(strlist)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_new(strlist: *const *const c_char, len: size_t) -> qiniu_ng_str_list_t {
    let mut vec: Vec<Box<CStr>> = Vec::with_capacity(len);
    for i in 0..len {
        vec.push(unsafe { CStr::from_ptr(*strlist.add(i)) }.to_owned().into_boxed_c_str());
    }
    vec.into_boxed_slice().into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_len(strlist: qiniu_ng_str_list_t) -> size_t {
    let strlist = Box::<[Box<CStr>]>::from(strlist);
    strlist.len().tap(|_| {
        let _ = qiniu_ng_str_list_t::from(strlist);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_get(
    strlist: qiniu_ng_str_list_t,
    index: size_t,
    str_ptr: *mut *const c_char,
) -> bool {
    let strlist = Box::<[Box<CStr>]>::from(strlist);
    let mut got = false;
    if let Some(s) = strlist.get(index) {
        if let Some(str_ptr) = unsafe { str_ptr.as_mut() } {
            *str_ptr = s.as_ptr();
        }
        got = true;
    }
    let _ = qiniu_ng_str_list_t::from(strlist);
    got
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_free(strlist: qiniu_ng_str_list_t) {
    let _ = Box::<[Box<CStr>]>::from(strlist);
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_string_list_t(*mut c_void, *mut c_void);

impl qiniu_ng_string_list_t {
    pub(crate) unsafe fn from_str_slice_unchecked(list: &[&str]) -> Self {
        list.iter()
            .map(|s| UCString::from_str_unchecked(s).into_boxed_ucstr())
            .collect::<Box<[_]>>()
            .into()
    }

    pub(crate) unsafe fn from_string_vec_unchecked(list: Vec<String>) -> Self {
        list.into_iter()
            .map(|s| UCString::from_string_unchecked(s).into_boxed_ucstr())
            .collect::<Box<[_]>>()
            .into()
    }
}

impl From<Box<[Box<ucstr>]>> for qiniu_ng_string_list_t {
    fn from(strlist: Box<[Box<ucstr>]>) -> Self {
        unsafe { transmute(Box::into_raw(strlist)) }
    }
}

impl From<qiniu_ng_string_list_t> for Box<[Box<ucstr>]> {
    fn from(strlist: qiniu_ng_string_list_t) -> Self {
        unsafe { Box::from_raw(transmute(strlist)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_list_new(
    strlist: *const *const qiniu_ng_char_t,
    len: size_t,
) -> qiniu_ng_string_list_t {
    let mut vec = Vec::with_capacity(len);
    for i in 0..len {
        vec.push(unsafe { UCString::from_ptr(*strlist.add(i)) }.into_boxed_ucstr());
    }
    vec.into_boxed_slice().into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_list_len(strlist: qiniu_ng_string_list_t) -> size_t {
    let strlist = Box::<[Box<ucstr>]>::from(strlist);
    strlist.len().tap(|_| {
        let _ = qiniu_ng_string_list_t::from(strlist);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_list_get(
    strlist: qiniu_ng_string_list_t,
    index: size_t,
    str_ptr: *mut *const qiniu_ng_char_t,
) -> bool {
    let strlist = Box::<[Box<ucstr>]>::from(strlist);
    let mut got = false;
    if let Some(s) = strlist.get(index) {
        if let Some(str_ptr) = unsafe { str_ptr.as_mut() } {
            *str_ptr = s.as_ptr();
        }
        got = true;
    }
    let _ = qiniu_ng_string_list_t::from(strlist);
    got
}

#[no_mangle]
pub extern "C" fn qiniu_ng_string_list_free(strlist: qiniu_ng_string_list_t) {
    let _ = Box::<[Box<ucstr>]>::from(strlist);
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_str_map_t(*mut c_void);

impl From<Box<HashMap<Box<CStr>, Box<CStr>, RandomState>>> for qiniu_ng_str_map_t {
    fn from(hashmap: Box<HashMap<Box<CStr>, Box<CStr>, RandomState>>) -> Self {
        unsafe { transmute(Box::into_raw(hashmap)) }
    }
}

impl From<qiniu_ng_str_map_t> for Box<HashMap<Box<CStr>, Box<CStr>, RandomState>> {
    fn from(hashmap: qiniu_ng_str_map_t) -> Self {
        unsafe { Box::from_raw(transmute(hashmap)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_new(capacity: size_t) -> qiniu_ng_str_map_t {
    Box::<HashMap<Box<CStr>, Box<CStr>, RandomState>>::new(HashMap::with_capacity(capacity)).into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_set(hashmap: qiniu_ng_str_map_t, key: *const c_char, value: *const c_char) {
    let mut hashmap = Box::<HashMap<Box<CStr>, Box<CStr>, RandomState>>::from(hashmap);
    hashmap.insert(
        unsafe { CStr::from_ptr(key) }.to_owned().into_boxed_c_str(),
        unsafe { CStr::from_ptr(value) }.to_owned().into_boxed_c_str(),
    );
    let _ = qiniu_ng_str_map_t::from(hashmap);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_each_entry(
    hashmap: qiniu_ng_str_map_t,
    handler: fn(key: *const c_char, value: *const c_char, data: *mut c_void) -> bool,
    data: *mut c_void,
) {
    let hashmap = Box::<HashMap<Box<CStr>, Box<CStr>, RandomState>>::from(hashmap);
    for (key, value) in hashmap.iter() {
        if !handler(key.as_ptr(), value.as_ptr(), data) {
            break;
        }
    }
    let _ = qiniu_ng_str_map_t::from(hashmap);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_get(hashmap: qiniu_ng_str_map_t, key: *const c_char) -> *const c_char {
    let hashmap = Box::<HashMap<Box<CStr>, Box<CStr>, RandomState>>::from(hashmap);
    hashmap
        .get(unsafe { CStr::from_ptr(key) })
        .map(|val| val.as_ptr())
        .unwrap_or_else(null)
        .tap(|_| {
            let _ = qiniu_ng_str_map_t::from(hashmap);
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_len(hashmap: qiniu_ng_str_map_t) -> size_t {
    let hashmap = Box::<HashMap<Box<CStr>, Box<CStr>, RandomState>>::from(hashmap);
    hashmap.len().tap(|_| {
        let _ = qiniu_ng_str_map_t::from(hashmap);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_free(hashmap: qiniu_ng_str_map_t) {
    let _ = Box::<HashMap<Box<CStr>, Box<CStr>, RandomState>>::from(hashmap);
}

#[repr(C)]
#[derive(Clone)]
pub struct qiniu_ng_readable_t {
    read_func: fn(context: *mut c_void, buf: *mut c_void, count: size_t, have_read: *mut size_t) -> bool,
    context: *mut c_void,
}

impl Read for qiniu_ng_readable_t {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut have_read: size_t = 0;
        if (self.read_func)(self.context, buf.as_mut_ptr().cast(), buf.len(), &mut have_read) {
            Ok(have_read)
        } else {
            Err(Error::new(ErrorKind::Other, "User callback returns false"))
        }
    }
}
unsafe impl Send for qiniu_ng_readable_t {}
