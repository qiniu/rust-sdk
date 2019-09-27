use curl_sys::CURLcode;
use libc::{c_char, c_int, c_ushort, strerror};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use qiniu_http::{Error as HTTPError, ErrorKind as HTTPErrorKind};
use std::{ffi::CStr, io};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    OsError(c_int),
    IoError(c_int),
    UnknownError,
    JSONError,
    ResponseStatusCodeError(c_ushort),
    CurlError(CURLcode),
}

#[repr(C)]
pub struct qiniu_ng_err(ErrorCode);

#[derive(FromPrimitive, ToPrimitive)]
enum IoErrorKind {
    NotFound = 1,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    Interrupted,
    Other,
    UnexpectedEof,
    Unknown = -1,
}

impl From<io::ErrorKind> for IoErrorKind {
    fn from(error_kind: io::ErrorKind) -> Self {
        match error_kind {
            io::ErrorKind::NotFound => IoErrorKind::NotFound,
            io::ErrorKind::PermissionDenied => IoErrorKind::PermissionDenied,
            io::ErrorKind::ConnectionRefused => IoErrorKind::ConnectionRefused,
            io::ErrorKind::ConnectionReset => IoErrorKind::ConnectionReset,
            io::ErrorKind::ConnectionAborted => IoErrorKind::ConnectionAborted,
            io::ErrorKind::NotConnected => IoErrorKind::NotConnected,
            io::ErrorKind::AddrInUse => IoErrorKind::AddrInUse,
            io::ErrorKind::AddrNotAvailable => IoErrorKind::AddrNotAvailable,
            io::ErrorKind::BrokenPipe => IoErrorKind::BrokenPipe,
            io::ErrorKind::AlreadyExists => IoErrorKind::AlreadyExists,
            io::ErrorKind::WouldBlock => IoErrorKind::WouldBlock,
            io::ErrorKind::InvalidInput => IoErrorKind::InvalidInput,
            io::ErrorKind::InvalidData => IoErrorKind::InvalidData,
            io::ErrorKind::TimedOut => IoErrorKind::TimedOut,
            io::ErrorKind::WriteZero => IoErrorKind::WriteZero,
            io::ErrorKind::Interrupted => IoErrorKind::Interrupted,
            io::ErrorKind::Other => IoErrorKind::Other,
            io::ErrorKind::UnexpectedEof => IoErrorKind::UnexpectedEof,
            _ => IoErrorKind::Unknown,
        }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_os_error_extract(
    err: *const qiniu_ng_err,
    code: *mut c_int,
    description: *mut *const c_char,
) -> bool {
    if err.is_null() {
        return false;
    }
    match unsafe { (*err).0 } {
        ErrorCode::OsError(os_error_code) => {
            if !code.is_null() {
                unsafe { *code = os_error_code };
            }
            if !description.is_null() {
                unsafe { *description = strerror(os_error_code) };
            }
            true
        }
        _ => false,
    }
}

#[no_mangle]
#[rustfmt::skip]
pub extern "C" fn qiniu_ng_err_io_error_extract(
    err: *const qiniu_ng_err,
    code: *mut c_int,
    description: *mut *const c_char,
) -> bool {
    if err.is_null() {
        return false;
    }
    match unsafe { (*err).0 } {
        ErrorCode::IoError(io_error_code) => {
            if !code.is_null() {
                unsafe { *code = io_error_code };
            }
            if !description.is_null() {
                unsafe {
                    *description = match IoErrorKind::from_i32(io_error_code).unwrap() {
                        IoErrorKind::NotFound => CStr::from_bytes_with_nul_unchecked(b"entity not found\0").as_ptr(),
                        IoErrorKind::PermissionDenied => CStr::from_bytes_with_nul_unchecked(b"permission denied\0").as_ptr(),
                        IoErrorKind::ConnectionRefused => CStr::from_bytes_with_nul_unchecked(b"connection refused\0").as_ptr(),
                        IoErrorKind::ConnectionReset => CStr::from_bytes_with_nul_unchecked(b"connection reset\0").as_ptr(),
                        IoErrorKind::ConnectionAborted => CStr::from_bytes_with_nul_unchecked(b"connection aborted\0").as_ptr(),
                        IoErrorKind::NotConnected => CStr::from_bytes_with_nul_unchecked(b"not connected\0").as_ptr(),
                        IoErrorKind::AddrInUse => CStr::from_bytes_with_nul_unchecked(b"address in use\0").as_ptr(),
                        IoErrorKind::AddrNotAvailable => CStr::from_bytes_with_nul_unchecked(b"address not available\0").as_ptr(),
                        IoErrorKind::BrokenPipe => CStr::from_bytes_with_nul_unchecked(b"broken pipe\0").as_ptr(),
                        IoErrorKind::AlreadyExists => CStr::from_bytes_with_nul_unchecked(b"entity already exists\0").as_ptr(),
                        IoErrorKind::WouldBlock => CStr::from_bytes_with_nul_unchecked(b"operation would block\0").as_ptr(),
                        IoErrorKind::InvalidInput => CStr::from_bytes_with_nul_unchecked(b"invalid input parameter\0").as_ptr(),
                        IoErrorKind::InvalidData => CStr::from_bytes_with_nul_unchecked(b"invalid data\0").as_ptr(),
                        IoErrorKind::TimedOut => CStr::from_bytes_with_nul_unchecked(b"timed out\0").as_ptr(),
                        IoErrorKind::WriteZero => CStr::from_bytes_with_nul_unchecked(b"write zero\0").as_ptr(),
                        IoErrorKind::Interrupted => CStr::from_bytes_with_nul_unchecked(b"operation interrupted\0").as_ptr(),
                        IoErrorKind::Other => CStr::from_bytes_with_nul_unchecked(b"other os error\0").as_ptr(),
                        IoErrorKind::UnexpectedEof => CStr::from_bytes_with_nul_unchecked(b"unexpected end of file\0").as_ptr(),
                        IoErrorKind::Unknown => CStr::from_bytes_with_nul_unchecked(b"unknown error\0").as_ptr(),
                    }
                };
            }
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_is_unknown_error(err: *const qiniu_ng_err) -> bool {
    if err.is_null() {
        return false;
    }
    match unsafe { (*err).0 } {
        ErrorCode::UnknownError => true,
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_is_json_error(err: *const qiniu_ng_err) -> bool {
    if err.is_null() {
        return false;
    }
    match unsafe { (*err).0 } {
        ErrorCode::JSONError => true,
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_response_status_code_error_extract(
    err: *const qiniu_ng_err,
    code: *mut c_ushort,
) -> bool {
    if err.is_null() {
        return false;
    }
    match unsafe { (*err).0 } {
        ErrorCode::ResponseStatusCodeError(status_code) => {
            if !code.is_null() {
                unsafe { *code = status_code };
            }
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_curl_error_extract(err: *const qiniu_ng_err, code: *mut CURLcode) -> bool {
    if err.is_null() {
        return false;
    }
    match unsafe { (*err).0 } {
        ErrorCode::CurlError(curl_code) => {
            if !code.is_null() {
                unsafe { *code = curl_code };
            }
            true
        }
        _ => false,
    }
}

pub(crate) fn make_qiniu_ng_err_from_io_error(err: &io::Error) -> qiniu_ng_err {
    if let Some(raw_os_error) = err.raw_os_error() {
        qiniu_ng_err(ErrorCode::OsError(raw_os_error))
    } else {
        qiniu_ng_err(ErrorCode::IoError(IoErrorKind::from(err.kind()).to_i32().unwrap()))
    }
}

pub(crate) fn make_qiniu_ng_err_from_qiniu_http_error(err: &HTTPError) -> qiniu_ng_err {
    match err.error_kind() {
        HTTPErrorKind::HTTPCallerError(e) => {
            if let Some(e) = e.downcast_ref::<curl::Error>() {
                qiniu_ng_err(ErrorCode::CurlError(e.code()))
            } else {
                qiniu_ng_err(ErrorCode::UnknownError)
            }
        }
        HTTPErrorKind::JSONError(_) => qiniu_ng_err(ErrorCode::JSONError),
        HTTPErrorKind::MaliciousResponse => qiniu_ng_err(ErrorCode::UnknownError),
        HTTPErrorKind::IOError(e) => make_qiniu_ng_err_from_io_error(e),
        HTTPErrorKind::UnknownError(_) => qiniu_ng_err(ErrorCode::UnknownError),
        HTTPErrorKind::ResponseStatusCodeError(status_code, _) => {
            qiniu_ng_err(ErrorCode::ResponseStatusCodeError(status_code.to_owned()))
        }
    }
}
