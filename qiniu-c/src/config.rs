use crate::{
    http::{qiniu_ng_http_request_t, qiniu_ng_http_response_t},
    result::qiniu_ng_err_t,
    string::{qiniu_ng_char_t, ucstr, UCString},
    utils::qiniu_ng_str_t,
};
use libc::{c_void, size_t};
use qiniu_http::{
    Error as HTTPError, ErrorKind as HTTPErrorKind, Request as HTTPRequest, Response as HTTPResponse,
    Result as HTTPResult,
};
use qiniu_ng::{
    config::{Config, ConfigBuilder},
    http::{DomainsManagerBuilder, HTTPAfterAction, HTTPBeforeAction},
    storage::{
        recorder::FileSystemRecorder,
        uploader::{UploadLoggerBuilder, UploadLoggerFileLockPolicy, UploadRecorderBuilder},
    },
};
use std::{fs::OpenOptions, mem::transmute, ptr::null_mut, time::Duration};
use tap::TapOps;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_config_builder_t(*mut c_void);

// TODO: 提供 Config 的 clone() 方法

struct Builder {
    config_builder: ConfigBuilder,
    upload_logger_builder: Option<UploadLoggerBuilder>,
    upload_recorder_builder: UploadRecorderBuilder,
    domains_manager_builder: DomainsManagerBuilder,
}

impl Default for qiniu_ng_config_builder_t {
    #[inline]
    fn default() -> Self {
        Self(null_mut())
    }
}

