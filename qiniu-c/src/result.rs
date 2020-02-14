#[cfg(any(feature = "use-libcurl"))]
use curl_sys::CURLcode;

use crate::utils::{qiniu_ng_str_free, qiniu_ng_str_t};
use matches::matches;
use qiniu_ng::{
    http::{domains_manager::PersistentError, Error as HTTPError, ErrorKind as HTTPErrorKind},
    storage::{
        manager::DropBucketError,
        uploader::{UploadError, UploadTokenParseError},
    },
};
use std::io::Error as IOError;

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_invalid_upload_token_error_kind_t {
    qiniu_ng_invalid_upload_token_error_kind_none,
    qiniu_ng_invalid_upload_token_error_kind_invalid_format,
    qiniu_ng_invalid_upload_token_error_kind_base64_decode_error(qiniu_ng_str_t),
    qiniu_ng_invalid_upload_token_error_kind_json_decode_error(qiniu_ng_str_t),
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_invalid_upload_token_error_t(qiniu_ng_invalid_upload_token_error_kind_t);

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_err_kind_t {
    qiniu_ng_err_kind_none,
    qiniu_ng_err_kind_os_error(i32),
    qiniu_ng_err_kind_io_error(qiniu_ng_str_t),
    qiniu_ng_err_kind_unexpected_redirect_error,
    qiniu_ng_err_kind_user_canceled,
    qiniu_ng_err_kind_json_error(qiniu_ng_str_t),
    qiniu_ng_err_kind_response_status_code_error(u16, qiniu_ng_str_t),
    qiniu_ng_err_kind_unknown_error(qiniu_ng_str_t),

    #[cfg(any(feature = "use-libcurl"))]
    qiniu_ng_err_kind_curl_error(CURLcode),
    /* Particular error */
    qiniu_ng_err_kind_cannot_drop_non_empty_bucket_error,
    qiniu_ng_err_kind_invalid_upload_token_error(qiniu_ng_invalid_upload_token_error_t),
    qiniu_ng_err_kind_bad_mime_type(qiniu_ng_str_t),
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_err_t(qiniu_ng_err_kind_t);

#[no_mangle]
pub extern "C" fn qiniu_ng_err_any_error(err: &mut qiniu_ng_err_t) -> bool {
    !matches!(err.0, qiniu_ng_err_kind_t::qiniu_ng_err_kind_none)
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_os_error_extract(err: &mut qiniu_ng_err_t, code: *mut i32) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_os_error(os_error_code) => {
            if let Some(code) = unsafe { code.as_mut() } {
                *code = os_error_code;
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_io_error_extract(err: &mut qiniu_ng_err_t, description: *mut qiniu_ng_str_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_io_error(mut error_description) => {
            if let Some(description) = unsafe { description.as_mut() } {
                *description = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_unknown_error_extract(
    err: &mut qiniu_ng_err_t,
    description: *mut qiniu_ng_str_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(mut error_description) => {
            if let Some(description) = unsafe { description.as_mut() } {
                *description = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_unexpected_redirect_error_extract(err: &mut qiniu_ng_err_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_unexpected_redirect_error => {
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_user_canceled_error_extract(err: &mut qiniu_ng_err_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_user_canceled => {
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_json_error_extract(err: &mut qiniu_ng_err_t, description: *mut qiniu_ng_str_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_json_error(mut error_description) => {
            if let Some(description) = unsafe { description.as_mut() } {
                *description = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_response_status_code_error_extract(
    err: &mut qiniu_ng_err_t,
    status_code: *mut u16,
    error: *mut qiniu_ng_str_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_response_status_code_error(code, mut error_description) => {
            if let Some(status_code) = unsafe { status_code.as_mut() } {
                *status_code = code;
            }
            if let Some(error) = unsafe { error.as_mut() } {
                *error = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_drop_non_empty_bucket_error_extract(err: &mut qiniu_ng_err_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_cannot_drop_non_empty_bucket_error => {
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_bad_mime_type_error_extract(
    err: &mut qiniu_ng_err_t,
    description: *mut qiniu_ng_str_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_bad_mime_type(mut error_description) => {
            if let Some(description) = unsafe { description.as_mut() } {
                *description = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

#[cfg(any(feature = "use-libcurl"))]
#[no_mangle]
pub extern "C" fn qiniu_ng_err_curl_error_extract(err: &mut qiniu_ng_err_t, code: *mut CURLcode) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_curl_error(curl_code) => {
            if let Some(code) = unsafe { code.as_mut() } {
                *code = curl_code;
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_extract(
    err: &mut qiniu_ng_err_t,
    upload_token_error: *mut qiniu_ng_invalid_upload_token_error_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_invalid_upload_token_error(mut e) => {
            if let Some(upload_token_error) = unsafe { upload_token_error.as_mut() } {
                *upload_token_error = e;
            } else {
                qiniu_ng_err_invalid_upload_token_error_ignore(&mut e);
            }
            err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_format_extract(
    err: &mut qiniu_ng_invalid_upload_token_error_t,
) -> bool {
    match err.0 {
        qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_invalid_format => {
            err.0 = qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_base64_error_extract(
    err: &mut qiniu_ng_invalid_upload_token_error_t,
    error: *mut qiniu_ng_str_t,
) -> bool {
    match err.0 {
        qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_base64_decode_error(
            mut error_description,
        ) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_json_error_extract(
    err: &mut qiniu_ng_invalid_upload_token_error_t,
    error: *mut qiniu_ng_str_t,
) -> bool {
    match err.0 {
        qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_json_decode_error(
            mut error_description,
        ) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = error_description;
            } else {
                qiniu_ng_str_free(&mut error_description);
            }
            err.0 = qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_none;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_ignore(err: &mut qiniu_ng_err_t) {
    match err.0 {
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_io_error(mut desc) => qiniu_ng_str_free(&mut desc),
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(mut desc) => qiniu_ng_str_free(&mut desc),
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_json_error(mut desc) => qiniu_ng_str_free(&mut desc),
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_response_status_code_error(_, mut desc) => qiniu_ng_str_free(&mut desc),
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_invalid_upload_token_error(mut err) => {
            qiniu_ng_err_invalid_upload_token_error_ignore(&mut err)
        }
        qiniu_ng_err_kind_t::qiniu_ng_err_kind_bad_mime_type(mut desc) => qiniu_ng_str_free(&mut desc),
        _ => {}
    }
    err.0 = qiniu_ng_err_kind_t::qiniu_ng_err_kind_none;
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_error_ignore(err: &mut qiniu_ng_invalid_upload_token_error_t) {
    match err.0 {
        qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_base64_decode_error(
            mut desc,
        ) => qiniu_ng_str_free(&mut desc),
        qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_json_decode_error(
            mut desc,
        ) => qiniu_ng_str_free(&mut desc),
        _ => {}
    }
    err.0 = qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_none;
}

impl From<&IOError> for qiniu_ng_err_t {
    fn from(err: &IOError) -> Self {
        qiniu_ng_err_t(
            err.raw_os_error()
                .map(qiniu_ng_err_kind_t::qiniu_ng_err_kind_os_error)
                .unwrap_or_else(|| {
                    qiniu_ng_err_kind_t::qiniu_ng_err_kind_io_error(unsafe {
                        qiniu_ng_str_t::from_string_unchecked(err.to_string())
                    })
                }),
        )
    }
}

impl From<&HTTPError> for qiniu_ng_err_t {
    fn from(err: &HTTPError) -> Self {
        match err.error_kind() {
            HTTPErrorKind::HTTPCallerError(e) => qiniu_ng_err_t(
                #[cfg(any(feature = "use-libcurl"))]
                {
                    e.inner()
                        .downcast_ref::<curl::Error>()
                        .map(|e| qiniu_ng_err_kind_t::qiniu_ng_err_kind_curl_error(e.code()))
                        .unwrap_or_else(|| {
                            qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(unsafe {
                                qiniu_ng_str_t::from_string_unchecked(err.to_string())
                            })
                        })
                },
                #[cfg(not(feature = "use-libcurl"))]
                {
                    qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(qiniu_ng_str_t::from_str_unchecked(
                        "Unrecognized HTTP Error kind",
                    ))
                },
            ),
            HTTPErrorKind::ResponseStatusCodeError(status_code, error_description) => qiniu_ng_err_t(
                qiniu_ng_err_kind_t::qiniu_ng_err_kind_response_status_code_error(status_code.to_owned(), unsafe {
                    qiniu_ng_str_t::from_str_unchecked(error_description)
                }),
            ),
            HTTPErrorKind::IOError(e) => e.into(),
            HTTPErrorKind::JSONError(e) => qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_json_error(unsafe {
                qiniu_ng_str_t::from_string_unchecked(e.to_string())
            })),
            HTTPErrorKind::UnexpectedRedirect => {
                qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_unexpected_redirect_error)
            }
            HTTPErrorKind::MaliciousResponse => {
                qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(unsafe {
                    qiniu_ng_str_t::from_str_unchecked("Malicious HTTP Response, please try HTTPs protocol")
                }))
            }
            HTTPErrorKind::UserCanceled => qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_user_canceled),
            HTTPErrorKind::UnknownError(e) => {
                qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_unknown_error(unsafe {
                    qiniu_ng_str_t::from_string_unchecked(e.to_string())
                }))
            }
        }
    }
}

impl From<&UploadError> for qiniu_ng_err_t {
    fn from(err: &UploadError) -> Self {
        match err {
            UploadError::IOError(err) => err.into(),
            UploadError::QiniuError(err) => err.into(),
        }
    }
}

impl From<&DropBucketError> for qiniu_ng_err_t {
    fn from(err: &DropBucketError) -> Self {
        match err {
            DropBucketError::CannotDropNonEmptyBucket => {
                qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_cannot_drop_non_empty_bucket_error)
            }
            DropBucketError::HTTPError(e) => e.into(),
        }
    }
}

impl From<&UploadTokenParseError> for qiniu_ng_err_t {
    fn from(err: &UploadTokenParseError) -> Self {
        qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_invalid_upload_token_error(
            qiniu_ng_invalid_upload_token_error_t(match err {
                UploadTokenParseError::InvalidUploadTokenFormat => {
                    qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_invalid_format
                }
                UploadTokenParseError::Base64DecodeError(e) => {
                    qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_base64_decode_error(unsafe {
                        qiniu_ng_str_t::from_string_unchecked(e.to_string())
                    })
                }
                UploadTokenParseError::JSONDecodeError(e) => {
                    qiniu_ng_invalid_upload_token_error_kind_t::qiniu_ng_invalid_upload_token_error_kind_json_decode_error(unsafe {
                        qiniu_ng_str_t::from_string_unchecked(e.to_string())
                    })
                }
            }),
        ))
    }
}

impl From<&mime::FromStrError> for qiniu_ng_err_t {
    fn from(e: &mime::FromStrError) -> Self {
        qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_bad_mime_type(unsafe {
            qiniu_ng_str_t::from_string_unchecked(e.to_string())
        }))
    }
}

impl From<&PersistentError> for qiniu_ng_err_t {
    fn from(err: &PersistentError) -> Self {
        match err {
            PersistentError::JSONError(e) => {
                qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_json_error(unsafe {
                    qiniu_ng_str_t::from_string_unchecked(e.to_string())
                }))
            }
            PersistentError::IOError(ref err) => err.into(),
        }
    }
}

impl From<&serde_json::Error> for qiniu_ng_err_t {
    fn from(err: &serde_json::Error) -> Self {
        qiniu_ng_err_t(qiniu_ng_err_kind_t::qiniu_ng_err_kind_json_error(unsafe {
            qiniu_ng_str_t::from_string_unchecked(err.to_string())
        }))
    }
}
