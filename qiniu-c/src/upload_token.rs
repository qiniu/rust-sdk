use crate::{
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr, UCString},
};
use libc::{c_ulonglong, c_void, size_t};
use once_cell::sync::OnceCell;
use qiniu_ng::{
    storage::{
        upload_policy::{UploadPolicy as QiniuUploadPolicy, UploadPolicyBuilder as QiniuUploadPolicyBuilder},
        upload_token::{UploadToken as QiniuUploadToken, UploadTokenParseResult as QiniuUploadTokenParseResult},
    },
    Credential,
};
use std::{
    mem::transmute,
    ptr::{null, null_mut},
    slice,
    time::{Duration, SystemTime},
};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_upload_policy_t {
    bucket: *const qiniu_ng_char_t,
    key: *const qiniu_ng_char_t,
    prefixal: bool,
    insert_only: bool,
    mime_detection: bool,
    deadline: c_ulonglong,

    return_url: *const qiniu_ng_char_t,
    return_body: *const qiniu_ng_char_t,

    callback_urls: *const *const qiniu_ng_char_t,
    callback_urls_len: size_t,
    callback_host: *const qiniu_ng_char_t,
    callback_body: *const qiniu_ng_char_t,
    callback_body_type: *const qiniu_ng_char_t,

    save_key: *const qiniu_ng_char_t,
    force_save_key: bool,

    file_size_min: *const size_t,
    file_size_max: *const size_t,

    mime: *const *const qiniu_ng_char_t,
    mime_len: size_t,

    infrequent_storage: bool,
    object_lifetime: *const c_ulonglong,
}

struct UploadPolicy {
    bucket: Option<Box<ucstr>>,
    key: Option<Box<ucstr>>,
    prefixal: bool,
    insert_only: bool,
    mime_detection: bool,
    deadline: Option<u64>,

    return_url: Option<Box<ucstr>>,
    return_body: Option<Box<ucstr>>,

    callback_urls_storage: Option<Box<[Box<ucstr>]>>,
    callback_urls: Option<Box<[*const qiniu_ng_char_t]>>,
    callback_host: Option<Box<ucstr>>,
    callback_body: Option<Box<ucstr>>,
    callback_body_type: Option<Box<ucstr>>,

    save_key: Option<Box<ucstr>>,
    force_save_key: bool,

    file_size_min: Option<usize>,
    file_size_max: Option<usize>,

    mime_storage: Option<Box<[Box<ucstr>]>>,
    mime: Option<Box<[*const qiniu_ng_char_t]>>,
    infrequent_storage: bool,
    object_lifetime: Option<u64>,
}

impl From<&qiniu_ng_upload_policy_t> for UploadPolicy {
    fn from(policy: &qiniu_ng_upload_policy_t) -> Self {
        let mut policy = UploadPolicy {
            bucket: unsafe { policy.bucket.as_ref() }.map(|s| unsafe { UCString::from_ptr(s) }.into_boxed_ucstr()),
            key: unsafe { policy.key.as_ref() }.map(|s| unsafe { UCString::from_ptr(s) }.into_boxed_ucstr()),
            prefixal: policy.prefixal,
            insert_only: policy.insert_only,
            mime_detection: policy.mime_detection,
            deadline: Some(policy.deadline),
            return_url: unsafe { policy.return_url.as_ref() }
                .map(|s| unsafe { UCString::from_ptr(s) }.into_boxed_ucstr()),
            return_body: unsafe { policy.return_body.as_ref() }
                .map(|s| unsafe { UCString::from_ptr(s) }.into_boxed_ucstr()),
            callback_urls_storage: unsafe { policy.callback_urls.as_ref() }.map(|callback_urls| {
                unsafe { slice::from_raw_parts(callback_urls, policy.callback_urls_len) }
                    .iter()
                    .map(|&ptr| {
                        unsafe { ptr.as_ref() }
                            .map(|s| unsafe { UCString::from_ptr(s) }.to_owned().into_boxed_ucstr())
                            .unwrap()
                    })
                    .collect()
            }),
            callback_urls: Default::default(),
            callback_host: unsafe { policy.callback_host.as_ref() }
                .map(|s| unsafe { UCString::from_ptr(s) }.into_boxed_ucstr()),
            callback_body: unsafe { policy.callback_body.as_ref() }
                .map(|s| unsafe { UCString::from_ptr(s) }.into_boxed_ucstr()),
            callback_body_type: unsafe { policy.callback_body_type.as_ref() }
                .map(|s| unsafe { UCString::from_ptr(s) }.into_boxed_ucstr()),
            save_key: unsafe { policy.save_key.as_ref() }.map(|s| unsafe { UCString::from_ptr(s) }.into_boxed_ucstr()),
            force_save_key: policy.force_save_key,
            file_size_min: unsafe { policy.file_size_min.as_ref() }.copied(),
            file_size_max: unsafe { policy.file_size_max.as_ref() }.copied(),
            mime_storage: unsafe { policy.mime.as_ref() }.map(|mime| {
                unsafe { slice::from_raw_parts(mime, policy.mime_len) }
                    .iter()
                    .map(|&ptr| {
                        unsafe { ptr.as_ref() }
                            .map(|s| unsafe { UCString::from_ptr(s) }.into_boxed_ucstr())
                            .unwrap()
                    })
                    .collect()
            }),
            mime: Default::default(),
            infrequent_storage: policy.infrequent_storage,
            object_lifetime: unsafe { policy.object_lifetime.as_ref() }.copied(),
        };
        policy.callback_urls = policy
            .callback_urls_storage
            .as_ref()
            .map(|urls| urls.iter().map(|u| u.as_ptr()).collect());
        policy.mime = policy
            .mime_storage
            .as_ref()
            .map(|mime| mime.iter().map(|m| m.as_ptr()).collect());
        policy
    }
}

