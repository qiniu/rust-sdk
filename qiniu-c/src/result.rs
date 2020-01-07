#[cfg(any(feature = "use-libcurl"))]
use curl_sys::CURLcode;

use libc::{c_char, c_int, c_ushort, strerror};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use qiniu_ng::{
    http::{domains_manager::PersistentError, Error as HTTPError, ErrorKind as HTTPErrorKind},
    storage::{manager::DropBucketError, upload_token::UploadTokenParseError, uploader::UploadError},
};
use std::{
    ffi::CStr,
    io::{Error as IOError, ErrorKind as IOErrorKind},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum QiniuNgInvalidUploadTokenCode {
    InvalidUploadTokenFormat = 1,
    Base64DecodeError,
    JSONDecodeError,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum qiniu_ng_err_code {
    QiniuNgOsError(c_int),
    QiniuNgIoError(c_int),
    QiniuNgUnknownError,
    QiniuNgUnexpectedRedirectError,
    QiniuNgUserCanceled,
    QiniuNgJSONError,
    QiniuNgResponseStatusCodeError(c_ushort),

    #[cfg(any(feature = "use-libcurl"))]
    QiniuNgCurlError(CURLcode),
    /* Particular error */
    QiniuNgCannotDropNonEmptyBucket,
    QiniuNgInvalidUploadToken(QiniuNgInvalidUploadTokenCode),
    QiniuNgBadMIMEType,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct qiniu_ng_err(qiniu_ng_err_code);

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug, PartialEq, Eq)]
enum QiniuNgIoErrorKind {
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

impl From<IOErrorKind> for QiniuNgIoErrorKind {
    fn from(error_kind: IOErrorKind) -> Self {
        match error_kind {
            IOErrorKind::NotFound => QiniuNgIoErrorKind::NotFound,
            IOErrorKind::PermissionDenied => QiniuNgIoErrorKind::PermissionDenied,
            IOErrorKind::ConnectionRefused => QiniuNgIoErrorKind::ConnectionRefused,
            IOErrorKind::ConnectionReset => QiniuNgIoErrorKind::ConnectionReset,
            IOErrorKind::ConnectionAborted => QiniuNgIoErrorKind::ConnectionAborted,
            IOErrorKind::NotConnected => QiniuNgIoErrorKind::NotConnected,
            IOErrorKind::AddrInUse => QiniuNgIoErrorKind::AddrInUse,
            IOErrorKind::AddrNotAvailable => QiniuNgIoErrorKind::AddrNotAvailable,
            IOErrorKind::BrokenPipe => QiniuNgIoErrorKind::BrokenPipe,
            IOErrorKind::AlreadyExists => QiniuNgIoErrorKind::AlreadyExists,
            IOErrorKind::WouldBlock => QiniuNgIoErrorKind::WouldBlock,
            IOErrorKind::InvalidInput => QiniuNgIoErrorKind::InvalidInput,
            IOErrorKind::InvalidData => QiniuNgIoErrorKind::InvalidData,
            IOErrorKind::TimedOut => QiniuNgIoErrorKind::TimedOut,
            IOErrorKind::WriteZero => QiniuNgIoErrorKind::WriteZero,
            IOErrorKind::Interrupted => QiniuNgIoErrorKind::Interrupted,
            IOErrorKind::Other => QiniuNgIoErrorKind::Other,
            IOErrorKind::UnexpectedEof => QiniuNgIoErrorKind::UnexpectedEof,
            _ => QiniuNgIoErrorKind::Unknown,
        }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_os_error_extract(
    err: *const qiniu_ng_err,
    code: *mut c_int,
    description: *mut *const c_char,
) -> bool {
    if let Some(err) = unsafe { err.as_ref() } {
        match err.0 {
            qiniu_ng_err_code::QiniuNgOsError(os_error_code) => {
                if let Some(code) = unsafe { code.as_mut() } {
                    *code = os_error_code;
                }
                if let Some(description) = unsafe { description.as_mut() } {
                    *description = unsafe { strerror(os_error_code) };
                }
                true
            }
            _ => false,
        }
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_io_error_extract(
    err: *const qiniu_ng_err,
    code: *mut c_int,
    description: *mut *const c_char,
) -> bool {
    if let Some(err) = unsafe { err.as_ref() } {
        match err.0 {
            qiniu_ng_err_code::QiniuNgIoError(io_error_code) => {
                if let Some(code) = unsafe { code.as_mut() } {
                    *code = io_error_code;
                }
                if let Some(description) = unsafe { description.as_mut() } {
                    *description = match QiniuNgIoErrorKind::from_i32(io_error_code).unwrap() {
                        QiniuNgIoErrorKind::NotFound => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"entity not found\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::PermissionDenied => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"permission denied\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::ConnectionRefused => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"connection refused\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::ConnectionReset => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"connection reset\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::ConnectionAborted => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"connection aborted\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::NotConnected => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"not connected\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::AddrInUse => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"address in use\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::AddrNotAvailable => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"address not available\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::BrokenPipe => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"broken pipe\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::AlreadyExists => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"entity already exists\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::WouldBlock => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"operation would block\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::InvalidInput => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"invalid input parameter\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::InvalidData => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"invalid data\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::TimedOut => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"timed out\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::WriteZero => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"write zero\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::Interrupted => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"operation interrupted\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::Other => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"other os error\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::UnexpectedEof => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"unexpected end of file\0") }.as_ptr()
                        }
                        QiniuNgIoErrorKind::Unknown => {
                            unsafe { CStr::from_bytes_with_nul_unchecked(b"unknown error\0") }.as_ptr()
                        }
                    }
                }
                true
            }
            _ => false,
        }
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_is_unknown_error(err: *const qiniu_ng_err) -> bool {
    if let Some(err) = unsafe { err.as_ref() } {
        match err.0 {
            qiniu_ng_err_code::QiniuNgUnknownError => true,
            _ => false,
        }
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_is_json_error(err: *const qiniu_ng_err) -> bool {
    if let Some(err) = unsafe { err.as_ref() } {
        match err.0 {
            qiniu_ng_err_code::QiniuNgJSONError => true,
            _ => false,
        }
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_response_status_code_error_extract(
    err: *const qiniu_ng_err,
    code: *mut c_ushort,
) -> bool {
    if let Some(err) = unsafe { err.as_ref() } {
        match err.0 {
            qiniu_ng_err_code::QiniuNgResponseStatusCodeError(status_code) => {
                if let Some(code) = unsafe { code.as_mut() } {
                    *code = status_code;
                }
                true
            }
            _ => false,
        }
    } else {
        false
    }
}

#[cfg(any(feature = "use-libcurl"))]
#[no_mangle]
pub extern "C" fn qiniu_ng_err_curl_error_extract(err: *const qiniu_ng_err, code: *mut CURLcode) -> bool {
    if let Some(err) = unsafe { err.as_ref() } {
        match err.0 {
            qiniu_ng_err_code::QiniuNgCurlError(curl_code) => {
                if let Some(code) = unsafe { code.as_mut() } {
                    *code = curl_code;
                }
                true
            }
            _ => false,
        }
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_is_cannot_drop_non_empty_bucket(err: *const qiniu_ng_err) -> bool {
    if let Some(err) = unsafe { err.as_ref() } {
        match err.0 {
            qiniu_ng_err_code::QiniuNgCannotDropNonEmptyBucket => true,
            _ => false,
        }
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_extract(
    err: *const qiniu_ng_err,
    code: *mut QiniuNgInvalidUploadTokenCode,
) -> bool {
    if let Some(err) = unsafe { err.as_ref() } {
        match err.0 {
            qiniu_ng_err_code::QiniuNgInvalidUploadToken(invalid_upload_token_code) => {
                if let Some(code) = unsafe { code.as_mut() } {
                    *code = invalid_upload_token_code;
                }
                true
            }
            _ => false,
        }
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_is_bad_mime(err: *const qiniu_ng_err) -> bool {
    if let Some(err) = unsafe { err.as_ref() } {
        match err.0 {
            qiniu_ng_err_code::QiniuNgBadMIMEType => true,
            _ => false,
        }
    } else {
        false
    }
}

impl From<&IOError> for qiniu_ng_err {
    fn from(err: &IOError) -> Self {
        qiniu_ng_err(err.raw_os_error().map_or_else(
            || qiniu_ng_err_code::QiniuNgIoError(QiniuNgIoErrorKind::from(err.kind()).to_i32().unwrap()),
            qiniu_ng_err_code::QiniuNgOsError,
        ))
    }
}

impl From<&HTTPError> for qiniu_ng_err {
    fn from(err: &HTTPError) -> Self {
        match err.error_kind() {
            HTTPErrorKind::HTTPCallerError(e) => qiniu_ng_err(
                #[cfg(any(feature = "use-libcurl"))]
                {
                    e.inner()
                        .downcast_ref::<curl::Error>()
                        .map_or(qiniu_ng_err_code::QiniuNgUnknownError, |e| {
                            qiniu_ng_err_code::QiniuNgCurlError(e.code())
                        })
                },
                #[cfg(not(feature = "use-libcurl"))]
                {
                    std::mem::drop(e);
                    qiniu_ng_err_code::QiniuNgUnknownError
                },
            ),
            HTTPErrorKind::JSONError(_) => qiniu_ng_err(qiniu_ng_err_code::QiniuNgJSONError),
            HTTPErrorKind::MaliciousResponse => qiniu_ng_err(qiniu_ng_err_code::QiniuNgUnknownError),
            HTTPErrorKind::UnexpectedRedirect => qiniu_ng_err(qiniu_ng_err_code::QiniuNgUnexpectedRedirectError),
            HTTPErrorKind::UserCanceled => qiniu_ng_err(qiniu_ng_err_code::QiniuNgUserCanceled),
            HTTPErrorKind::IOError(e) => e.into(),
            HTTPErrorKind::UnknownError(_) => qiniu_ng_err(qiniu_ng_err_code::QiniuNgUnknownError),
            HTTPErrorKind::ResponseStatusCodeError(status_code, _) => qiniu_ng_err(
                qiniu_ng_err_code::QiniuNgResponseStatusCodeError(status_code.to_owned()),
            ),
        }
    }
}

impl From<&UploadError> for qiniu_ng_err {
    fn from(err: &UploadError) -> Self {
        match err {
            UploadError::IOError(err) => err.into(),
            UploadError::QiniuError(err) => err.into(),
        }
    }
}

impl From<&DropBucketError> for qiniu_ng_err {
    fn from(err: &DropBucketError) -> Self {
        match err {
            DropBucketError::CannotDropNonEmptyBucket => {
                qiniu_ng_err(qiniu_ng_err_code::QiniuNgCannotDropNonEmptyBucket)
            }
            DropBucketError::HTTPError(e) => e.into(),
        }
    }
}

impl From<&UploadTokenParseError> for qiniu_ng_err {
    fn from(err: &UploadTokenParseError) -> Self {
        qiniu_ng_err(qiniu_ng_err_code::QiniuNgInvalidUploadToken(match err {
            UploadTokenParseError::InvalidUploadTokenFormat => QiniuNgInvalidUploadTokenCode::InvalidUploadTokenFormat,
            UploadTokenParseError::Base64DecodeError(_) => QiniuNgInvalidUploadTokenCode::Base64DecodeError,
            UploadTokenParseError::JSONDecodeError(_) => QiniuNgInvalidUploadTokenCode::JSONDecodeError,
        }))
    }
}

impl From<&mime::FromStrError> for qiniu_ng_err {
    fn from(_err: &mime::FromStrError) -> Self {
        qiniu_ng_err(qiniu_ng_err_code::QiniuNgBadMIMEType)
    }
}

impl From<&PersistentError> for qiniu_ng_err {
    fn from(err: &PersistentError) -> Self {
        match err {
            PersistentError::JSONError(_) => qiniu_ng_err(qiniu_ng_err_code::QiniuNgJSONError),
            PersistentError::IOError(ref err) => err.into(),
        }
    }
}

impl From<&serde_json::Error> for qiniu_ng_err {
    fn from(_err: &serde_json::Error) -> Self {
        qiniu_ng_err(qiniu_ng_err_code::QiniuNgJSONError)
    }
}
