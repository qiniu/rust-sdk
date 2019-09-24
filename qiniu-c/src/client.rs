use crate::config::qiniu_ng_config;
use libc::{c_char, c_void};
use qiniu::{Client, Config, ConfigBuilder};
use std::{ffi::CStr, mem, time::Duration};

#[repr(C)]
pub struct qiniu_ng_client_t(*mut c_void);

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_client_new(
    access_key: *const c_char,
    secret_key: *const c_char,
    config: *const qiniu_ng_config,
) -> qiniu_ng_client_t {
    let client = Box::new(Client::new(
        CStr::from_ptr(access_key).to_string_lossy(),
        CStr::from_ptr(secret_key).to_string_lossy(),
        make_config(config),
    ));
    mem::transmute(Box::into_raw(client))
}

#[no_mangle]
pub unsafe extern "C" fn qiniu_ng_client_free(client: qiniu_ng_client_t) {
    Box::from_raw(mem::transmute::<_, *mut Client>(client));
}

unsafe fn make_config(config: *const qiniu_ng_config) -> Config {
    ConfigBuilder::default()
        .use_https((*config).use_https)
        .upload_token_lifetime(Duration::from_secs((*config).upload_token_lifetime))
        .batch_max_operation_size((*config).batch_max_operation_size)
        .upload_threshold((*config).upload_threshold)
        .upload_chunk_size((*config).upload_chunk_size)
        .http_request_retries((*config).http_request_retries)
        .http_request_retry_delay(Duration::from_secs((*config).http_request_retry_delay))
        .host_freeze_duration(Duration::from_secs((*config).host_freeze_duration))
        .build()
        .unwrap()
}