impl From<&UploadPolicy> for qiniu_ng_upload_policy_t {
    fn from(policy: &UploadPolicy) -> Self {
        qiniu_ng_upload_policy_t {
            bucket: policy.bucket.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null),
            key: policy.key.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null),
            prefixal: policy.prefixal,
            insert_only: policy.insert_only,
            mime_detection: policy.mime_detection,
            deadline: policy.deadline.unwrap_or(0),
            return_url: policy.return_url.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null),
            return_body: policy.return_body.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null),
            callback_urls: policy
                .callback_urls
                .as_ref()
                .map(|urls| urls.as_ptr())
                .unwrap_or_else(null),
            callback_urls_len: policy.callback_urls.as_ref().map(|urls| urls.len()).unwrap_or(0),
            callback_host: policy.callback_host.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null),
            callback_body: policy.callback_body.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null),
            callback_body_type: policy
                .callback_body_type
                .as_ref()
                .map(|s| s.as_ptr())
                .unwrap_or_else(null),
            save_key: policy.save_key.as_ref().map(|s| s.as_ptr()).unwrap_or_else(null),
            force_save_key: policy.force_save_key,
            file_size_min: policy
                .file_size_min
                .as_ref()
                .map(|d| d as *const size_t)
                .unwrap_or_else(null),
            file_size_max: policy
                .file_size_max
                .as_ref()
                .map(|d| d as *const size_t)
                .unwrap_or_else(null),
            mime: policy.mime.as_ref().map(|mime| mime.as_ptr()).unwrap_or_else(null),
            mime_len: policy.mime.as_ref().map(|mime| mime.len()).unwrap_or(0),
            infrequent_storage: policy.infrequent_storage,
            object_lifetime: policy
                .object_lifetime
                .as_ref()
                .map(|d| d as *const c_ulonglong)
                .unwrap_or_else(null),
        }
    }
}

impl From<&QiniuUploadPolicy<'_>> for UploadPolicy {
    fn from(policy: &QiniuUploadPolicy) -> Self {
        let mut policy = UploadPolicy {
            bucket: policy
                .bucket()
                .map(|s| unsafe { UCString::from_str_unchecked(s) }.into_boxed_ucstr()),
            key: policy
                .key()
                .map(|s| unsafe { UCString::from_str_unchecked(s) }.into_boxed_ucstr()),
            prefixal: policy.prefixal(),
            insert_only: policy.insert_only(),
            mime_detection: policy.mime_detection(),
            deadline: policy
                .deadline()
                .map(|t| t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()),
            return_url: policy
                .return_url()
                .map(|s| unsafe { UCString::from_str_unchecked(s) }.into_boxed_ucstr()),
            return_body: policy
                .return_body()
                .map(|s| unsafe { UCString::from_str_unchecked(s) }.into_boxed_ucstr()),
            callback_urls_storage: policy.callback_urls().map(|iter| {
                iter.map(|s| unsafe { UCString::from_str_unchecked(s) }.into_boxed_ucstr())
                    .collect()
            }),
            callback_urls: Default::default(),
            callback_host: policy
                .callback_host()
                .map(|s| unsafe { UCString::from_str_unchecked(s) }.into_boxed_ucstr()),
            callback_body: policy
                .callback_body()
                .map(|s| unsafe { UCString::from_str_unchecked(s) }.into_boxed_ucstr()),
            callback_body_type: policy
                .callback_body_type()
                .map(|s| unsafe { UCString::from_str_unchecked(s) }.into_boxed_ucstr()),
            save_key: policy
                .save_key()
                .map(|s| unsafe { UCString::from_str_unchecked(s) }.into_boxed_ucstr()),
            force_save_key: policy.force_save_key(),
            file_size_min: policy.file_size().0,
            file_size_max: policy.file_size().1,
            mime_storage: policy.mime().map(|iter| {
                iter.map(|s| unsafe { UCString::from_str_unchecked(s) }.into_boxed_ucstr())
                    .collect()
            }),
            mime: Default::default(),
            infrequent_storage: policy.infrequent_storage(),
            object_lifetime: policy.object_lifetime().map(|d| d.as_secs()),
        };
        policy.callback_urls = policy
            .callback_urls_storage
            .as_ref()
            .map(|urls| urls.iter().map(|u| u.as_ptr()).collect());
        policy.mime = policy
            .mime_storage
            .as_ref()
            .map(|mime| mime.iter().map(|m| m.as_ptr()).collect());
        policy
    }
}

