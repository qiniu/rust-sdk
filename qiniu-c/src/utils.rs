use crate::string::{qiniu_ng_char_t, ucstr, UCString};
use libc::{c_void, size_t};
use std::{
    boxed::Box,
    collections::{hash_map::RandomState, HashMap},
    io::{Error, ErrorKind, Read, Result},
    mem::transmute,
    ptr::{null, null_mut},
};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_str_t(*mut c_void, *mut c_void);

// TODO: 提供 String 的 eql() 方法
// TODO: 提供 String 的 clone() 方法

impl qiniu_ng_str_t {
    pub(crate) unsafe fn from_str_unchecked(s: &str) -> Self {
        UCString::from_str_unchecked(s).into()
    }

    pub(crate) unsafe fn from_optional_str_unchecked(s: Option<&str>) -> Self {
        s.map(|s| UCString::from_str_unchecked(s).into()).unwrap_or_default()
    }

    pub(crate) unsafe fn from_string_unchecked(s: String) -> Self {
        UCString::from_string_unchecked(s).into_boxed_ucstr().into()
    }

    pub(crate) unsafe fn from_optional_string_unchecked(s: Option<String>) -> Self {
        s.map(|s| UCString::from_string_unchecked(s).into_boxed_ucstr().into())
            .unwrap_or_default()
    }

    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null() && self.1.is_null()
    }
}

impl Default for qiniu_ng_str_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut(), null_mut())
    }
}

impl From<Option<Box<ucstr>>> for qiniu_ng_str_t {
    fn from(s: Option<Box<ucstr>>) -> Self {
        s.map(|s| unsafe { transmute(Box::into_raw(s)) }).unwrap_or_default()
    }
}

impl From<Option<UCString>> for qiniu_ng_str_t {
    fn from(s: Option<UCString>) -> Self {
        s.map(|s| unsafe { transmute(Box::into_raw(s.into_boxed_ucstr())) })
            .unwrap_or_default()
    }
}

impl From<Box<ucstr>> for qiniu_ng_str_t {
    #[inline]
    fn from(s: Box<ucstr>) -> Self {
        Some(s).into()
    }
}

impl From<UCString> for qiniu_ng_str_t {
    #[inline]
    fn from(s: UCString) -> Self {
        Some(s).into()
    }
}

impl From<qiniu_ng_str_t> for Option<Box<ucstr>> {
    fn from(s: qiniu_ng_str_t) -> Self {
        if s.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(s)) })
        }
    }
}

