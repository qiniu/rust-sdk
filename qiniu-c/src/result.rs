#[cfg(any(feature = "use-libcurl"))]
use curl_sys::CURLcode;

use crate::utils::{qiniu_ng_string_free, qiniu_ng_string_t};
use qiniu_ng::{
    http::{domains_manager::PersistentError, Error as HTTPError, ErrorKind as HTTPErrorKind},
    storage::{manager::DropBucketError, upload_token::UploadTokenParseError, uploader::UploadError},
};
use std::io::Error as IOError;

#[repr(C)]
#[derive(Copy, Clone)]
pub enum qiniu_ng_invalid_upload_token_error_kind_t {
    ValidUploadToken,
    InvalidUploadTokenFormat,
    Base64DecodeError(qiniu_ng_string_t),
    JSONDecodeError(qiniu_ng_string_t),
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_invalid_upload_token_error_t(qiniu_ng_invalid_upload_token_error_kind_t);

#[repr(C)]
#[derive(Copy, Clone)]
pub enum qiniu_ng_err_kind_t {
    NoError,
    QiniuNgOsError(i32),
    QiniuNgIoError(qiniu_ng_string_t),
    QiniuNgUnexpectedRedirectError,
    QiniuNgUserCanceled,
    QiniuNgJSONError(qiniu_ng_string_t),
    QiniuNgResponseStatusCodeError(u16, qiniu_ng_string_t),
    QiniuNgUnknownError(qiniu_ng_string_t),

