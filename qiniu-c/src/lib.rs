use cfg_if::cfg_if;
use libc::{c_char, c_int, malloc, size_t};
use qiniu::utils::etag;
use std::ffi::CString;
use std::path::PathBuf;
use std::slice;

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_from_file(
    path: *const c_char,
    path_len: size_t,
    result_ptr: *mut *mut c_char,
) -> c_int {
    match etag::from_file(make_path_buf(path as *const u8, path_len)) {
        Ok(etag_string) => {
            assign_string_to_cstring_ptr(etag_string, result_ptr);
            0
        }
        Err(_) => -1,
    }
}

fn assign_string_to_cstring_ptr<S: AsRef<str>>(src: S, dst: *mut *mut c_char) {
    let src_cstring = CString::new(src.as_ref()).unwrap();
    let src_bytes = src_cstring.as_bytes_with_nul();
    unsafe {
        let mem = malloc(src_bytes.len() as size_t) as *mut c_char;
        if mem.is_null() {
            panic!("malloc failed");
        }
        mem.copy_from_nonoverlapping(src_bytes.as_ptr() as *mut c_char, src_bytes.len());
        *dst = mem;
    }
}

cfg_if! {
    if #[cfg(unix)] {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        fn make_path_buf(path: *const u8, path_len: size_t) -> PathBuf {
            let buf = unsafe { slice::from_raw_parts(path, path_len) };
            PathBuf::from(OsStr::from_bytes(buf))
        }
    } else if #[cfg(windows)] {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        fn make_path_buf(path: *const u8, path_len: size_t) -> PathBuf {
            let buf = unsafe { slice::from_raw_parts(path, path_len) };
            PathBuf::from(OsStr::from_wide(buf))
        }
    } else {
        panic!("Unsupported platform");
    }
}
