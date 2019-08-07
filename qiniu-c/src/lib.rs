use cfg_if::cfg_if;
use libc::{c_char, c_int, malloc, size_t, strerror};
use qiniu::utils::etag;
use std::ffi::{CStr, CString};
use std::io;
use std::path::PathBuf;
use std::ptr;
use std::slice;

#[repr(C)]
pub struct qiniu_ng_result {
    pub ok: bool,
    pub error_code: c_int,
    pub description: *const c_char,
}

#[no_mangle]
pub extern "C" fn qiniu_ng_etag_from_file(
    path: *const c_char,
    path_len: size_t,
    result_ptr: *mut *mut c_char,
) -> qiniu_ng_result {
    match etag::from_file(make_path_buf(path as *const u8, path_len)) {
        Ok(etag_string) => {
            assign_string_to_cstring_ptr(etag_string, result_ptr);
            qiniu_ng_result {
                ok: true,
                error_code: 0,
                description: ptr::null(),
            }
        }
        Err(err) => make_qiniu_ng_result_from_io_error(err),
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

#[rustfmt::skip]
fn make_qiniu_ng_result_from_io_error(err: io::Error) -> qiniu_ng_result {
    if let Some(raw_os_error) = err.raw_os_error() {
        qiniu_ng_result {
            ok: false,
            error_code: raw_os_error,
            description: unsafe { strerror(raw_os_error) },
        }
    } else {
        qiniu_ng_result {
            ok: false,
            error_code: -1,
            description: match err.kind() {
                io::ErrorKind::NotFound => CStr::from_bytes_with_nul(b"entity not found\0").unwrap().as_ptr(),
                io::ErrorKind::PermissionDenied => CStr::from_bytes_with_nul(b"permission denied\0").unwrap().as_ptr(),
                io::ErrorKind::ConnectionRefused => CStr::from_bytes_with_nul(b"connection refused\0").unwrap().as_ptr(),
                io::ErrorKind::ConnectionReset => CStr::from_bytes_with_nul(b"connection reset\0").unwrap().as_ptr(),
                io::ErrorKind::ConnectionAborted => CStr::from_bytes_with_nul(b"connection aborted\0").unwrap().as_ptr(),
                io::ErrorKind::NotConnected => CStr::from_bytes_with_nul(b"not connected\0").unwrap().as_ptr(),
                io::ErrorKind::AddrInUse => CStr::from_bytes_with_nul(b"address in use\0").unwrap().as_ptr(),
                io::ErrorKind::AddrNotAvailable => CStr::from_bytes_with_nul(b"address not available\0").unwrap().as_ptr(),
                io::ErrorKind::BrokenPipe => CStr::from_bytes_with_nul(b"broken pipe\0").unwrap().as_ptr(),
                io::ErrorKind::AlreadyExists => CStr::from_bytes_with_nul(b"entity already exists\0").unwrap().as_ptr(),
                io::ErrorKind::WouldBlock => CStr::from_bytes_with_nul(b"operation would block\0").unwrap().as_ptr(),
                io::ErrorKind::InvalidInput => CStr::from_bytes_with_nul(b"invalid input parameter\0").unwrap().as_ptr(),
                io::ErrorKind::InvalidData => CStr::from_bytes_with_nul(b"invalid data\0").unwrap().as_ptr(),
                io::ErrorKind::TimedOut => CStr::from_bytes_with_nul(b"timed out\0").unwrap().as_ptr(),
                io::ErrorKind::WriteZero => CStr::from_bytes_with_nul(b"write zero\0").unwrap().as_ptr(),
                io::ErrorKind::Interrupted => CStr::from_bytes_with_nul(b"operation interrupted\0").unwrap().as_ptr(),
                io::ErrorKind::Other => CStr::from_bytes_with_nul(b"other os error\0").unwrap().as_ptr(),
                io::ErrorKind::UnexpectedEof => CStr::from_bytes_with_nul(b"unexpected end of file\0").unwrap().as_ptr(),
                _ => CStr::from_bytes_with_nul(b"unknown error\0").unwrap().as_ptr(),
            },
        }
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