impl qiniu_ng_config_builder_t {
    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl From<Box<Builder>> for qiniu_ng_config_builder_t {
    fn from(builder: Box<Builder>) -> Self {
        unsafe { transmute(Box::into_raw(builder)) }
    }
}

impl From<Option<Box<Builder>>> for qiniu_ng_config_builder_t {
    fn from(builder: Option<Box<Builder>>) -> Self {
        builder.map(|builder| builder.into()).unwrap_or_default()
    }
}

impl From<qiniu_ng_config_builder_t> for Option<Box<Builder>> {
    fn from(builder: qiniu_ng_config_builder_t) -> Self {
        if builder.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(transmute(builder)) })
        }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_new() -> qiniu_ng_config_builder_t {
    Box::new(Builder {
        config_builder: Default::default(),
        upload_logger_builder: Some(Default::default()),
        upload_recorder_builder: Default::default(),
        domains_manager_builder: Default::default(),
    })
    .into()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_free(builder: *mut qiniu_ng_config_builder_t) {
    if let Some(builder) = unsafe { builder.as_mut() } {
        let _ = Option::<Box<Builder>>::from(*builder);
        *builder = qiniu_ng_config_builder_t::default();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_is_freed(builder: qiniu_ng_config_builder_t) -> bool {
    builder.is_null()
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_set_appended_user_agent(
    builder: qiniu_ng_config_builder_t,
    user_agent: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder.config_builder.appended_user_agent(
        unsafe { user_agent.as_ref() }
            .map(|user_agent| unsafe { ucstr::from_ptr(user_agent) }.to_string().unwrap().into()),
    );
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_use_https(builder: qiniu_ng_config_builder_t, use_https: bool) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder.config_builder.use_https(use_https);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_uc_host(builder: qiniu_ng_config_builder_t, uc_host: *const qiniu_ng_char_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .uc_host(unsafe { ucstr::from_ptr(uc_host) }.to_string().unwrap());
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_rs_host(builder: qiniu_ng_config_builder_t, rs_host: *const qiniu_ng_char_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .rs_host(unsafe { ucstr::from_ptr(rs_host) }.to_string().unwrap());
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_rsf_host(
    builder: qiniu_ng_config_builder_t,
    rsf_host: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .rsf_host(unsafe { ucstr::from_ptr(rsf_host) }.to_string().unwrap());
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_api_host(
    builder: qiniu_ng_config_builder_t,
    api_host: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .api_host(unsafe { ucstr::from_ptr(api_host) }.to_string().unwrap());
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_uplog_host(
    builder: qiniu_ng_config_builder_t,
    uplog_host: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .uplog_host(unsafe { ucstr::from_ptr(uplog_host) }.to_string().unwrap());
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_token_lifetime(
    builder: qiniu_ng_config_builder_t,
    upload_token_lifetime: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .upload_token_lifetime(Duration::from_secs(upload_token_lifetime));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_batch_max_operation_size(
    builder: qiniu_ng_config_builder_t,
    batch_max_operation_size: size_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .batch_max_operation_size(batch_max_operation_size);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_threshold(builder: qiniu_ng_config_builder_t, upload_threshold: u32) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder.config_builder.upload_threshold(upload_threshold);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_block_size(
    builder: qiniu_ng_config_builder_t,
    upload_block_size: u32,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder.config_builder.upload_block_size(upload_block_size);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_http_connect_timeout(
    builder: qiniu_ng_config_builder_t,
    http_connect_timeout: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .http_connect_timeout(Duration::from_secs(http_connect_timeout));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_http_request_timeout(
    builder: qiniu_ng_config_builder_t,
    http_request_timeout: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .http_request_timeout(Duration::from_secs(http_request_timeout));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_tcp_keepalive_idle_timeout(
    builder: qiniu_ng_config_builder_t,
    tcp_keepalive_idle_timeout: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .tcp_keepalive_idle_timeout(Duration::from_secs(tcp_keepalive_idle_timeout));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_tcp_keepalive_probe_interval(
    builder: qiniu_ng_config_builder_t,
    tcp_keepalive_probe_interval: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .tcp_keepalive_probe_interval(Duration::from_secs(tcp_keepalive_probe_interval));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_http_low_transfer_speed(
    builder: qiniu_ng_config_builder_t,
    low_transfer_speed: u32,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder.config_builder.http_low_transfer_speed(low_transfer_speed);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_http_low_transfer_speed_timeout(
    builder: qiniu_ng_config_builder_t,
    low_transfer_speed_timeout: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .http_low_transfer_speed_timeout(Duration::from_secs(low_transfer_speed_timeout));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_http_request_retries(
    builder: qiniu_ng_config_builder_t,
    http_request_retries: size_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder.config_builder.http_request_retries(http_request_retries);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_http_request_retry_delay(
    builder: qiniu_ng_config_builder_t,
    http_request_retry_delay: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .http_request_retry_delay(Duration::from_secs(http_request_retry_delay));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_disable_uplog(builder: qiniu_ng_config_builder_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.upload_logger_builder = None;
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_enable_uplog(builder: qiniu_ng_config_builder_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.upload_logger_builder = Some(Default::default());
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_uplog_file_path(
    builder: qiniu_ng_config_builder_t,
    file_path: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    let log_file_path = unsafe { UCString::from_ptr(file_path) }.into_path_buf().into();
    builder.upload_logger_builder = Some(
        builder
            .upload_logger_builder
            .unwrap_or_default()
            .log_file_path(log_file_path),
    );
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_uplog_file_lock_policy(
    builder: qiniu_ng_config_builder_t,
    lock_policy: qiniu_ng_upload_logger_lock_policy_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.upload_logger_builder = Some(
        builder
            .upload_logger_builder
            .unwrap_or_default()
            .lock_policy(lock_policy.into()),
    );
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_uplog_file_upload_threshold(
    builder: qiniu_ng_config_builder_t,
    upload_threshold: u32,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.upload_logger_builder = Some(
        builder
            .upload_logger_builder
            .unwrap_or_default()
            .upload_threshold(upload_threshold),
    );
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_uplog_file_max_size(builder: qiniu_ng_config_builder_t, max_size: u32) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.upload_logger_builder = Some(builder.upload_logger_builder.unwrap_or_default().max_size(max_size));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_recorder_root_directory(
    builder: qiniu_ng_config_builder_t,
    root_directory: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    let recorder = FileSystemRecorder::from(unsafe { UCString::from_ptr(root_directory) }.into_path_buf());
    builder.upload_recorder_builder = builder.upload_recorder_builder.recorder(recorder);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_recorder_upload_block_lifetime(
    builder: qiniu_ng_config_builder_t,
    upload_block_lifetime: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.upload_recorder_builder = builder
        .upload_recorder_builder
        .upload_block_lifetime(Duration::from_secs(upload_block_lifetime));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_recorder_always_flush_records(
    builder: qiniu_ng_config_builder_t,
    always_flush_records: bool,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.upload_recorder_builder = builder
        .upload_recorder_builder
        .always_flush_records(always_flush_records);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_load_domains_manager_from_file(
    builder: qiniu_ng_config_builder_t,
    persistent_file: *const qiniu_ng_char_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    let mut result = true;
    match DomainsManagerBuilder::load_from_file(unsafe { UCString::from_ptr(persistent_file) }.into_path_buf()) {
        Ok(domains_manager_builder) => builder.domains_manager_builder = domains_manager_builder,
        Err(ref err) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = err.into();
            }
            result = false;
        }
    }
    let _ = qiniu_ng_config_builder_t::from(builder);
    result
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_create_new_domains_manager(
    builder: qiniu_ng_config_builder_t,
    persistent_file: *const qiniu_ng_char_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    let mut result = true;
    let persistent_file =
        unsafe { persistent_file.as_ref() }.map(|file| unsafe { UCString::from_ptr(file) }.into_path_buf());
    match DomainsManagerBuilder::create_new(persistent_file) {
        Ok(domains_manager_builder) => builder.domains_manager_builder = domains_manager_builder,
        Err(ref err) => {
            if let Some(error) = unsafe { error.as_mut() } {
                *error = err.into();
            }
            result = false;
        }
    }
    let _ = qiniu_ng_config_builder_t::from(builder);
    result
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_url_frozen_duration(
    builder: qiniu_ng_config_builder_t,
    url_frozen_duration: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder
        .domains_manager_builder
        .url_frozen_duration(Duration::from_secs(url_frozen_duration));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_resolutions_cache_lifetime(
    builder: qiniu_ng_config_builder_t,
    resolutions_cache_lifetime: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder
        .domains_manager_builder
        .resolutions_cache_lifetime(Duration::from_secs(resolutions_cache_lifetime));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_disable_url_resolution(builder: qiniu_ng_config_builder_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder.domains_manager_builder.disable_url_resolution();
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_enable_url_resolution(builder: qiniu_ng_config_builder_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder.domains_manager_builder.enable_url_resolution();
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_auto_persistent_interval(
    builder: qiniu_ng_config_builder_t,
    persistent_interval: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder
        .domains_manager_builder
        .auto_persistent_interval(Duration::from_secs(persistent_interval));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_disable_auto_persistent(builder: qiniu_ng_config_builder_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder.domains_manager_builder.disable_auto_persistent();
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_url_resolve_retries(
    builder: qiniu_ng_config_builder_t,
    url_resolve_retries: size_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder.domains_manager_builder.url_resolve_retries(url_resolve_retries);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_url_resolve_retry_delay(
    builder: qiniu_ng_config_builder_t,
    url_resolve_retry_delay: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder
        .domains_manager_builder
        .url_resolve_retry_delay(Duration::from_secs(url_resolve_retry_delay));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_persistent_file_path(
    builder: qiniu_ng_config_builder_t,
    persistent_file_path: *const qiniu_ng_char_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    let mut result = true;
    if let Some(persistent_file_path) =
        unsafe { persistent_file_path.as_ref() }.map(|file| unsafe { UCString::from_ptr(file) }.into_path_buf())
    {
        match OpenOptions::new().write(true).create(true).open(&persistent_file_path) {
            Ok(persistent_file) => {
                builder.domains_manager_builder = builder
                    .domains_manager_builder
                    .persistent_file(persistent_file, persistent_file_path);
            }
            Err(ref e) => {
                if let Some(error) = unsafe { error.as_mut() } {
                    *error = e.into();
                }
                result = false;
            }
        }
    } else {
        builder.domains_manager_builder = builder.domains_manager_builder.disable_persistent();
    }
    let _ = qiniu_ng_config_builder_t::from(builder);
    result
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_pre_resolve_url(
    builder: qiniu_ng_config_builder_t,
    pre_resolve_url: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder
        .domains_manager_builder
        .pre_resolve_url(unsafe { ucstr::from_ptr(pre_resolve_url) }.to_string().unwrap());
    let _ = qiniu_ng_config_builder_t::from(builder);
}

struct QiniuNgHTTPBeforeActionHandler {
    handler: fn(request: qiniu_ng_http_request_t) -> bool,
}

impl QiniuNgHTTPBeforeActionHandler {
    fn new(handler: fn(request: qiniu_ng_http_request_t) -> bool) -> Self {
        QiniuNgHTTPBeforeActionHandler { handler }
    }
}

impl HTTPBeforeAction for QiniuNgHTTPBeforeActionHandler {
    fn before_call(&self, request: &mut HTTPRequest) -> HTTPResult<()> {
        let request = qiniu_ng_http_request_t::from(Box::new(request));
        if (self.handler)(request) {
            Ok(())
        } else {
            Err(HTTPError::new_unretryable_error(
                HTTPErrorKind::UserCanceled,
                &Box::<&HTTPRequest>::from(request),
                None,
            ))
        }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_append_http_request_before_action_handler(
    builder: qiniu_ng_config_builder_t,
    handler: fn(request: qiniu_ng_http_request_t) -> bool,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .append_http_request_before_action_handler(QiniuNgHTTPBeforeActionHandler::new(handler));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_prepend_http_request_before_action_handler(
    builder: qiniu_ng_config_builder_t,
    handler: fn(request: qiniu_ng_http_request_t) -> bool,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .prepend_http_request_before_action_handler(QiniuNgHTTPBeforeActionHandler::new(handler));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

struct QiniuNgHTTPAfterActionHandler {
    handler: fn(request: qiniu_ng_http_request_t, response: qiniu_ng_http_response_t) -> bool,
}

impl QiniuNgHTTPAfterActionHandler {
    fn new(handler: fn(request: qiniu_ng_http_request_t, response: qiniu_ng_http_response_t) -> bool) -> Self {
        QiniuNgHTTPAfterActionHandler { handler }
    }
}

impl HTTPAfterAction for QiniuNgHTTPAfterActionHandler {
    fn after_call(&self, request: &mut HTTPRequest, response: &mut HTTPResponse) -> HTTPResult<()> {
        let request = qiniu_ng_http_request_t::from(Box::new(request));
        let response = qiniu_ng_http_response_t::from(Box::new(response));
        if (self.handler)(request, response) {
            Ok(())
        } else {
            Err(HTTPError::new_unretryable_error(
                HTTPErrorKind::UserCanceled,
                &Box::<&HTTPRequest>::from(request),
                Some(&Box::<&HTTPResponse>::from(response)),
            ))
        }
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_append_http_request_after_action_handler(
    builder: qiniu_ng_config_builder_t,
    handler: fn(request: qiniu_ng_http_request_t, response: qiniu_ng_http_response_t) -> bool,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .append_http_request_after_action_handler(QiniuNgHTTPAfterActionHandler::new(handler));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_prepend_http_request_after_action_handler(
    builder: qiniu_ng_config_builder_t,
    handler: fn(request: qiniu_ng_http_request_t, response: qiniu_ng_http_response_t) -> bool,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .prepend_http_request_after_action_handler(QiniuNgHTTPAfterActionHandler::new(handler));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_build(
    builder_ptr: *mut qiniu_ng_config_builder_t,
    config: *mut qiniu_ng_config_t,
    error: *mut qiniu_ng_err_t,
) -> bool {
    let builder_ptr = unsafe { builder_ptr.as_mut() }.unwrap();
    let builder = Option::<Box<Builder>>::from(*builder_ptr).unwrap();
    *builder_ptr = qiniu_ng_config_builder_t::default();

    let config_builder = {
        builder
            .config_builder
            .upload_logger(
                match builder
                    .upload_logger_builder
                    .map(|logger_builder| logger_builder.build())
                    .map_or(Ok(None), |result| result.map(Some))
                {
                    Ok(upload_logger) => upload_logger,
                    Err(ref err) => {
                        if let Some(error) = unsafe { error.as_mut() } {
                            *error = err.into();
                        }
                        return false;
                    }
                },
            )
            .upload_recorder(builder.upload_recorder_builder.build())
            .domains_manager(builder.domains_manager_builder.build())
    };

    if let Some(config) = unsafe { config.as_mut() } {
        *config = config_builder.build().into();
    }
    true
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_config_t(*mut c_void);

impl qiniu_ng_config_t {
    #[inline]
    fn new_freed() -> Self {
        Self(null_mut())
    }

    #[inline]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_new_default() -> qiniu_ng_config_t {
    Config::default().into()
}

impl From<qiniu_ng_config_t> for Option<Config> {
    fn from(config: qiniu_ng_config_t) -> Self {
        if config.is_null() {
            None
        } else {
            Some(unsafe { Config::from_raw(transmute(config)) })
        }
    }
}

impl From<Option<Config>> for qiniu_ng_config_t {
    fn from(config: Option<Config>) -> Self {
        config.map(|config| config.into()).unwrap_or_else(Self::new_freed)
    }
}

impl From<Config> for qiniu_ng_config_t {
    fn from(config: Config) -> Self {
        unsafe { transmute(config.into_raw()) }
    }
}

impl qiniu_ng_config_t {
    pub fn get_clone(self) -> Option<Config> {
        let config = Option::<Config>::from(self);
        config.clone().tap(|_| {
            let _: Self = config.into();
        })
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_user_agent(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(config.user_agent()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_appended_user_agent(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(config.appended_user_agent()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_use_https(config: qiniu_ng_config_t) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config.use_https().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uc_host(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(config.uc_host().as_ref()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uc_url(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(config.uc_url()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_rs_host(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(config.rs_host().as_ref()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_rs_url(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(config.rs_url()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_rsf_host(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(config.rsf_host().as_ref()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_rsf_url(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(config.rsf_url()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_api_host(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(config.api_host().as_ref()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_api_url(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(config.api_url()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_host(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(config.uplog_host()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_url(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(config.uplog_url()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_token_lifetime(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.upload_token_lifetime().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_batch_max_operation_size(config: qiniu_ng_config_t) -> size_t {
    let config = Option::<Config>::from(config).unwrap();
    config.batch_max_operation_size().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_threshold(config: qiniu_ng_config_t) -> u32 {
    let config = Option::<Config>::from(config).unwrap();
    config.upload_threshold().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_block_size(config: qiniu_ng_config_t) -> u32 {
    let config = Option::<Config>::from(config).unwrap();
    config.upload_block_size().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_tcp_keepalive_idle_timeout(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.tcp_keepalive_idle_timeout().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_tcp_keepalive_probe_interval(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.tcp_keepalive_probe_interval().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_low_transfer_speed(config: qiniu_ng_config_t) -> u32 {
    let config = Option::<Config>::from(config).unwrap();
    config.http_low_transfer_speed().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_low_transfer_speed_timeout(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.http_low_transfer_speed_timeout().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_connect_timeout(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.http_connect_timeout().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_request_timeout(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.http_request_timeout().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_request_retries(config: qiniu_ng_config_t) -> size_t {
    let config = Option::<Config>::from(config).unwrap();
    config.http_request_retries().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_request_retry_delay(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.http_request_retry_delay().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_is_uplog_enabled(config: qiniu_ng_config_t) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config.upload_logger().is_some().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_file_path(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    config
        .upload_logger()
        .as_ref()
        .map(|upload_logger| {
            qiniu_ng_str_t::from(UCString::from(upload_logger.log_file_path().to_owned()).into_boxed_ucstr())
        })
        .unwrap_or_default()
        .tap(|_| {
            let _ = qiniu_ng_config_t::from(config);
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_file_lock_policy(
    config: qiniu_ng_config_t,
    lock_policy: *mut qiniu_ng_upload_logger_lock_policy_t,
) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config
        .upload_logger()
        .as_ref()
        .map(|upload_logger| {
            if let Some(lock_policy) = unsafe { lock_policy.as_mut() } {
                *lock_policy = upload_logger.lock_policy().into();
            }
            true
        })
        .unwrap_or(false)
        .tap(|_| {
            let _ = qiniu_ng_config_t::from(config);
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_file_upload_threshold(
    config: qiniu_ng_config_t,
    upload_threshold: *mut u32,
) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config
        .upload_logger()
        .as_ref()
        .map(|upload_logger| {
            if let Some(upload_threshold) = unsafe { upload_threshold.as_mut() } {
                *upload_threshold = upload_logger.upload_threshold();
            }
            true
        })
        .unwrap_or(false)
        .tap(|_| {
            let _ = qiniu_ng_config_t::from(config);
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_file_max_size(config: qiniu_ng_config_t, max_size: *mut u32) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config
        .upload_logger()
        .as_ref()
        .map(|upload_logger| {
            if let Some(max_size) = unsafe { max_size.as_mut() } {
                *max_size = upload_logger.max_size();
            }
            true
        })
        .unwrap_or(false)
        .tap(|_| {
            let _ = qiniu_ng_config_t::from(config);
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_recorder_root_directory(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    config
        .upload_recorder()
        .recorder()
        .as_downcastable()
        .downcast_ref::<FileSystemRecorder>()
        .map(|file_system_recorder| {
            qiniu_ng_str_t::from(UCString::from(file_system_recorder.root_directory().to_owned()).into_boxed_ucstr())
        })
        .unwrap_or_default()
        .tap(|_| {
            let _ = qiniu_ng_config_t::from(config);
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_recorder_upload_block_lifetime(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.upload_recorder().upload_block_lifetime().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_recorder_always_flush_records(config: qiniu_ng_config_t) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config.upload_recorder().always_flush_records().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_url_frozen_duration(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.domains_manager().url_frozen_duration().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_resolutions_cache_lifetime(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config
        .domains_manager()
        .resolutions_cache_lifetime()
        .as_secs()
        .tap(|_| {
            let _ = qiniu_ng_config_t::from(config);
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_url_resolution_disabled(config: qiniu_ng_config_t) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config.domains_manager().url_resolution_disabled().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_auto_persistent_interval(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config
        .domains_manager()
        .auto_persistent_interval()
        .map(|interval| interval.as_secs())
        .unwrap_or(0)
        .tap(|_| {
            let _ = qiniu_ng_config_t::from(config);
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_auto_persistent_disabled(config: qiniu_ng_config_t) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config.domains_manager().auto_persistent_disabled().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_url_resolve_retries(config: qiniu_ng_config_t) -> size_t {
    let config = Option::<Config>::from(config).unwrap();
    config.domains_manager().url_resolve_retries().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_url_resolve_retry_delay(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.domains_manager().url_resolve_retry_delay().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_persistent_file_path(
    config: qiniu_ng_config_t,
) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    config
        .domains_manager()
        .persistent_file_path()
        .map(|path| qiniu_ng_str_t::from(UCString::from(path.to_owned()).into_boxed_ucstr()))
        .unwrap_or_default()
        .tap(|_| {
            let _ = qiniu_ng_config_t::from(config);
        })
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_free(config: *mut qiniu_ng_config_t) {
    if let Some(config) = unsafe { config.as_mut() } {
        let _ = Option::<Config>::from(*config);
        *config = qiniu_ng_config_t::new_freed();
    }
}

#[no_mangle]
pub extern "C" fn qiniu_ng_config_is_freed(config: qiniu_ng_config_t) -> bool {
    config.is_null()
}

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_upload_logger_lock_policy_t {
    qiniu_ng_lock_policy_lock_shared_duration_appending_and_lock_exclusive_duration_uploading,
    qiniu_ng_lock_policy_always_lock_exclusive,
    qiniu_ng_lock_policy_none,
}

impl From<qiniu_ng_upload_logger_lock_policy_t> for UploadLoggerFileLockPolicy {
    fn from(policy: qiniu_ng_upload_logger_lock_policy_t) -> Self {
        match policy {
            qiniu_ng_upload_logger_lock_policy_t::qiniu_ng_lock_policy_lock_shared_duration_appending_and_lock_exclusive_duration_uploading => {
                UploadLoggerFileLockPolicy::LockSharedDuringAppendingAndLockExclusiveDuringUploading
            }
            qiniu_ng_upload_logger_lock_policy_t::qiniu_ng_lock_policy_always_lock_exclusive => UploadLoggerFileLockPolicy::AlwaysLockExclusive,
            qiniu_ng_upload_logger_lock_policy_t::qiniu_ng_lock_policy_none => UploadLoggerFileLockPolicy::None,
        }
    }
}

impl From<UploadLoggerFileLockPolicy> for qiniu_ng_upload_logger_lock_policy_t {
    fn from(policy: UploadLoggerFileLockPolicy) -> Self {
        match policy {
            UploadLoggerFileLockPolicy::LockSharedDuringAppendingAndLockExclusiveDuringUploading => {
                qiniu_ng_upload_logger_lock_policy_t::qiniu_ng_lock_policy_lock_shared_duration_appending_and_lock_exclusive_duration_uploading
            }
            UploadLoggerFileLockPolicy::AlwaysLockExclusive => qiniu_ng_upload_logger_lock_policy_t::qiniu_ng_lock_policy_always_lock_exclusive,
            UploadLoggerFileLockPolicy::None => qiniu_ng_upload_logger_lock_policy_t::qiniu_ng_lock_policy_none,
        }
    }
}