impl<'a> From<&'a UploadPolicyWithParams> for QiniuUploadPolicy<'a> {
    fn from(policy_with_params: &'a UploadPolicyWithParams) -> Self {
        let policy = &policy_with_params.upload_policy;
        let mut policy_builder = match (
            policy.bucket.as_ref(),
            policy.key.as_ref(),
            policy.deadline.map(|deadline| {
                (SystemTime::UNIX_EPOCH + Duration::from_secs(deadline))
                    .duration_since(SystemTime::now())
                    .unwrap_or_else(|_| Duration::from_secs(0))
            }),
        ) {
            (Some(bucket), Some(key), Some(lifetime)) if policy.prefixal => {
                QiniuUploadPolicyBuilder::new_policy_for_objects_with_prefix(
                    bucket.to_string().unwrap(),
                    key.to_string().unwrap(),
                    lifetime,
                )
            }
            (Some(bucket), Some(key), Some(lifetime)) if !policy.prefixal => {
                QiniuUploadPolicyBuilder::new_policy_for_object(
                    bucket.to_string().unwrap(),
                    key.to_string().unwrap(),
                    lifetime,
                )
            }
            (Some(bucket), None, Some(lifetime)) => {
                QiniuUploadPolicyBuilder::new_policy_for_bucket(bucket.to_string().unwrap(), lifetime)
            }
            _ => panic!("Invalid upload token, bucket or lifetime is none"),
        };
        if policy.insert_only {
            policy_builder = policy_builder.insert_only();
        } else {
            policy_builder = policy_builder.overwritable();
        }

        if policy.mime_detection {
            policy_builder = policy_builder.enable_mime_detection();
        } else {
            policy_builder = policy_builder.disable_mime_detection();
        }

        if policy.infrequent_storage {
            policy_builder = policy_builder.infrequent_storage();
        } else {
            policy_builder = policy_builder.normal_storage();
        }

        if let Some(return_url) = policy.return_url.as_ref() {
            policy_builder = policy_builder.return_url(return_url.to_string().unwrap());
        }

        if let Some(return_body) = policy.return_body.as_ref() {
            policy_builder = policy_builder.return_body(return_body.to_string().unwrap());
        }

        if let Some(callback_urls) = policy.callback_urls_storage.as_ref() {
            policy_builder = policy_builder.callback_urls(
                &callback_urls
                    .iter()
                    .map(|url| url.to_string().unwrap())
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|url| url.as_ref())
                    .collect::<Vec<_>>(),
                policy.callback_host.as_ref().map(|host| host.to_string().unwrap()),
            );

            if let Some(callback_body) = policy.callback_body.as_ref() {
                policy_builder = policy_builder.callback_body(
                    callback_body.to_string().unwrap(),
                    policy.callback_body_type.as_ref().map(|bt| bt.to_string().unwrap()),
                );
            }
        }

        if let Some(save_key) = policy.save_key.as_ref() {
            policy_builder = policy_builder.save_as(save_key.to_string().unwrap(), policy.force_save_key);
        }

        match (policy.file_size_min, policy.file_size_max) {
            (Some(file_size_min), Some(file_size_max)) => {
                policy_builder = policy_builder.file_size(file_size_min..=file_size_max);
            }
            (None, Some(file_size_max)) => {
                policy_builder = policy_builder.file_size(..=file_size_max);
            }
            (Some(file_size_min), None) => {
                policy_builder = policy_builder.file_size(file_size_min..);
            }
            (None, None) => {}
        }

        if let Some(mime) = policy.mime_storage.as_ref() {
            policy_builder = policy_builder.mime(
                &mime
                    .iter()
                    .map(|m| m.to_string().unwrap())
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|m| m.as_ref())
                    .collect::<Vec<_>>(),
            );
        }

        if let Some(lifetime) = policy.object_lifetime {
            policy_builder = policy_builder.object_lifetime(Duration::from_secs(lifetime));
        }
        policy_builder.build()
    }
}

