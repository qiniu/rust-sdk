use libc::{c_ulonglong, c_void, size_t};
use qiniu::{Config, ConfigBuilder};
use std::time::Duration;

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_new() -> *mut c_void {
    let config_builder = Box::new(ConfigBuilder::default());
    Box::into_raw(config_builder) as usize as *mut c_void
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_use_https(config_ptr: *mut c_void) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_ptr as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.use_https(true);
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_use_http(config_ptr: *mut c_void) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_ptr as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.use_https(false);
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_token_lifetime(config_ptr: *mut c_void, lifetime_secs: c_ulonglong) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_ptr as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.upload_token_lifetime(Duration::from_secs(lifetime_secs));
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_batch_max_operation_size(config_ptr: *mut c_void, operation_size: size_t) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_ptr as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.batch_max_operation_size(operation_size);
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_threshold(config_ptr: *mut c_void, threshold: c_ulonglong) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_ptr as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.upload_threshold(threshold);
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_chunk_size(config_ptr: *mut c_void, chunk_size: size_t) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_ptr as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.upload_chunk_size(chunk_size);
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_http_request_retries(config_ptr: *mut c_void, retries: size_t) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_ptr as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.http_request_retries(retries);
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_http_request_retry_delay(config_ptr: *mut c_void, delay_secs: c_ulonglong) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_ptr as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.http_request_retry_delay(Duration::from_secs(delay_secs));
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_host_freeze_duration(config_ptr: *mut c_void, duration_secs: c_ulonglong) {
    let mut boxed_config_builder = unsafe { Box::from_raw(config_ptr as usize as *mut ConfigBuilder) };
    *boxed_config_builder = boxed_config_builder.host_freeze_duration(Duration::from_secs(duration_secs));
    Box::into_raw(boxed_config_builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_free(config_ptr: *mut c_void) {
    unsafe { Box::from_raw(config_ptr as usize as *mut ConfigBuilder) };
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_build(config_ptr: *mut c_void) -> *mut c_void {
    let boxed_config_builder = unsafe { Box::from_raw(config_ptr as usize as *mut ConfigBuilder) };
    let boxed_config = Box::new(boxed_config_builder.build().unwrap());
    Box::into_raw(boxed_config) as usize as *mut c_void
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_use_https(config_ptr: *mut c_void) -> bool {
    let boxed_config = unsafe { Box::from_raw(config_ptr as usize as *mut Config) };
    let use_https = boxed_config.use_https();
    Box::into_raw(boxed_config);
    use_https
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_upload_token_lifetime(config_ptr: *mut c_void) -> c_ulonglong {
    let boxed_config = unsafe { Box::from_raw(config_ptr as usize as *mut Config) };
    let upload_token_lifetime = boxed_config.upload_token_lifetime();
    Box::into_raw(boxed_config);
    upload_token_lifetime.as_secs()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_batch_max_operation_size(config_ptr: *mut c_void) -> size_t {
    let boxed_config = unsafe { Box::from_raw(config_ptr as usize as *mut Config) };
    let batch_max_operation_size = boxed_config.batch_max_operation_size();
    Box::into_raw(boxed_config);
    batch_max_operation_size
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_upload_threshold(config_ptr: *mut c_void) -> c_ulonglong {
    let boxed_config = unsafe { Box::from_raw(config_ptr as usize as *mut Config) };
    let upload_threshold = boxed_config.upload_threshold();
    Box::into_raw(boxed_config);
    upload_threshold
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_upload_chunk_size(config_ptr: *mut c_void) -> size_t {
    let boxed_config = unsafe { Box::from_raw(config_ptr as usize as *mut Config) };
    let upload_chunk_size = boxed_config.upload_chunk_size();
    Box::into_raw(boxed_config);
    upload_chunk_size
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_http_request_retries(config_ptr: *mut c_void) -> size_t {
    let boxed_config = unsafe { Box::from_raw(config_ptr as usize as *mut Config) };
    let http_request_retries = boxed_config.http_request_retries();
    Box::into_raw(boxed_config);
    http_request_retries
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_http_request_retry_delay(config_ptr: *mut c_void) -> c_ulonglong {
    let boxed_config = unsafe { Box::from_raw(config_ptr as usize as *mut Config) };
    let http_request_retry_delay = boxed_config.http_request_retry_delay();
    Box::into_raw(boxed_config);
    http_request_retry_delay.as_secs()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_host_freeze_duration(config_ptr: *mut c_void) -> c_ulonglong {
    let boxed_config = unsafe { Box::from_raw(config_ptr as usize as *mut Config) };
    let host_freeze_duration = boxed_config.host_freeze_duration();
    Box::into_raw(boxed_config);
    host_freeze_duration.as_secs()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_free(config_ptr: *mut c_void) {
    unsafe { Box::from_raw(config_ptr as usize as *mut Config) };
}
