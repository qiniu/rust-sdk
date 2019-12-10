use crate::{
    result::qiniu_ng_err,
    utils::{
        convert_c_char_pointer_to_boxed_cstr, make_path_buf, make_string, qiniu_ng_optional_string_t, qiniu_ng_string_t,
    },
};
use libc::{c_char, c_uint, c_ulonglong, c_void, size_t};
use qiniu_ng::{
    config::{default as default_config, Config as QiniuConfig, ConfigBuilder as QiniuConfigBuilder},
    storage::{
        recorder::FileSystemRecorder,
        uploader::{
            upload_logger::{
                default as default_upload_logger, LockPolicy as UploadLoggerLockPolicy,
                UploadLoggerBuilder as QiniuUploadLoggerBuilder,
            },
            upload_recorder::{
                default as default_upload_recorder, UploadRecorderBuilder as QiniuUploadRecorderBuilder,
            },
        },
    },
};
use std::{ffi::CStr, mem::transmute, ptr::null, time::Duration};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_config_builder_t(*mut c_void);

struct ConfigBuilder {
    use_https: bool,
    uc_host: Box<CStr>,
    rs_host: Box<CStr>,
    upload_token_lifetime: Duration,
    batch_max_operation_size: usize,
    upload_threshold: u32,
    upload_block_size: u32,
    http_request_retries: usize,
    http_request_retry_delay: Duration,
    upload_logger: Option<UploadLoggerBuilder>,
    upload_recorder: UploadRecorderBuilder,
}

struct UploadLoggerBuilder {
    server_url: Box<CStr>,
    log_file_path: Box<CStr>,
    lock_policy: UploadLoggerLockPolicy,
    upload_threshold: u32,
    max_size: u32,
}

struct UploadRecorderBuilder {
    root_directory: Box<CStr>,
    upload_block_lifetime: Duration,
    always_flush_records: bool,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        ConfigBuilder {
            use_https: default_config::use_https(),
            uc_host: make_string(default_config::uc_host().as_ref()).into(),
            rs_host: make_string(default_config::rs_host().as_ref()).into(),
            upload_token_lifetime: default_config::upload_token_lifetime(),
            batch_max_operation_size: default_config::batch_max_operation_size(),
            upload_threshold: default_config::upload_threshold(),
            upload_block_size: default_config::upload_block_size(),
            http_request_retries: default_config::http_request_retries(),
            http_request_retry_delay: default_config::http_request_retry_delay(),
            upload_logger: Some(Default::default()),
            upload_recorder: Default::default(),
        }
    }
}

impl Default for UploadLoggerBuilder {
    fn default() -> Self {
        UploadLoggerBuilder {
            server_url: make_string(default_upload_logger::server_url().as_ref()).into(),
            log_file_path: make_string(
                default_upload_logger::log_file_path()
                    .as_ref()
                    .to_string_lossy()
                    .as_ref(),
            )
            .into(),
            lock_policy: default_upload_logger::lock_policy(),
            upload_threshold: default_upload_logger::upload_threshold(),
            max_size: default_upload_logger::max_size(),
        }
    }
}

impl Default for UploadRecorderBuilder {
    fn default() -> Self {
        UploadRecorderBuilder {
            root_directory: make_string(FileSystemRecorder::default_root_directory().to_string_lossy().as_ref()).into(),
            upload_block_lifetime: default_upload_recorder::upload_block_lifetime(),
            always_flush_records: default_upload_recorder::always_flush_records(),
        }
    }
}

impl From<Box<ConfigBuilder>> for qiniu_ng_config_builder_t {
    fn from(builder: Box<ConfigBuilder>) -> Self {
        unsafe { transmute(Box::into_raw(builder)) }
    }
}