impl From<qiniu_ng_str_t> for Option<UCString> {
    fn from(s: qiniu_ng_str_t) -> Self {
        Option::<Box<ucstr>>::from(s).map(|s| s.into())
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_is_null(s: qiniu_ng_str_t) -> bool {
    s.is_null()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_new(ptr: *const qiniu_ng_char_t) -> qiniu_ng_str_t {
    unsafe { ptr.as_ref() }
        .map(|ptr| unsafe { UCString::from_ptr(ptr) }.into_boxed_ucstr().into())
        .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_get_ptr(s: qiniu_ng_str_t) -> *const qiniu_ng_char_t {
    let s = Option::<Box<ucstr>>::from(s);
    s.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null).tap(|_| {
        let _ = qiniu_ng_str_t::from(s);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_get_len(s: qiniu_ng_str_t) -> size_t {
    let s = Option::<Box<ucstr>>::from(s);
    s.as_ref().map(|s| s.len()).unwrap_or(0).tap(|_| {
        let _ = qiniu_ng_str_t::from(s);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_free(s: *mut qiniu_ng_str_t) {
    if let Some(s) = unsafe { s.as_mut() } {
        let _ = Option::<Box<ucstr>>::from(*s);
        *s = qiniu_ng_str_t::default();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_is_freed(s: qiniu_ng_str_t) -> bool {
    s.is_null()
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_str_list_t(*mut c_void, *mut c_void);

// TODO: 提供 StrList 的 eql() 方法
// TODO: 提供 StrList 的 clone() 方法

impl Default for qiniu_ng_str_list_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut(), null_mut())
    }
}

impl qiniu_ng_str_list_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null() && self.1.is_null()
    }
}

impl qiniu_ng_str_list_t {
    pub(crate) unsafe fn from_str_slice_unchecked(list: &[&str]) -> Self {
        list.iter()
            .map(|s| UCString::from_str_unchecked(s).into_boxed_ucstr())
            .collect::<Box<[_]>>()
            .into()
    }

    pub(crate) unsafe fn from_optional_str_slice_unchecked(list: Option<&[&str]>) -> Self {
        list.map(|list| Self::from_str_slice_unchecked(list))
            .unwrap_or_default()
    }

    pub(crate) unsafe fn from_string_vec_unchecked(list: Vec<String>) -> Self {
        list.into_iter()
            .map(|s| UCString::from_string_unchecked(s).into_boxed_ucstr())
            .collect::<Box<[_]>>()
            .into()
    }

    pub(crate) unsafe fn from_optional_string_vec_unchecked(list: Option<Vec<String>>) -> Self {
        list.map(|list| Self::from_string_vec_unchecked(list))
            .unwrap_or_default()
    }
}

impl From<Box<[Box<ucstr>]>> for qiniu_ng_str_list_t {
    fn from(strlist: Box<[Box<ucstr>]>) -> Self {
        unsafe { transmute(Box::into_raw(strlist)) }
    }
}

impl From<Option<Box<[Box<ucstr>]>>> for qiniu_ng_str_list_t {
    fn from(strlist: Option<Box<[Box<ucstr>]>>) -> Self {
        strlist.map(|strlist| strlist.into()).unwrap_or_default()
    }
}

impl From<qiniu_ng_str_list_t> for Option<Box<[Box<ucstr>]>> {
    fn from(strlist: qiniu_ng_str_list_t) -> Self {
        if strlist.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(strlist)) })
        }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_new(strlist: *const *const qiniu_ng_char_t, len: size_t) -> qiniu_ng_str_list_t {
    (0..len)
        .map(|i| unsafe { UCString::from_ptr(*strlist.add(i)) }.into_boxed_ucstr())
        .collect::<Box<[_]>>()
        .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_len(strlist: qiniu_ng_str_list_t) -> size_t {
    let strlist = Option::<Box<[Box<ucstr>]>>::from(strlist);
    strlist.as_ref().map(|list| list.len()).unwrap_or(0).tap(|_| {
        let _ = qiniu_ng_str_list_t::from(strlist);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_get(strlist: qiniu_ng_str_list_t, index: size_t) -> *const qiniu_ng_char_t {
    let strlist = Option::<Box<[Box<ucstr>]>>::from(strlist);
    strlist
        .as_ref()
        .and_then(|list| list.get(index))
        .map(|s| s.as_ptr())
        .unwrap_or_else(null)
        .tap(|_| {
            let _ = qiniu_ng_str_list_t::from(strlist);
        })
        .cast()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_free(strlist: *mut qiniu_ng_str_list_t) {
    if let Some(strlist) = unsafe { strlist.as_mut() } {
        let _ = Option::<Box<[Box<ucstr>]>>::from(*strlist);
        *strlist = qiniu_ng_str_list_t::default();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_list_is_freed(strlist: qiniu_ng_str_list_t) -> bool {
    strlist.is_null()
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_str_map_t(*mut c_void);

// TODO: 提供 StrMap 的 eql() 方法
// TODO: 提供 StrMap 的 clone() 方法

impl Default for qiniu_ng_str_map_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_str_map_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>> for qiniu_ng_str_map_t {
    fn from(hashmap: Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>) -> Self {
        unsafe { transmute(Box::into_raw(hashmap)) }
    }
}

impl From<Option<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>> for qiniu_ng_str_map_t {
    fn from(hashmap: Option<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>) -> Self {
        hashmap.map(|hashmap| hashmap.into()).unwrap_or_default()
    }
}

impl From<qiniu_ng_str_map_t> for Option<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>> {
    fn from(hashmap: qiniu_ng_str_map_t) -> Self {
        if hashmap.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(hashmap)) })
        }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_new(capacity: size_t) -> qiniu_ng_str_map_t {
    Box::<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>::new(HashMap::with_capacity(capacity)).into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_set(
    hashmap: qiniu_ng_str_map_t,
    key: *const qiniu_ng_char_t,
    value: *const qiniu_ng_char_t,
) {
    let mut hashmap = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(hashmap).unwrap();
    hashmap.insert(
        unsafe { ucstr::from_ptr(key) }.to_owned().into_boxed_ucstr(),
        unsafe { ucstr::from_ptr(value) }.to_owned().into_boxed_ucstr(),
    );
    let _ = qiniu_ng_str_map_t::from(hashmap);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_each_entry(
    hashmap: qiniu_ng_str_map_t,
    handler: fn(key: *const qiniu_ng_char_t, value: *const qiniu_ng_char_t, data: *mut c_void) -> bool,
    data: *mut c_void,
) {
    let hashmap = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(hashmap);
    if let Some(hashmap) = hashmap.as_ref() {
        for (key, value) in hashmap.iter() {
            if !handler(key.as_ptr(), value.as_ptr(), data) {
                break;
            }
        }
    }
    let _ = qiniu_ng_str_map_t::from(hashmap);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_get(
    hashmap: qiniu_ng_str_map_t,
    key: *const qiniu_ng_char_t,
) -> *const qiniu_ng_char_t {
    let hashmap = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(hashmap);
    hashmap
        .as_ref()
        .and_then(|hashmap| hashmap.get(unsafe { ucstr::from_ptr(key) }))
        .map(|val| val.as_ptr())
        .unwrap_or_else(null)
        .tap(|_| {
            let _ = qiniu_ng_str_map_t::from(hashmap);
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_len(hashmap: qiniu_ng_str_map_t) -> size_t {
    let hashmap = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(hashmap);
    hashmap.as_ref().map(|hashmap| hashmap.len()).unwrap_or(0).tap(|_| {
        let _ = qiniu_ng_str_map_t::from(hashmap);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_str_map_free(hashmap: *mut qiniu_ng_str_map_t) {
    if let Some(hashmap) = unsafe { hashmap.as_mut() } {
        let _ = Option::<Box<HashMap<Box<ucstr>, Box<ucstr>, RandomState>>>::from(*hashmap);
        *hashmap = qiniu_ng_str_map_t::default();
    }
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
