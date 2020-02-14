use crate::{
    config::qiniu_ng_config_t,
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr},
    utils::{qiniu_ng_str_list_t, qiniu_ng_str_t},
};
use libc::{c_void, size_t};
use qiniu_ng::{
    storage::uploader::{UploadPolicy, UploadPolicyBuilder, UploadToken},
    Config, Credential,
};
use std::{
    mem::transmute,
    ptr::null_mut,
    time::{Duration, SystemTime},
};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_upload_policy_builder_t(*mut c_void);

impl Default for qiniu_ng_upload_policy_builder_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_upload_policy_builder_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_upload_policy_builder_t> for Option<Box<UploadPolicyBuilder<'_>>> {
    fn from(builder: qiniu_ng_upload_policy_builder_t) -> Self {
        if builder.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(builder)) })
        }
    }
}

impl From<Option<Box<UploadPolicyBuilder<'_>>>> for qiniu_ng_upload_policy_builder_t {
    fn from(builder: Option<Box<UploadPolicyBuilder>>) -> Self {
        builder.map(|builder| builder.into()).unwrap_or_default()
    }
}

impl From<Box<UploadPolicyBuilder<'_>>> for qiniu_ng_upload_policy_builder_t {
    fn from(builder: Box<UploadPolicyBuilder>) -> Self {
        unsafe { transmute(Box::into_raw(builder)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_new_for_bucket(
    bucket: *const qiniu_ng_char_t,
    config: qiniu_ng_config_t,
) -> qiniu_ng_upload_policy_builder_t {
    let config = Option::<Config>::from(config).unwrap();
    Box::new(UploadPolicyBuilder::new_policy_for_bucket(
        unsafe { ucstr::from_ptr(bucket) }.to_string().unwrap(),
        &config,
    ))
    .tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
    .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_new_for_object(
    bucket: *const qiniu_ng_char_t,
    key: *const qiniu_ng_char_t,
    config: qiniu_ng_config_t,
) -> qiniu_ng_upload_policy_builder_t {
    let config = Option::<Config>::from(config).unwrap();
    Box::new(UploadPolicyBuilder::new_policy_for_object(
        unsafe { ucstr::from_ptr(bucket) }.to_string().unwrap(),
        unsafe { ucstr::from_ptr(key) }.to_string().unwrap(),
        &config,
    ))
    .tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
    .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_new_for_objects_with_prefix(
    bucket: *const qiniu_ng_char_t,
    prefix: *const qiniu_ng_char_t,
    config: qiniu_ng_config_t,
) -> qiniu_ng_upload_policy_builder_t {
    let config = Option::<Config>::from(config).unwrap();
    Box::new(UploadPolicyBuilder::new_policy_for_objects_with_prefix(
        unsafe { ucstr::from_ptr(bucket) }.to_string().unwrap(),
        unsafe { ucstr::from_ptr(prefix) }.to_string().unwrap(),
        &config,
    ))
    .tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
    .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_token_lifetime(
    builder: qiniu_ng_upload_policy_builder_t,
    lifetime: u64,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.token_lifetime(Duration::from_secs(lifetime));
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_token_deadline(
    builder: qiniu_ng_upload_policy_builder_t,
    deadline: u64,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.token_deadline(
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(deadline))
            .unwrap(),
    );
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_insert_only(builder: qiniu_ng_upload_policy_builder_t) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.insert_only();
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_overwritable(builder: qiniu_ng_upload_policy_builder_t) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.overwritable();
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_enable_mime_detection(builder: qiniu_ng_upload_policy_builder_t) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.enable_mime_detection();
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_disable_mime_detection(builder: qiniu_ng_upload_policy_builder_t) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.disable_mime_detection();
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_use_infrequent_storage(builder: qiniu_ng_upload_policy_builder_t) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.infrequent_storage();
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_use_normal_storage(builder: qiniu_ng_upload_policy_builder_t) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.normal_storage();
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_return_url(
    builder: qiniu_ng_upload_policy_builder_t,
    return_url: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.return_url(unsafe { ucstr::from_ptr(return_url) }.to_string().unwrap());
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_return_body(
    builder: qiniu_ng_upload_policy_builder_t,
    return_body: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.return_body(unsafe { ucstr::from_ptr(return_body) }.to_string().unwrap());
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_callback_urls(
    builder: qiniu_ng_upload_policy_builder_t,
    callback_urls: *const *const qiniu_ng_char_t,
    callback_urls_size: size_t,
    callback_host: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.callback_urls(
        Vec::<String>::with_capacity(callback_urls_size)
            .tap(|urls| {
                for i in 0..callback_urls_size {
                    urls.push(unsafe { ucstr::from_ptr(*callback_urls.add(i)) }.to_string().unwrap());
                }
            })
            .iter()
            .map(|url| url.as_ref())
            .collect::<Box<[_]>>(),
        unsafe { callback_host.as_ref() }
            .map(|callback_host| unsafe { ucstr::from_ptr(callback_host) }.to_string().unwrap())
            .unwrap_or_else(String::new),
    );
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_callback_body(
    builder: qiniu_ng_upload_policy_builder_t,
    body: *const qiniu_ng_char_t,
    body_type: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.callback_body(
        unsafe { ucstr::from_ptr(body) }.to_string().unwrap(),
        unsafe { body_type.as_ref() }
            .map(|body_type| unsafe { ucstr::from_ptr(body_type) }.to_string().unwrap())
            .unwrap_or_else(String::new),
    );
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_save_as_key(
    builder: qiniu_ng_upload_policy_builder_t,
    key: *const qiniu_ng_char_t,
    force: bool,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.save_as(unsafe { ucstr::from_ptr(key) }.to_string().unwrap(), force);
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_file_size_limitation(
    builder: qiniu_ng_upload_policy_builder_t,
    min_file_size: *mut size_t,
    max_file_size: *mut size_t,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    match (unsafe { min_file_size.as_ref() }, unsafe { max_file_size.as_ref() }) {
        (Some(min_file_size), Some(max_file_size)) => {
            *builder = builder.file_size_limitation(min_file_size..=max_file_size);
        }
        (None, Some(max_file_size)) => {
            *builder = builder.file_size_limitation(..=max_file_size);
        }
        (Some(min_file_size), None) => {
            *builder = builder.file_size_limitation(min_file_size..);
        }
        (None, None) => {
            *builder = builder.file_size_limitation(..);
        }
    };
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_mime_types(
    builder: qiniu_ng_upload_policy_builder_t,
    mime_types: *const *const qiniu_ng_char_t,
    mime_types_size: size_t,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.mime_types(
        Vec::<String>::with_capacity(mime_types_size)
            .tap(|m| {
                for i in 0..mime_types_size {
                    m.push(unsafe { ucstr::from_ptr(*mime_types.add(i)) }.to_string().unwrap());
                }
            })
            .iter()
            .map(|m| m.as_ref())
            .collect::<Box<[&str]>>(),
    );
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_object_lifetime(
    builder: qiniu_ng_upload_policy_builder_t,
    lifetime: u64,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.object_lifetime(Duration::from_secs(lifetime));
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_set_object_deadline(
    builder: qiniu_ng_upload_policy_builder_t,
    deadline: u64,
) {
    let mut builder = Option::<Box<UploadPolicyBuilder>>::from(builder).unwrap();
    *builder = builder.object_deadline(
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(deadline))
            .unwrap(),
    );
    let _ = qiniu_ng_upload_policy_builder_t::from(builder);
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_upload_policy_t(*mut c_void);

impl Default for qiniu_ng_upload_policy_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_upload_policy_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<qiniu_ng_upload_policy_t> for Option<Box<UploadPolicy<'_>>> {
    fn from(upload_policy: qiniu_ng_upload_policy_t) -> Self {
        if upload_policy.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(upload_policy)) })
        }
    }
}

impl From<Option<Box<UploadPolicy<'_>>>> for qiniu_ng_upload_policy_t {
    fn from(upload_policy: Option<Box<UploadPolicy>>) -> Self {
        upload_policy
            .map(|upload_policy| upload_policy.into())
            .unwrap_or_default()
    }
}

impl From<Box<UploadPolicy<'_>>> for qiniu_ng_upload_policy_t {
    fn from(upload_policy: Box<UploadPolicy>) -> Self {
        unsafe { transmute(Box::into_raw(upload_policy)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_build(
    builder_ptr: *mut qiniu_ng_upload_policy_builder_t,
) -> qiniu_ng_upload_policy_t {
    let builder_ptr = unsafe { builder_ptr.as_mut() }.unwrap();
    let builder = Option::<Box<UploadPolicyBuilder>>::from(*builder_ptr).unwrap();
    *builder_ptr = qiniu_ng_upload_policy_builder_t::default();
    Box::new(builder.build()).into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_free(builder: *mut qiniu_ng_upload_policy_builder_t) {
    if let Some(builder) = unsafe { builder.as_mut() } {
        let _ = Option::<Box<UploadPolicyBuilder>>::from(*builder);
        *builder = qiniu_ng_upload_policy_builder_t::default();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_builder_is_freed(builder: qiniu_ng_upload_policy_builder_t) -> bool {
    builder.is_null()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_bucket(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.bucket()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_key(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.key()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_prefixal_scope(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.use_prefixal_object_key().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_insert_only(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.is_insert_only().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_overwritable(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.is_overwritable().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_mime_detection_enabled(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.mime_detection_enabled().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_token_lifetime(
    upload_policy: qiniu_ng_upload_policy_t,
    lifetime: *mut u64,
) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    if let Some(token_lifetime) = upload_policy.token_lifetime() {
        if let Some(lifetime) = unsafe { lifetime.as_mut() } {
            *lifetime = token_lifetime.as_secs()
        }
        true
    } else {
        false
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_token_deadline(
    upload_policy: qiniu_ng_upload_policy_t,
    deadline: *mut u64,
) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    if let Some(token_deadline) = upload_policy.token_deadline() {
        if let Some(deadline) = unsafe { deadline.as_mut() } {
            *deadline = token_deadline.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        }
        true
    } else {
        false
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_return_url(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.return_url()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_return_body(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.return_body()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_callback_urls(
    upload_policy: qiniu_ng_upload_policy_t,
) -> qiniu_ng_str_list_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe {
        qiniu_ng_str_list_t::from_optional_str_slice_unchecked(
            upload_policy
                .callback_urls()
                .map(|urls| urls.collect::<Box<[&str]>>())
                .as_ref()
                .map(|urls| urls.as_ref()),
        )
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_callback_host(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.callback_host()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_callback_body(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.callback_body()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_callback_body_type(
    upload_policy: qiniu_ng_upload_policy_t,
) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.callback_body_type()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_save_key(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(upload_policy.save_key()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_save_key_forced(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.is_save_key_forced().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_file_size_limitation(
    upload_policy: qiniu_ng_upload_policy_t,
    min_file_size: *mut size_t,
    max_file_size: *mut size_t,
) -> u8 {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    let mut return_value = 0u8;
    let (file_size_min, file_size_max) = upload_policy.file_size_limitation();
    if let Some(file_size_min) = file_size_min {
        if let Some(min_file_size) = unsafe { min_file_size.as_mut() } {
            *min_file_size = file_size_min;
            return_value |= 0b10;
        }
    }
    if let Some(file_size_max) = file_size_max {
        if let Some(max_file_size) = unsafe { max_file_size.as_mut() } {
            *max_file_size = file_size_max;
            return_value |= 0b01;
        }
    }
    let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    return_value
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_mime_types(
    upload_policy: qiniu_ng_upload_policy_t,
) -> qiniu_ng_str_list_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe {
        qiniu_ng_str_list_t::from_optional_str_slice_unchecked(
            upload_policy
                .mime_types()
                .map(|types| types.collect::<Box<[&str]>>())
                .as_ref()
                .map(|types| types.as_ref()),
        )
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_normal_storage_used(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.is_normal_storage_used().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_infrequent_storage_used(upload_policy: qiniu_ng_upload_policy_t) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    upload_policy.is_infrequent_storage_used().tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_object_lifetime(
    upload_policy: qiniu_ng_upload_policy_t,
    lifetime: *mut u64,
) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    if let Some(object_lifetime) = upload_policy.object_lifetime() {
        if let Some(lifetime) = unsafe { lifetime.as_mut() } {
            *lifetime = object_lifetime.as_secs()
        }
        true
    } else {
        false
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_get_object_deadline(
    upload_policy: qiniu_ng_upload_policy_t,
    deadline: *mut u64,
) -> bool {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    if let Some(object_deadline) = upload_policy.object_deadline() {
        if let Some(deadline) = unsafe { deadline.as_mut() } {
            *deadline = object_deadline
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
        true
    } else {
        false
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_as_json(upload_policy: qiniu_ng_upload_policy_t) -> qiniu_ng_str_t {
    let upload_policy = Option::<Box<UploadPolicy>>::from(upload_policy).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(upload_policy.as_json()) }.tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(upload_policy);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_from_json(
    json: *const qiniu_ng_char_t,
    upload_policy: *mut qiniu_ng_upload_policy_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    match UploadPolicy::from_json(&unsafe { ucstr::from_ptr(json) }.to_string().unwrap()) {
        Ok(policy) => {
            if let Some(upload_policy) = unsafe { upload_policy.as_mut() } {
                *upload_policy = Box::new(policy).into();
            }
            true
        }
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_free(policy: *mut qiniu_ng_upload_policy_t) {
    if let Some(policy) = unsafe { policy.as_mut() } {
        let _ = Option::<Box<UploadPolicy>>::from(*policy);
        *policy = qiniu_ng_upload_policy_t::default();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_policy_is_freed(policy: qiniu_ng_upload_policy_t) -> bool {
    policy.is_null()
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

impl From<qiniu_ng_upload_token_t> for Option<Box<UploadToken<'_>>> {
    fn from(upload_token: qiniu_ng_upload_token_t) -> Self {
        if upload_token.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(upload_token)) })
        }
    }
}

impl From<Option<Box<UploadToken<'_>>>> for qiniu_ng_upload_token_t {
    fn from(upload_token: Option<Box<UploadToken>>) -> Self {
        upload_token.map(|upload_token| upload_token.into()).unwrap_or_default()
    }
}

impl From<Box<UploadToken<'_>>> for qiniu_ng_upload_token_t {
    fn from(upload_token: Box<UploadToken>) -> Self {
        unsafe { transmute(Box::into_raw(upload_token)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_new_from_policy_builder(
    policy_builder_ptr: *mut qiniu_ng_upload_policy_builder_t,
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
) -> qiniu_ng_upload_token_t {
    let policy_builder_ptr = unsafe { policy_builder_ptr.as_mut() }.unwrap();
    let policy_builder = Option::<Box<UploadPolicyBuilder>>::from(*policy_builder_ptr).unwrap();
    Box::new(UploadToken::from_policy(
        policy_builder.build(),
        Credential::new(
            unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
            unsafe { ucstr::from_ptr(secret_key) }.to_string().unwrap(),
        ),
    ))
    .tap(|_| {
        *policy_builder_ptr = qiniu_ng_upload_policy_builder_t::default();
    })
    .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_new_from_policy(
    policy: qiniu_ng_upload_policy_t,
    access_key: *const qiniu_ng_char_t,
    secret_key: *const qiniu_ng_char_t,
) -> qiniu_ng_upload_token_t {
    let policy = Option::<Box<UploadPolicy>>::from(policy).unwrap();
    Box::new(UploadToken::from_policy(
        policy.as_ref().to_owned(),
        Credential::new(
            unsafe { ucstr::from_ptr(access_key) }.to_string().unwrap(),
            unsafe { ucstr::from_ptr(secret_key) }.to_string().unwrap(),
        ),
    ))
    .tap(|_| {
        let _ = qiniu_ng_upload_policy_t::from(policy);
    })
    .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_new_from_token(token: *const qiniu_ng_char_t) -> qiniu_ng_upload_token_t {
    Box::new(UploadToken::from_token(
        unsafe { ucstr::from_ptr(token) }.to_string().unwrap(),
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
pub extern "C" fn qiniu_ng_upload_token_get_access_key(
    upload_token: qiniu_ng_upload_token_t,
    access_key: *mut qiniu_ng_str_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let upload_token = Option::<Box<UploadToken>>::from(upload_token).unwrap();
    match upload_token.access_key() {
        Ok(ak) => {
            if let Some(access_key) = unsafe { access_key.as_mut() } {
                *access_key = unsafe { qiniu_ng_str_t::from_str_unchecked(ak) };
            }
            true
        }
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            false
        }
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_token_t::from(upload_token);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_get_token(upload_token: qiniu_ng_upload_token_t) -> qiniu_ng_str_t {
    let upload_token = Option::<Box<UploadToken>>::from(upload_token).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(upload_token.token()) }.tap(|_| {
        let _ = qiniu_ng_upload_token_t::from(upload_token);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_upload_token_get_policy(
    token: qiniu_ng_upload_token_t,
    policy: *mut qiniu_ng_upload_policy_t,
    err: *mut qiniu_ng_err_t,
) -> bool {
    let token = Option::<Box<UploadToken>>::from(token).unwrap();
    match token.policy() {
        Ok(upload_policy) => {
            if let Some(policy) = unsafe { policy.as_mut() } {
                *policy = Box::new(upload_policy.into_owned()).into();
            }
            true
        }
        Err(ref e) => {
            if let Some(err) = unsafe { err.as_mut() } {
                *err = e.into();
            }
            false
        }
    }
    .tap(|_| {
        let _ = qiniu_ng_upload_token_t::from(token);
    })
}
