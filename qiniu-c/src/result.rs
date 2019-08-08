use libc::{c_char, c_int, strerror};
use std::ffi::CStr;
use std::io;
use std::ptr;

#[repr(C)]
pub struct qiniu_ng_result {
    pub ok: bool,
    pub error_code: c_int,
    pub description: *const c_char,
}

pub(crate) const QINIU_NG_RESULT_OK: qiniu_ng_result = qiniu_ng_result {
    ok: true,
    error_code: 0,
    description: ptr::null(),
};

#[rustfmt::skip]
pub(crate) fn make_qiniu_ng_result_from_io_error(err: io::Error) -> qiniu_ng_result {
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
