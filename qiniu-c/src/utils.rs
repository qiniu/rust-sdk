use cfg_if::cfg_if;
use libc::{c_char, c_void, size_t};
use std::{boxed::Box, ffi::CString, mem, path::PathBuf, slice};

#[repr(C)]
pub struct qiniu_ng_string_list_t(*mut c_void, *mut c_void);

pub(crate) unsafe fn make_string_list<S: AsRef<str>, A: AsRef<[S]>>(list: A) -> qiniu_ng_string_list_t {
    mem::transmute(Box::into_raw({
        list.as_ref()
            .into_iter()
            .map(|s| CString::new(s.as_ref()).unwrap())
            .collect::<Box<[CString]>>()
    }))
}

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_string_list_len(list: qiniu_ng_string_list_t) -> size_t {
    let boxed_list = Box::from_raw(mem::transmute::<_, *mut [CString]>(list));
    let len = boxed_list.len();
    Box::into_raw(boxed_list);
    len
}

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_string_list_get(
    list: qiniu_ng_string_list_t,
    index: size_t,
    str: *mut *const c_char,
) -> bool {
    let boxed_list = Box::from_raw(mem::transmute::<_, *mut [CString]>(list));
    let mut got = false;
    if let Some(s) = boxed_list.get(index) {
        *str = mem::transmute(s.as_ptr());
        got = true;
    }
    Box::into_raw(boxed_list);
    got
}

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_string_list_free(list: qiniu_ng_string_list_t) {
    Box::from_raw(mem::transmute::<_, *mut [CString]>(list));
}

pub(crate) unsafe fn write_string_to_ptr<S: AsRef<str>>(src: S, dst: *mut c_char) {
    let src_bytes = src.as_ref();
    dst.copy_from_nonoverlapping(mem::transmute(src_bytes.as_ptr()), src_bytes.len());
}

cfg_if! {
    if #[cfg(unix)] {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        pub unsafe fn make_path_buf(path: *const u8, path_len: size_t) -> PathBuf {
            let buf = slice::from_raw_parts(path, path_len);
            PathBuf::from(OsStr::from_bytes(buf))
        }
    } else if #[cfg(windows)] {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        pub unsafe fn make_path_buf(path: *const u8, path_len: size_t) -> PathBuf {
            let buf = slice::from_raw_parts(path, path_len);
            PathBuf::from(OsStr::from_wide(buf))
        }
    } else {
        panic!("Unsupported platform");
    }
}
