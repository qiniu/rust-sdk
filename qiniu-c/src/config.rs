use libc::{c_uint, c_ulonglong, c_void, size_t};
use qiniu_ng::config::{default as default_config, Config, ConfigBuilder};
use std::{mem::transmute, time::Duration};
use tap::TapOps;

#[repr(C)]
pub struct qiniu_ng_config_fields_t {
    pub use_https: bool,
    pub upload_token_lifetime: c_ulonglong,
    pub batch_max_operation_size: size_t,
    pub upload_threshold: c_uint,
    pub upload_block_size: c_uint,
    pub http_request_retries: size_t,
    pub http_request_retry_delay: c_ulonglong,
}

#[repr(C)]
pub struct qiniu_ng_config_t(*mut c_void);

#[no_mangle]
pub extern "C" fn qiniu_ng_config_fields_init(config: *mut qiniu_ng_config_fields_t) {
    unsafe {
        (*config).use_https = default_config::use_https();
        (*config).upload_token_lifetime = default_config::upload_token_lifetime().as_secs();
        (*config).batch_max_operation_size = default_config::batch_max_operation_size();
        (*config).upload_threshold = default_config::upload_threshold();
        (*config).upload_block_size = default_config::upload_block_size();
        (*config).http_request_retries = default_config::http_request_retries();
        (*config).http_request_retry_delay = default_config::http_request_retry_delay().as_secs();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_new(fields: *const qiniu_ng_config_fields_t) -> qiniu_ng_config_t {
    let config = unsafe { fields.as_ref() }.unwrap();
    ConfigBuilder::default()
        .use_https(config.use_https)
        .upload_token_lifetime(Duration::from_secs(config.upload_token_lifetime))
        .batch_max_operation_size(config.batch_max_operation_size)
        .upload_threshold(config.upload_threshold)
        .upload_block_size(config.upload_block_size)
        .http_request_retries(config.http_request_retries)
        .http_request_retry_delay(Duration::from_secs(config.http_request_retry_delay))
        .build()
        .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_new_default() -> qiniu_ng_config_t {
    Config::default().into()
}

impl From<qiniu_ng_config_t> for Config {
    fn from(config: qiniu_ng_config_t) -> Self {
        unsafe { Config::from_raw(transmute(config)) }
    }
}

impl From<Config> for qiniu_ng_config_t {
    fn from(config: Config) -> Self {
        unsafe { transmute(config.into_raw()) }
    }
}

impl qiniu_ng_config_t {
    pub fn get_clone(self) -> Config {
        let config: Config = self.into();
        config.clone().tap(|_| {
            let _: Self = config.into();
        })
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_fill_fields(config: qiniu_ng_config_t, fields: *mut qiniu_ng_config_fields_t) {
    let config: Config = config.into();
    unsafe {
        (*fields).use_https = config.use_https();
        (*fields).upload_token_lifetime = config.upload_token_lifetime().as_secs();
        (*fields).batch_max_operation_size = config.batch_max_operation_size();
        (*fields).upload_threshold = config.upload_threshold();
        (*fields).upload_block_size = config.upload_block_size();
        (*fields).http_request_retries = config.http_request_retries();
        (*fields).http_request_retry_delay = config.http_request_retry_delay().as_secs();
    }
    let _: qiniu_ng_config_t = config.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_free(config: qiniu_ng_config_t) {
    let _: Config = config.into();
}
