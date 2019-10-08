use libc::{c_ulonglong, size_t};
use qiniu::config::{Config, ConfigBuilder, ConfigInner};
use std::time::Duration;

#[repr(C)]
pub struct qiniu_ng_config_t {
    pub use_https: bool,
    pub upload_token_lifetime: c_ulonglong,
    pub batch_max_operation_size: size_t,
    pub upload_threshold: c_ulonglong,
    pub upload_block_size: size_t,
    pub upload_block_lifetime: c_ulonglong,
    pub http_request_retries: size_t,
    pub http_request_retry_delay: c_ulonglong,
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_init(config: *mut qiniu_ng_config_t) {
    let default = ConfigInner::default();
    unsafe {
        (*config).use_https = default.use_https();
        (*config).upload_token_lifetime = default.upload_token_lifetime().as_secs();
        (*config).batch_max_operation_size = default.batch_max_operation_size();
        (*config).upload_threshold = default.upload_threshold();
        (*config).upload_block_size = default.upload_block_size();
        (*config).upload_block_lifetime = default.upload_block_lifetime().as_secs();
        (*config).http_request_retries = default.http_request_retries();
        (*config).http_request_retry_delay = default.http_request_retry_delay().as_secs();
    }
}

impl From<&qiniu_ng_config_t> for Config {
    fn from(config: &qiniu_ng_config_t) -> Self {
        ConfigBuilder::default()
            .use_https(config.use_https)
            .upload_token_lifetime(Duration::from_secs(config.upload_token_lifetime))
            .batch_max_operation_size(config.batch_max_operation_size)
            .upload_threshold(config.upload_threshold)
            .upload_block_size(config.upload_block_size)
            .upload_block_lifetime(Duration::from_secs(config.upload_block_lifetime))
            .http_request_retries(config.http_request_retries)
            .http_request_retry_delay(Duration::from_secs(config.http_request_retry_delay))
            .build()
            .unwrap()
    }
}
