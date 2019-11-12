use libc::{c_ulonglong, size_t};
use qiniu_ng::config::{default as default_config, Config, ConfigBuilder};
use std::time::Duration;

#[repr(C)]
pub struct qiniu_ng_config_t {
    pub use_https: bool,
    pub upload_token_lifetime: c_ulonglong,
    pub batch_max_operation_size: size_t,
    pub upload_threshold: c_ulonglong,
    pub upload_block_size: size_t,
    pub upload_block_lifetime: c_ulonglong,
    pub always_flush_records: bool,
    pub uplog_disabled: bool,
    pub uplog_upload_threshold: size_t,
    pub uplog_max_size: size_t,
    pub http_request_retries: size_t,
    pub http_request_retry_delay: c_ulonglong,
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_init(config: *mut qiniu_ng_config_t) {
    unsafe {
        (*config).use_https = default_config::use_https();
        (*config).upload_token_lifetime = default_config::upload_token_lifetime().as_secs();
        (*config).batch_max_operation_size = default_config::batch_max_operation_size();
        (*config).upload_threshold = default_config::upload_threshold();
        (*config).upload_block_size = default_config::upload_block_size();
        (*config).upload_block_lifetime = default_config::upload_block_lifetime().as_secs();
        (*config).always_flush_records = default_config::always_flush_records();
        (*config).uplog_disabled = default_config::uplog_disabled();
        (*config).uplog_upload_threshold = default_config::uplog_upload_threshold();
        (*config).uplog_max_size = default_config::uplog_max_size();
        (*config).http_request_retries = default_config::http_request_retries();
        (*config).http_request_retry_delay = default_config::http_request_retry_delay().as_secs();
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
            .always_flush_records(config.always_flush_records)
            .uplog_disabled(config.uplog_disabled)
            .uplog_upload_threshold(config.uplog_upload_threshold)
            .uplog_max_size(config.uplog_max_size)
            .http_request_retries(config.http_request_retries)
            .http_request_retry_delay(Duration::from_secs(config.http_request_retry_delay))
            .build()
    }
}