impl From<qiniu_ng_config_builder_t> for Box<ConfigBuilder> {
    fn from(builder: qiniu_ng_config_builder_t) -> Self {
        unsafe { Box::from_raw(transmute::<_, *mut ConfigBuilder>(builder)) }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_new() -> qiniu_ng_config_builder_t {
    Box::new(ConfigBuilder::default()).into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_free(builder: qiniu_ng_config_builder_t) {
    let _: Box<ConfigBuilder> = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_use_https(builder: qiniu_ng_config_builder_t) -> bool {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.use_https.tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_uc_host(builder: qiniu_ng_config_builder_t) -> *const c_char {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.uc_host.as_ptr().tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_rs_host(builder: qiniu_ng_config_builder_t) -> *const c_char {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.rs_host.as_ptr().tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_upload_token_lifetime(builder: qiniu_ng_config_builder_t) -> c_ulonglong {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.upload_token_lifetime.as_secs().tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_batch_max_operation_size(builder: qiniu_ng_config_builder_t) -> size_t {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.batch_max_operation_size.tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_upload_threshold(builder: qiniu_ng_config_builder_t) -> c_uint {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.upload_threshold.tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_upload_block_size(builder: qiniu_ng_config_builder_t) -> c_uint {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.upload_block_size.tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_http_request_retries(builder: qiniu_ng_config_builder_t) -> size_t {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.http_request_retries.tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_http_request_retry_delay(
    builder: qiniu_ng_config_builder_t,
) -> c_ulonglong {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.http_request_retry_delay.as_secs().tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_is_uplog_enabled(builder: qiniu_ng_config_builder_t) -> bool {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.upload_logger.is_some().tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_uplog_server_url(builder: qiniu_ng_config_builder_t) -> *const c_char {
    let builder: Box<ConfigBuilder> = builder.into();
    builder
        .upload_logger
        .as_ref()
        .map(|logger| logger.server_url.as_ptr())
        .unwrap_or_else(null)
        .tap(|_| {
            let _: qiniu_ng_config_builder_t = builder.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_uplog_file_path(builder: qiniu_ng_config_builder_t) -> *const c_char {
    let builder: Box<ConfigBuilder> = builder.into();
    builder
        .upload_logger
        .as_ref()
        .map(|logger| logger.log_file_path.as_ptr())
        .unwrap_or_else(null)
        .tap(|_| {
            let _: qiniu_ng_config_builder_t = builder.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_uplog_file_lock_policy(
    builder: qiniu_ng_config_builder_t,
    lock_policy: *mut qiniu_ng_upload_logger_lock_policy_t,
) -> bool {
    let builder: Box<ConfigBuilder> = builder.into();
    builder
        .upload_logger
        .as_ref()
        .map(|logger| {
            if !lock_policy.is_null() {
                unsafe { *lock_policy = logger.lock_policy.into() };
            }
            true
        })
        .unwrap_or(false)
        .tap(|_| {
            let _: qiniu_ng_config_builder_t = builder.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_uplog_file_upload_threshold(
    builder: qiniu_ng_config_builder_t,
    upload_threshold: *mut c_uint,
) -> bool {
    let builder: Box<ConfigBuilder> = builder.into();
    builder
        .upload_logger
        .as_ref()
        .map(|logger| {
            if !upload_threshold.is_null() {
                unsafe { *upload_threshold = logger.upload_threshold };
            }
            true
        })
        .unwrap_or(false)
        .tap(|_| {
            let _: qiniu_ng_config_builder_t = builder.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_uplog_file_max_size(
    builder: qiniu_ng_config_builder_t,
    max_size: *mut c_uint,
) -> bool {
    let builder: Box<ConfigBuilder> = builder.into();
    builder
        .upload_logger
        .as_ref()
        .map(|logger| {
            if !max_size.is_null() {
                unsafe { *max_size = logger.max_size };
            }
            true
        })
        .unwrap_or(false)
        .tap(|_| {
            let _: qiniu_ng_config_builder_t = builder.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_upload_recorder_root_directory(
    builder: qiniu_ng_config_builder_t,
) -> *const c_char {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.upload_recorder.root_directory.as_ptr().tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_upload_recorder_upload_block_lifetime(
    builder: qiniu_ng_config_builder_t,
) -> c_ulonglong {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.upload_recorder.upload_block_lifetime.as_secs().tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_get_upload_recorder_always_flush_records(
    builder: qiniu_ng_config_builder_t,
) -> bool {
    let builder: Box<ConfigBuilder> = builder.into();
    builder.upload_recorder.always_flush_records.tap(|_| {
        let _: qiniu_ng_config_builder_t = builder.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_use_https(builder: qiniu_ng_config_builder_t, use_https: bool) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.use_https = use_https;
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_uc_host(builder: qiniu_ng_config_builder_t, uc_host: *const c_char) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.uc_host = convert_c_char_pointer_to_boxed_cstr(uc_host);
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_rs_host(builder: qiniu_ng_config_builder_t, rs_host: *const c_char) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.rs_host = convert_c_char_pointer_to_boxed_cstr(rs_host);
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_upload_token_lifetime(
    builder: qiniu_ng_config_builder_t,
    upload_token_lifetime: c_ulonglong,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.upload_token_lifetime = Duration::from_secs(upload_token_lifetime);
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_batch_max_operation_size(
    builder: qiniu_ng_config_builder_t,
    batch_max_operation_size: size_t,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.batch_max_operation_size = batch_max_operation_size;
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_upload_threshold(
    builder: qiniu_ng_config_builder_t,
    upload_threshold: c_uint,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.upload_threshold = upload_threshold;
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_upload_block_size(
    builder: qiniu_ng_config_builder_t,
    upload_block_size: c_uint,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.upload_block_size = upload_block_size;
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_http_request_retries(
    builder: qiniu_ng_config_builder_t,
    http_request_retries: size_t,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.http_request_retries = http_request_retries;
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_http_request_retry_delay(
    builder: qiniu_ng_config_builder_t,
    http_request_retry_delay: c_ulonglong,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.http_request_retry_delay = Duration::from_secs(http_request_retry_delay);
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_disable_uplog(builder: qiniu_ng_config_builder_t) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.upload_logger = None;
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_enable_uplog(builder: qiniu_ng_config_builder_t) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.upload_logger = Some(Default::default());
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_uplog_server_url(
    builder: qiniu_ng_config_builder_t,
    server_url: *const c_char,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    if let Some(upload_logger) = &mut builder.upload_logger {
        upload_logger.server_url = convert_c_char_pointer_to_boxed_cstr(server_url);
    } else {
        builder.upload_logger = Some(UploadLoggerBuilder {
            server_url: convert_c_char_pointer_to_boxed_cstr(server_url),
            ..Default::default()
        });
    }
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_uplog_file_path(
    builder: qiniu_ng_config_builder_t,
    file_path: *const c_char,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    if let Some(upload_logger) = &mut builder.upload_logger {
        upload_logger.log_file_path = convert_c_char_pointer_to_boxed_cstr(file_path);
    } else {
        builder.upload_logger = Some(UploadLoggerBuilder {
            log_file_path: convert_c_char_pointer_to_boxed_cstr(file_path),
            ..Default::default()
        });
    }
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_uplog_file_lock_policy(
    builder: qiniu_ng_config_builder_t,
    lock_policy: qiniu_ng_upload_logger_lock_policy_t,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    if let Some(upload_logger) = &mut builder.upload_logger {
        upload_logger.lock_policy = lock_policy.into();
    } else {
        builder.upload_logger = Some(UploadLoggerBuilder {
            lock_policy: lock_policy.into(),
            ..Default::default()
        });
    }
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_uplog_file_upload_threshold(
    builder: qiniu_ng_config_builder_t,
    upload_threshold: c_uint,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    if let Some(upload_logger) = &mut builder.upload_logger {
        upload_logger.upload_threshold = upload_threshold;
    } else {
        builder.upload_logger = Some(UploadLoggerBuilder {
            upload_threshold,
            ..Default::default()
        });
    }
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_uplog_file_max_size(
    builder: qiniu_ng_config_builder_t,
    max_size: c_uint,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    if let Some(upload_logger) = &mut builder.upload_logger {
        upload_logger.max_size = max_size;
    } else {
        builder.upload_logger = Some(UploadLoggerBuilder {
            max_size,
            ..Default::default()
        });
    }
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_upload_recorder_root_directory(
    builder: qiniu_ng_config_builder_t,
    root_directory: *const c_char,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.upload_recorder.root_directory = convert_c_char_pointer_to_boxed_cstr(root_directory);
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_upload_recorder_upload_block_lifetime(
    builder: qiniu_ng_config_builder_t,
    upload_block_lifetime: c_ulonglong,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.upload_recorder.upload_block_lifetime = Duration::from_secs(upload_block_lifetime);
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_upload_recorder_always_flush_records(
    builder: qiniu_ng_config_builder_t,
    always_flush_records: bool,
) {
    let mut builder: Box<ConfigBuilder> = builder.into();
    builder.upload_recorder.always_flush_records = always_flush_records;
    let _: qiniu_ng_config_builder_t = builder.into();
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_build(
    builder: qiniu_ng_config_builder_t,
    config: *mut qiniu_ng_config_t,
    error: *mut qiniu_ng_err,
) -> bool {
    let builder: Box<ConfigBuilder> = builder.into();
    let upload_logger = match builder.upload_logger.map(|logger| {
        QiniuUploadLoggerBuilder::default()
            .server_url(logger.server_url.to_string_lossy().into_owned().into())
            .log_file_path(make_path_buf(logger.log_file_path.as_ptr()).into())
            .lock_policy(logger.lock_policy)
            .upload_threshold(logger.upload_threshold)
            .max_size(logger.max_size)
            .build()
    }) {
        Some(Ok(upload_logger)) => Some(upload_logger),
        Some(Err(err)) => {
            if !error.is_null() {
                unsafe { *error = (&err).into() };
            }
            return false;
        }
        None => None,
    };
    if !config.is_null() {
        unsafe {
            *config = QiniuConfigBuilder::default()
                .use_https(builder.use_https)
                .uc_host(builder.uc_host.to_string_lossy().into_owned().into())
                .rs_host(builder.rs_host.to_string_lossy().into_owned().into())
                .upload_token_lifetime(builder.upload_token_lifetime)
                .batch_max_operation_size(builder.batch_max_operation_size)
                .upload_threshold(builder.upload_threshold)
                .upload_block_size(builder.upload_block_size)
                .http_request_retries(builder.http_request_retries)
                .http_request_retry_delay(builder.http_request_retry_delay)
                .upload_logger(upload_logger)
                .upload_recorder(
                    QiniuUploadRecorderBuilder::default()
                        .recorder(FileSystemRecorder::from(make_path_buf(
                            builder.upload_recorder.root_directory.as_ptr(),
                        )))
                        .upload_block_lifetime(builder.upload_recorder.upload_block_lifetime)
                        .always_flush_records(builder.upload_recorder.always_flush_records)
                        .build(),
                )
                .build()
                .into()
        };
    }
    true
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_config_t(*mut c_void);

#[no_mangle]
pub extern "C" fn qiniu_ng_config_new_default() -> qiniu_ng_config_t {
    QiniuConfig::default().into()
}

impl From<qiniu_ng_config_t> for QiniuConfig {
    fn from(config: qiniu_ng_config_t) -> Self {
        unsafe { QiniuConfig::from_raw(transmute(config)) }
    }
}

impl From<QiniuConfig> for qiniu_ng_config_t {
    fn from(config: QiniuConfig) -> Self {
        unsafe { transmute(config.into_raw()) }
    }
}

impl qiniu_ng_config_t {
    pub fn get_clone(self) -> QiniuConfig {
        let config: QiniuConfig = self.into();
        config.clone().tap(|_| {
            let _: Self = config.into();
        })
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_use_https(config: qiniu_ng_config_t) -> bool {
    let config: QiniuConfig = config.into();
    config.use_https().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uc_host(config: qiniu_ng_config_t) -> qiniu_ng_string_t {
    let config: QiniuConfig = config.into();
    make_string(config.uc_host().as_ref()).tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uc_url(config: qiniu_ng_config_t) -> qiniu_ng_string_t {
    let config: QiniuConfig = config.into();
    make_string(config.uc_url()).tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_rs_host(config: qiniu_ng_config_t) -> qiniu_ng_string_t {
    let config: QiniuConfig = config.into();
    make_string(config.rs_host().as_ref()).tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_rs_url(config: qiniu_ng_config_t) -> qiniu_ng_string_t {
    let config: QiniuConfig = config.into();
    make_string(config.rs_url()).tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_token_lifetime(config: qiniu_ng_config_t) -> c_ulonglong {
    let config: QiniuConfig = config.into();
    config.upload_token_lifetime().as_secs().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_batch_max_operation_size(config: qiniu_ng_config_t) -> size_t {
    let config: QiniuConfig = config.into();
    config.batch_max_operation_size().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_threshold(config: qiniu_ng_config_t) -> c_uint {
    let config: QiniuConfig = config.into();
    config.upload_threshold().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_block_size(config: qiniu_ng_config_t) -> c_uint {
    let config: QiniuConfig = config.into();
    config.upload_block_size().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_request_retries(config: qiniu_ng_config_t) -> size_t {
    let config: QiniuConfig = config.into();
    config.http_request_retries().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_request_retry_delay(config: qiniu_ng_config_t) -> c_ulonglong {
    let config: QiniuConfig = config.into();
    config.http_request_retry_delay().as_secs().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_is_uplog_enabled(config: qiniu_ng_config_t) -> bool {
    let config: QiniuConfig = config.into();
    config.upload_logger().is_some().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_server_url(config: qiniu_ng_config_t) -> qiniu_ng_optional_string_t {
    let config: QiniuConfig = config.into();
    config
        .upload_logger()
        .as_ref()
        .map(|upload_logger| make_string(upload_logger.server_url().to_owned()).into())
        .unwrap_or_else(qiniu_ng_optional_string_t::default)
        .tap(|_| {
            let _: qiniu_ng_config_t = config.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_file_path(config: qiniu_ng_config_t) -> qiniu_ng_optional_string_t {
    let config: QiniuConfig = config.into();
    config
        .upload_logger()
        .as_ref()
        .map(|upload_logger| make_string(upload_logger.log_file_path().to_string_lossy().into_owned()).into())
        .unwrap_or_else(qiniu_ng_optional_string_t::default)
        .tap(|_| {
            let _: qiniu_ng_config_t = config.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_file_lock_policy(
    config: qiniu_ng_config_t,
    lock_policy: *mut qiniu_ng_upload_logger_lock_policy_t,
) -> bool {
    let config: QiniuConfig = config.into();
    config
        .upload_logger()
        .as_ref()
        .map(|upload_logger| {
            if !lock_policy.is_null() {
                unsafe { *lock_policy = upload_logger.lock_policy().into() };
            }
            true
        })
        .unwrap_or(false)
        .tap(|_| {
            let _: qiniu_ng_config_t = config.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_file_upload_threshold(
    config: qiniu_ng_config_t,
    upload_threshold: *mut c_uint,
) -> bool {
    let config: QiniuConfig = config.into();
    config
        .upload_logger()
        .as_ref()
        .map(|upload_logger| {
            if !upload_threshold.is_null() {
                unsafe { *upload_threshold = upload_logger.upload_threshold() };
            }
            true
        })
        .unwrap_or(false)
        .tap(|_| {
            let _: qiniu_ng_config_t = config.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_file_max_size(config: qiniu_ng_config_t, max_size: *mut c_uint) -> bool {
    let config: QiniuConfig = config.into();
    config
        .upload_logger()
        .as_ref()
        .map(|upload_logger| {
            if !max_size.is_null() {
                unsafe { *max_size = upload_logger.max_size() };
            }
            true
        })
        .unwrap_or(false)
        .tap(|_| {
            let _: qiniu_ng_config_t = config.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_recorder_root_directory(
    config: qiniu_ng_config_t,
) -> qiniu_ng_optional_string_t {
    let config: QiniuConfig = config.into();
    config
        .upload_recorder()
        .recorder()
        .as_any()
        .downcast_ref::<FileSystemRecorder>()
        .map(|file_system_recorder| {
            make_string(file_system_recorder.root_directory().to_string_lossy().into_owned()).into()
        })
        .unwrap_or_else(qiniu_ng_optional_string_t::default)
        .tap(|_| {
            let _: qiniu_ng_config_t = config.into();
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_recorder_upload_block_lifetime(config: qiniu_ng_config_t) -> c_ulonglong {
    let config: QiniuConfig = config.into();
    config.upload_recorder().upload_block_lifetime().as_secs().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_recorder_always_flush_records(config: qiniu_ng_config_t) -> bool {
    let config: QiniuConfig = config.into();
    config.upload_recorder().always_flush_records().tap(|_| {
        let _: qiniu_ng_config_t = config.into();
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_free(config: qiniu_ng_config_t) {
    let _: QiniuConfig = config.into();
}

#[repr(C)]
#[derive(Copy, Clone)]
pub enum qiniu_ng_upload_logger_lock_policy_t {
    LockSharedDuringAppendingAndLockExclusiveDuringUploading,
    AlwaysLockExclusive,
    None,
}

impl From<qiniu_ng_upload_logger_lock_policy_t> for UploadLoggerLockPolicy {
    fn from(policy: qiniu_ng_upload_logger_lock_policy_t) -> Self {
        match policy {
            qiniu_ng_upload_logger_lock_policy_t::LockSharedDuringAppendingAndLockExclusiveDuringUploading => {
                UploadLoggerLockPolicy::LockSharedDuringAppendingAndLockExclusiveDuringUploading
            }
            qiniu_ng_upload_logger_lock_policy_t::AlwaysLockExclusive => UploadLoggerLockPolicy::AlwaysLockExclusive,
            qiniu_ng_upload_logger_lock_policy_t::None => UploadLoggerLockPolicy::None,
        }
    }
}

impl From<UploadLoggerLockPolicy> for qiniu_ng_upload_logger_lock_policy_t {
    fn from(policy: UploadLoggerLockPolicy) -> Self {
        match policy {
            UploadLoggerLockPolicy::LockSharedDuringAppendingAndLockExclusiveDuringUploading => {
                qiniu_ng_upload_logger_lock_policy_t::LockSharedDuringAppendingAndLockExclusiveDuringUploading
            }
            UploadLoggerLockPolicy::AlwaysLockExclusive => qiniu_ng_upload_logger_lock_policy_t::AlwaysLockExclusive,
            UploadLoggerLockPolicy::None => qiniu_ng_upload_logger_lock_policy_t::None,
        }
    }
}
