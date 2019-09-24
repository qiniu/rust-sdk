use libc::{c_ulonglong, c_void, size_t};
use qiniu::{Config, ConfigBuilder};
use std::time::Duration;

#[repr(C)]
pub struct qiniu_ng_config_builder_t(pub(crate) *mut c_void);
#[repr(C)]
pub struct qiniu_ng_config_t(pub(crate) *mut c_void);

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_new() -> qiniu_ng_config_builder_t {
    let config_builder = Box::new(ConfigBuilder::default());
    qiniu_ng_config_builder_t(Box::into_raw(config_builder) as usize as *mut c_void)
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_use_https(config_builder: qiniu_ng_config_builder_t) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_builder.0 as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.use_https(true);
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_use_http(config_builder: qiniu_ng_config_builder_t) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_builder.0 as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.use_https(false);
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_token_lifetime(
    config_builder: qiniu_ng_config_builder_t,
    lifetime_secs: c_ulonglong,
) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_builder.0 as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.upload_token_lifetime(Duration::from_secs(lifetime_secs));
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_batch_max_operation_size(
    config_builder: qiniu_ng_config_builder_t,
    operation_size: size_t,
) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_builder.0 as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.batch_max_operation_size(operation_size);
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_threshold(
    config_builder: qiniu_ng_config_builder_t,
    threshold: c_ulonglong,
) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_builder.0 as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.upload_threshold(threshold);
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_chunk_size(
    config_builder: qiniu_ng_config_builder_t,
    chunk_size: size_t,
) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_builder.0 as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.upload_chunk_size(chunk_size);
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_http_request_retries(
    config_builder: qiniu_ng_config_builder_t,
    retries: size_t,
) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_builder.0 as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.http_request_retries(retries);
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_http_request_retry_delay(
    config_builder: qiniu_ng_config_builder_t,
    delay_secs: c_ulonglong,
) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_builder.0 as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.http_request_retry_delay(Duration::from_secs(delay_secs));
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_host_freeze_duration(
    config_builder: qiniu_ng_config_builder_t,
    duration_secs: c_ulonglong,
) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_builder.0 as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.host_freeze_duration(Duration::from_secs(duration_secs));
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_free(config_builder: qiniu_ng_config_builder_t) {
    unsafe { Box::from_raw(config_builder.0 as usize as *mut ConfigBuilder) };
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_build(config_builder: qiniu_ng_config_builder_t) -> qiniu_ng_config_t {
    let boxed_config_builder = unsafe { Box::from_raw(config_builder.0 as usize as *mut ConfigBuilder) };
    let boxed_config = Box::new(boxed_config_builder.build().unwrap());
    qiniu_ng_config_t(Box::into_raw(boxed_config) as usize as *mut c_void)
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_use_https(config: qiniu_ng_config_t) -> bool {
    let boxed_config = unsafe { Box::from_raw(config.0 as usize as *mut Config) };
    let use_https = boxed_config.use_https();
    Box::into_raw(boxed_config);
    use_https
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_upload_token_lifetime(config: qiniu_ng_config_t) -> c_ulonglong {
    let boxed_config = unsafe { Box::from_raw(config.0 as usize as *mut Config) };
    let upload_token_lifetime = boxed_config.upload_token_lifetime();
    Box::into_raw(boxed_config);
    upload_token_lifetime.as_secs()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_batch_max_operation_size(config: qiniu_ng_config_t) -> size_t {
    let boxed_config = unsafe { Box::from_raw(config.0 as usize as *mut Config) };
    let batch_max_operation_size = boxed_config.batch_max_operation_size();
    Box::into_raw(boxed_config);
    batch_max_operation_size
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_upload_threshold(config: qiniu_ng_config_t) -> c_ulonglong {
    let boxed_config = unsafe { Box::from_raw(config.0 as usize as *mut Config) };
    let upload_threshold = boxed_config.upload_threshold();
    Box::into_raw(boxed_config);
    upload_threshold
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_upload_chunk_size(config: qiniu_ng_config_t) -> size_t {
    let boxed_config = unsafe { Box::from_raw(config.0 as usize as *mut Config) };
    let upload_chunk_size = boxed_config.upload_chunk_size();
    Box::into_raw(boxed_config);
    upload_chunk_size
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_http_request_retries(config: qiniu_ng_config_t) -> size_t {
    let boxed_config = unsafe { Box::from_raw(config.0 as usize as *mut Config) };
    let http_request_retries = boxed_config.http_request_retries();
    Box::into_raw(boxed_config);
    http_request_retries
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_http_request_retry_delay(config: qiniu_ng_config_t) -> c_ulonglong {
    let boxed_config = unsafe { Box::from_raw(config.0 as usize as *mut Config) };
    let http_request_retry_delay = boxed_config.http_request_retry_delay();
    Box::into_raw(boxed_config);
    http_request_retry_delay.as_secs()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_host_freeze_duration(config: qiniu_ng_config_t) -> c_ulonglong {
    let boxed_config = unsafe { Box::from_raw(config.0 as usize as *mut Config) };
    let host_freeze_duration = boxed_config.host_freeze_duration();
    Box::into_raw(boxed_config);
    host_freeze_duration.as_secs()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_free(config: qiniu_ng_config_t) {
    unsafe { Box::from_raw(config.0 as usize as *mut Config) };
}