struct UploadToken {
    upload_token: OnceCell<Box<ucstr>>,
    upload_policy_with_params: OnceCell<UploadPolicyWithParams>,
}

struct UploadPolicyWithParams {
    upload_policy: UploadPolicy,
    credential: Option<Credential>,
}

impl From<UploadPolicyWithParams> for UploadToken {
    fn from(upload_policy_with_params: UploadPolicyWithParams) -> Self {
        UploadToken {
            upload_token: OnceCell::new(),
            upload_policy_with_params: OnceCell::from(upload_policy_with_params),
        }
    }
}

impl From<Box<ucstr>> for UploadToken {
    fn from(token: Box<ucstr>) -> Self {
        UploadToken {
            upload_token: OnceCell::from(token),
            upload_policy_with_params: OnceCell::new(),
        }
    }
}

impl UploadToken {
    fn get_upload_token(&self) -> &ucstr {
        self.upload_token.get_or_init(|| {
            let upload_policy_with_params = self.upload_policy_with_params.get().unwrap();
            let policy = QiniuUploadPolicy::from(upload_policy_with_params);
            unsafe {
                UCString::from_string_unchecked(
                    QiniuUploadToken::from_policy(policy, upload_policy_with_params.credential.as_ref().unwrap())
                        .token(),
                )
            }
            .into_boxed_ucstr()
        })
    }

    fn get_upload_policy(&self) -> QiniuUploadTokenParseResult<&UploadPolicy> {
        let upload_policy_with_params: QiniuUploadTokenParseResult<&UploadPolicyWithParams> =
            self.upload_policy_with_params.get_or_try_init(|| {
                let policy: UploadPolicy =
                    QiniuUploadToken::from_token(self.upload_token.get().unwrap().to_string().unwrap())
                        .policy()?
                        .as_ref()
                        .into();
                Ok(UploadPolicyWithParams {
                    upload_policy: policy,
                    credential: None,
                })
            });
        Ok(&upload_policy_with_params?.upload_policy)
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_upload_token_t(*mut c_void);

impl Default for qiniu_ng_upload_token_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_upload_token_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_upload_token_t> for Option<Box<UploadToken>> {
    fn from(upload_token: qiniu_ng_upload_token_t) -> Self {
        if upload_token.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(upload_token)) })
        }
    }
}

impl From<Option<Box<UploadToken>>> for qiniu_ng_upload_token_t {
    fn from(upload_token: Option<Box<UploadToken>>) -> Self {
        upload_token.map(|upload_token| upload_token.into()).unwrap_or_default()
    }
}

impl From<Box<UploadToken>> for qiniu_ng_upload_token_t {
    fn from(upload_token: Box<UploadToken>) -> Self {
        unsafe { transmute(Box::into_raw(upload_token)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_new_upload_token_from_policy(
    policy: *const qiniu_ng_upload_policy_t,
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
) -> qiniu_ng_upload_token_t {
    Box::new(UploadToken::from(UploadPolicyWithParams {
        upload_policy: unsafe { policy.as_ref() }.unwrap().into(),
        credential: Some(Credential::new(
            unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
            unsafe { ucstr::from_ptr(secret_key) }.to_string().unwrap(),
        )),
    }))
    .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_new_upload_token_from_token(token: *const qiniu_ng_char_t) -> qiniu_ng_upload_token_t {
    Box::new(UploadToken::from(
        unsafe { UCString::from_ptr(token) }.into_boxed_ucstr(),
    ))
    .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_free(token: *mut qiniu_ng_upload_token_t) {
    if let Some(token) = unsafe { token.as_mut() } {
        let _ = Option::<Box<UploadToken>>::from(*token);
        *token = qiniu_ng_upload_token_t::default();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_is_freed(token: qiniu_ng_upload_token_t) -> bool {
    token.is_null()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_get_token(token: qiniu_ng_upload_token_t) -> *const qiniu_ng_char_t {
    let token = Option::<Box<UploadToken>>::from(token).unwrap();
    token.get_upload_token().as_ptr().tap(|_| {
        let _ = qiniu_ng_upload_token_t::from(token);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_get_policy(
    token: qiniu_ng_upload_token_t,
    policy: *mut qiniu_ng_upload_policy_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let token = Option::<Box<UploadToken>>::from(token).unwrap();
    match token.get_upload_policy() {
        Ok(upload_policy) => {
            if let Some(policy) = unsafe { policy.as_mut() } {
                *policy = upload_policy.into();
            }
            true
        }
        Err(ref err) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = err.into();
            }
            false
        }
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_token_t::from(token);
    })
}