    #[cfg(any(feature = "use-libcurl"))]
    QiniuNgCurlError(CURLcode),
    /* Particular error */
    QiniuNgCannotDropNonEmptyBucket,
    QiniuNgInvalidUploadToken(qiniu_ng_invalid_upload_token_error_t),
    QiniuNgBadMIMEType(qiniu_ng_string_t),
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_err_t(qiniu_ng_err_kind_t);

#[no_mangle]
pub extern "C" fn qiniu_ng_err_os_error_extract(err: &mut qiniu_ng_err_t, code: *mut i32) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::QiniuNgOsError(os_error_code) => {
            if let Some(code) = unsafe { code.as_mut() } {
                *code = os_error_code;
            }
            err.0 = qiniu_ng_err_kind_t::NoError;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_io_error_extract(err: &mut qiniu_ng_err_t, description: *mut qiniu_ng_string_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::QiniuNgIoError(error_description) => {
            if let Some(description) = unsafe { description.as_mut() } {
                *description = error_description;
            } else {
                qiniu_ng_string_free(error_description);
            }
            err.0 = qiniu_ng_err_kind_t::NoError;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_unknown_error_extract(
    err: &mut qiniu_ng_err_t,
    description: *mut qiniu_ng_string_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::QiniuNgUnknownError(error_description) => {
            if let Some(description) = unsafe { description.as_mut() } {
                *description = error_description;
            } else {
                qiniu_ng_string_free(error_description);
            }
            err.0 = qiniu_ng_err_kind_t::NoError;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_unexpected_redirect_error_extract(err: &mut qiniu_ng_err_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::QiniuNgUnexpectedRedirectError => {
            err.0 = qiniu_ng_err_kind_t::NoError;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_user_canceled_error_extract(err: &mut qiniu_ng_err_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::QiniuNgUserCanceled => {
            err.0 = qiniu_ng_err_kind_t::NoError;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_json_error_extract(
    err: &mut qiniu_ng_err_t,
    description: *mut qiniu_ng_string_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::QiniuNgJSONError(error_description) => {
            if let Some(description) = unsafe { description.as_mut() } {
                *description = error_description;
            } else {
                qiniu_ng_string_free(error_description);
            }
            err.0 = qiniu_ng_err_kind_t::NoError;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_response_status_code_error_extract(
    err: &mut qiniu_ng_err_t,
    status_code: *mut u16,
    error: *mut qiniu_ng_string_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::QiniuNgResponseStatusCodeError(code, error_description) => {
            if let Some(status_code) = unsafe { status_code.as_mut() } {
                *status_code = code;
            }
            if let Some(error) = unsafe { error.as_mut() } {
                *error = error_description;
            } else {
                qiniu_ng_string_free(error_description);
            }
            err.0 = qiniu_ng_err_kind_t::NoError;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_drop_non_empty_bucket_error_extract(err: &mut qiniu_ng_err_t) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::QiniuNgCannotDropNonEmptyBucket => {
            err.0 = qiniu_ng_err_kind_t::NoError;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_bad_mime_type_error_extract(
    err: &mut qiniu_ng_err_t,
    description: *mut qiniu_ng_string_t,
) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::QiniuNgBadMIMEType(error_description) => {
            if let Some(description) = unsafe { description.as_mut() } {
                *description = error_description;
            } else {
                qiniu_ng_string_free(error_description);
            }
            err.0 = qiniu_ng_err_kind_t::NoError;
            true
        }
        _ => false,
    }
}

#[cfg(any(feature = "use-libcurl"))]
#[no_mangle]
pub extern "C" fn qiniu_ng_err_curl_error_extract(err: &mut qiniu_ng_err_t, code: *mut CURLcode) -> bool {
    match err.0 {
        qiniu_ng_err_kind_t::QiniuNgCurlError(curl_code) => {
            if let Some(code) = unsafe { code.as_mut() } {
                *code = curl_code;
            }
            err.0 = qiniu_ng_err_kind_t::NoError;
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
        qiniu_ng_err_kind_t::QiniuNgInvalidUploadToken(mut e) => {
            if let Some(upload_token_error) = unsafe { upload_token_error.as_mut() } {
                *upload_token_error = e;
            } else {
                qiniu_ng_invalid_upload_token_error_ignore(&mut e);
            }
            err.0 = qiniu_ng_err_kind_t::NoError;
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
        qiniu_ng_invalid_upload_token_error_kind_t::InvalidUploadTokenFormat => {
            err.0 = qiniu_ng_invalid_upload_token_error_kind_t::ValidUploadToken;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_base64_error_extract(
    err: &mut qiniu_ng_invalid_upload_token_error_t,
    error: *mut qiniu_ng_string_t,
) -> bool {
    match err.0 {
        qiniu_ng_invalid_upload_token_error_kind_t::Base64DecodeError(error_description) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = error_description;
            } else {
                qiniu_ng_string_free(error_description);
            }
            err.0 = qiniu_ng_invalid_upload_token_error_kind_t::ValidUploadToken;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_invalid_upload_token_json_error_extract(
    err: &mut qiniu_ng_invalid_upload_token_error_t,
    error: *mut qiniu_ng_string_t,
) -> bool {
    match err.0 {
        qiniu_ng_invalid_upload_token_error_kind_t::JSONDecodeError(error_description) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = error_description;
            } else {
                qiniu_ng_string_free(error_description);
            }
            err.0 = qiniu_ng_invalid_upload_token_error_kind_t::ValidUploadToken;
            true
        }
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_err_ignore(err: &mut qiniu_ng_err_t) {
    match err.0 {
        qiniu_ng_err_kind_t::QiniuNgIoError(error_description) => qiniu_ng_string_free(error_description),
        qiniu_ng_err_kind_t::QiniuNgUnknownError(error_description) => qiniu_ng_string_free(error_description),
        qiniu_ng_err_kind_t::QiniuNgJSONError(error_description) => qiniu_ng_string_free(error_description),
        qiniu_ng_err_kind_t::QiniuNgResponseStatusCodeError(_, error_description) => {
            qiniu_ng_string_free(error_description)
        }
        qiniu_ng_err_kind_t::QiniuNgInvalidUploadToken(mut err) => qiniu_ng_invalid_upload_token_error_ignore(&mut err),
        qiniu_ng_err_kind_t::QiniuNgBadMIMEType(error_description) => qiniu_ng_string_free(error_description),
        _ => {}
    }
    err.0 = qiniu_ng_err_kind_t::NoError;
}

#[no_mangle]
pub extern "C" fn qiniu_ng_invalid_upload_token_error_ignore(err: &mut qiniu_ng_invalid_upload_token_error_t) {
    match err.0 {
        qiniu_ng_invalid_upload_token_error_kind_t::Base64DecodeError(error_description) => {
            qiniu_ng_string_free(error_description)
        }
        qiniu_ng_invalid_upload_token_error_kind_t::JSONDecodeError(error_description) => {
            qiniu_ng_string_free(error_description)
        }
        _ => {}
    }
    err.0 = qiniu_ng_invalid_upload_token_error_kind_t::ValidUploadToken;
}

impl From<&IOError> for qiniu_ng_err_t {
    fn from(err: &IOError) -> Self {
        qiniu_ng_err_t(err.raw_os_error().map_or_else(
            || {
                qiniu_ng_err_kind_t::QiniuNgIoError(unsafe {
                    qiniu_ng_string_t::from_string_unchecked(err.to_string())
                })
            },
            qiniu_ng_err_kind_t::QiniuNgOsError,
        ))
    }
}

impl From<&HTTPError> for qiniu_ng_err_t {
    fn from(err: &HTTPError) -> Self {
        match err.error_kind() {
            HTTPErrorKind::HTTPCallerError(e) => qiniu_ng_err_t(
                #[cfg(any(feature = "use-libcurl"))]
                {
                    e.inner().downcast_ref::<curl::Error>().map_or_else(
                        || {
                            qiniu_ng_err_kind_t::QiniuNgUnknownError(unsafe {
                                qiniu_ng_string_t::from_string_unchecked(err.to_string())
                            })
                        },
                        |e| qiniu_ng_err_kind_t::QiniuNgCurlError(e.code()),
                    )
                },
                #[cfg(not(feature = "use-libcurl"))]
                {
                    qiniu_ng_err_kind_t::QiniuNgUnknownError(qiniu_ng_string_t::from_str_unchecked(
                        "Unrecognized HTTP Error kind",
                    ))
                },
            ),
            HTTPErrorKind::JSONError(e) => qiniu_ng_err_t(qiniu_ng_err_kind_t::QiniuNgJSONError(unsafe {
                qiniu_ng_string_t::from_string_unchecked(e.to_string())
            })),
            HTTPErrorKind::MaliciousResponse => qiniu_ng_err_t(qiniu_ng_err_kind_t::QiniuNgUnknownError(unsafe {
                qiniu_ng_string_t::from_str_unchecked("Malicious HTTP Response, please try HTTPs protocol")
            })),
            HTTPErrorKind::UnexpectedRedirect => qiniu_ng_err_t(qiniu_ng_err_kind_t::QiniuNgUnexpectedRedirectError),
            HTTPErrorKind::UserCanceled => qiniu_ng_err_t(qiniu_ng_err_kind_t::QiniuNgUserCanceled),
            HTTPErrorKind::IOError(e) => e.into(),
            HTTPErrorKind::UnknownError(e) => qiniu_ng_err_t(qiniu_ng_err_kind_t::QiniuNgUnknownError(unsafe {
                qiniu_ng_string_t::from_string_unchecked(e.to_string())
            })),
            HTTPErrorKind::ResponseStatusCodeError(status_code, error_description) => qiniu_ng_err_t(
                qiniu_ng_err_kind_t::QiniuNgResponseStatusCodeError(status_code.to_owned(), unsafe {
                    qiniu_ng_string_t::from_str_unchecked(error_description)
                }),
            ),
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
                qiniu_ng_err_t(qiniu_ng_err_kind_t::QiniuNgCannotDropNonEmptyBucket)
            }
            DropBucketError::HTTPError(e) => e.into(),
        }
    }
}

impl From<&UploadTokenParseError> for qiniu_ng_err_t {
    fn from(err: &UploadTokenParseError) -> Self {
        qiniu_ng_err_t(qiniu_ng_err_kind_t::QiniuNgInvalidUploadToken(
            qiniu_ng_invalid_upload_token_error_t(match err {
                UploadTokenParseError::InvalidUploadTokenFormat => {
                    qiniu_ng_invalid_upload_token_error_kind_t::InvalidUploadTokenFormat
                }
                UploadTokenParseError::Base64DecodeError(e) => {
                    qiniu_ng_invalid_upload_token_error_kind_t::Base64DecodeError(unsafe {
                        qiniu_ng_string_t::from_string_unchecked(e.to_string())
                    })
                }
                UploadTokenParseError::JSONDecodeError(e) => {
                    qiniu_ng_invalid_upload_token_error_kind_t::JSONDecodeError(unsafe {
                        qiniu_ng_string_t::from_string_unchecked(e.to_string())
                    })
                }
            }),
        ))
    }
}

impl From<&mime::FromStrError> for qiniu_ng_err_t {
    fn from(e: &mime::FromStrError) -> Self {
        qiniu_ng_err_t(qiniu_ng_err_kind_t::QiniuNgBadMIMEType(unsafe {
            qiniu_ng_string_t::from_string_unchecked(e.to_string())
        }))
    }
}

impl From<&PersistentError> for qiniu_ng_err_t {
    fn from(err: &PersistentError) -> Self {
        match err {
            PersistentError::JSONError(e) => qiniu_ng_err_t(qiniu_ng_err_kind_t::QiniuNgJSONError(unsafe {
                qiniu_ng_string_t::from_string_unchecked(e.to_string())
            })),
            PersistentError::IOError(ref err) => err.into(),
        }
    }
}

impl From<&serde_json::Error> for qiniu_ng_err_t {
    fn from(err: &serde_json::Error) -> Self {
        qiniu_ng_err_t(qiniu_ng_err_kind_t::QiniuNgJSONError(unsafe {
            qiniu_ng_string_t::from_string_unchecked(err.to_string())
        }))
    }
}
