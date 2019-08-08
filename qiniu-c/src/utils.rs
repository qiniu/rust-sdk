use cfg_if::cfg_if;
use libc::c_char;
use libc::size_t;
use std::path::PathBuf;
use std::slice;

pub(crate) fn write_string_to_ptr<S: AsRef<str>>(src: S, dst: *mut c_char) {
    let src_bytes = src.as_ref();
    unsafe {
        dst.copy_from_nonoverlapping(src_bytes.as_ptr() as *mut c_char, src_bytes.len());
    }
}

cfg_if! {
    if #[cfg(unix)] {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        pub(crate) fn make_path_buf(path: *const u8, path_len: size_t) -> PathBuf {
            let buf = unsafe { slice::from_raw_parts(path, path_len) };
            PathBuf::from(OsStr::from_bytes(buf))
        }
    } else if #[cfg(windows)] {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        pub(crate) fn make_path_buf(path: *const u8, path_len: size_t) -> PathBuf {
            let buf = unsafe { slice::from_raw_parts(path, path_len) };
            PathBuf::from(OsStr::from_wide(buf))
        }
    } else {
        panic!("Unsupported platform");
    }
}
