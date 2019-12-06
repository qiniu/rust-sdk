use crate::utils::{make_string, qiniu_ng_string_t};
use libc::{c_char, c_uint, c_ulonglong, c_void, size_t};
use qiniu_ng::config::{default as default_config, Config, ConfigBuilder};
use std::{ffi::CStr, mem::transmute, ptr::null, time::Duration};
use tap::TapOps;

#[repr(C)]
#[derive(Clone)]
pub struct qiniu_ng_config_fields_t {
    pub use_https: bool,
    pub uc_host: *const c_char,
    pub rs_host: *const c_char,
    pub upload_token_lifetime: c_ulonglong,
    pub batch_max_operation_size: size_t,
    pub upload_threshold: c_uint,
    pub upload_block_size: c_uint,
    pub http_request_retries: size_t,
    pub http_request_retry_delay: c_ulonglong,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_config_t(*mut c_void);

#[no_mangle]
pub extern "C" fn qiniu_ng_config_fields_init(config: *mut qiniu_ng_config_fields_t) {
    unsafe {
        *config = qiniu_ng_config_fields_t {
            use_https: default_config::use_https(),
            uc_host: null(),
            rs_host: null(),
            upload_token_lifetime: default_config::upload_token_lifetime().as_secs(),
            batch_max_operation_size: default_config::batch_max_operation_size(),
            upload_threshold: default_config::upload_threshold(),
            upload_block_size: default_config::upload_block_size(),
            http_request_retries: default_config::http_request_retries(),
            http_request_retry_delay: default_config::http_request_retry_delay().as_secs(),
        };
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_new(fields: *const qiniu_ng_config_fields_t) -> qiniu_ng_config_t {
    let config = unsafe { fields.as_ref() }.unwrap();
    let mut builder = ConfigBuilder::default()
        .use_https(config.use_https)
        .upload_token_lifetime(Duration::from_secs(config.upload_token_lifetime))
        .batch_max_operation_size(config.batch_max_operation_size)
        .upload_threshold(config.upload_threshold)
        .upload_block_size(config.upload_block_size)
        .http_request_retries(config.http_request_retries)
        .http_request_retry_delay(Duration::from_secs(config.http_request_retry_delay));
    if !config.uc_host.is_null() {
        builder = builder.uc_host(
            unsafe { CStr::from_ptr(config.uc_host) }
                .to_string_lossy()
                .into_owned()
                .into(),
        );
    }
    if !config.rs_host.is_null() {
        builder = builder.rs_host(
            unsafe { CStr::from_ptr(config.rs_host) }
                .to_string_lossy()
                .into_owned()
                .into(),
        );
    }
    builder.build().into()
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
pub extern "C" fn qiniu_ng_config_get_use_https(config: qiniu_ng_config_t) -> bool {
    let config: Config = config.into();
    config.use_https().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uc_host(config: qiniu_ng_config_t) -> qiniu_ng_string_t {
    let config: Config = config.into();
    make_string(config.uc_host().as_ref()).tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uc_url(config: qiniu_ng_config_t) -> qiniu_ng_string_t {
    let config: Config = config.into();
    make_string(config.uc_url()).tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_rs_host(config: qiniu_ng_config_t) -> qiniu_ng_string_t {
    let config: Config = config.into();
    make_string(config.rs_host().as_ref()).tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_rs_url(config: qiniu_ng_config_t) -> qiniu_ng_string_t {
    let config: Config = config.into();
    make_string(config.rs_url()).tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_token_lifetime(config: qiniu_ng_config_t) -> c_ulonglong {
    let config: Config = config.into();
    config.upload_token_lifetime().as_secs().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_batch_max_operation_size(config: qiniu_ng_config_t) -> size_t {
    let config: Config = config.into();
    config.batch_max_operation_size().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_threshold(config: qiniu_ng_config_t) -> c_uint {
    let config: Config = config.into();
    config.upload_threshold().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_block_size(config: qiniu_ng_config_t) -> c_uint {
    let config: Config = config.into();
    config.upload_block_size().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_request_retries(config: qiniu_ng_config_t) -> size_t {
    let config: Config = config.into();
    config.http_request_retries().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_request_retry_delay(config: qiniu_ng_config_t) -> c_ulonglong {
    let config: Config = config.into();
    config.http_request_retry_delay().as_secs().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_free(config: qiniu_ng_config_t) {
    let _: Config = config.into();
}
