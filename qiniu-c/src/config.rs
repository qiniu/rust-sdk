use crate::{
    http::{qiniu_ng_http_request_t, qiniu_ng_http_response_t},
    result::{qiniu_ng_err_ignore, qiniu_ng_err_t, qiniu_ng_retry_kind_t},
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

/// @brief 七牛客户端配置生成器
/// @note
///   * 调用 `qiniu_ng_config_builder_new()` 函数创建 `qiniu_ng_config_builder_t` 实例。
///   * 调用一系列方法修改 `qiniu_ng_config_builder_t` 实例的数据。
///   * 调用 `qiniu_ng_config_build()` 生成 `qiniu_ng_config_t` 实例。
///   * 当通过 `qiniu_ng_config_builder_t` 生成 `qiniu_ng_config_t` 完毕后，`qiniu_ng_config_builder_t` 的内存将被自动回收，您无需调用 `qiniu_ng_config_builder_free()` 回收内存，也不可能再使用该生成器实例
/// @note
///   该结构体不可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_config_builder_t(*mut c_void);

struct Builder {
    config_builder: ConfigBuilder,
    upload_logger_builder: Option<UploadLoggerBuilder>,
    upload_recorder_builder: UploadRecorderBuilder,
    domains_manager_builder: DomainsManagerBuilder,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            config_builder: Default::default(),
            upload_logger_builder: Some(Default::default()),
            upload_recorder_builder: Default::default(),
            domains_manager_builder: Default::default(),
        }
    }
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

/// @brief 创建客户端配置生成器实例
/// @retval qiniu_ng_config_builder_t 获取创建的客户端配置生成器实例
/// @warning 当通过 `qiniu_ng_config_builder_t` 生成 `qiniu_ng_config_t` 完毕后，`qiniu_ng_config_builder_t` 的内存将被自动回收，您无需调用 `qiniu_ng_config_builder_free()` 回收内存，也不可能再使用该生成器实例
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_new() -> qiniu_ng_config_builder_t {
    Box::new(Builder::default()).into()
}

/// @brief 释放客户端配置生成器实例
/// @param[in,out] builder 客户端配置生成器实例地址，释放完毕后该生成器实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_free(builder: *mut qiniu_ng_config_builder_t) {
    if let Some(builder) = unsafe { builder.as_mut() } {
        let _ = Option::<Box<Builder>>::from(*builder);
        *builder = qiniu_ng_config_builder_t::default();
    }
}

/// @brief 判断客户端配置生成器实例是否已经被释放
/// @param[in] builder 客户端配置生成器实例
/// @retval bool 如果返回 `true` 则表示客户端配置生成器实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_is_freed(builder: qiniu_ng_config_builder_t) -> bool {
    builder.is_null()
}

/// @brief 指定客户端配置的追加用户代理
/// @details SDK 本身会包含预定的用户代理字符串，您不能修改该字符串，但可以向该字符串追加更多内容
/// @param[in] builder 客户端配置生成器实例
/// @param[in] user_agent 追加的用户代理
/// @note 调用该方法时，输入的 `user_agent` 将被复制并存储，因此 `user_agent` 的调用完毕后即可释放
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

/// @brief 指定客户端配置是否使用 HTTPS 协议
/// @param[in] builder 客户端配置生成器实例
/// @param[in] use_https 是否使用 HTTPS 协议
/// @note 默认将会使用 HTTPS 协议
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_use_https(builder: qiniu_ng_config_builder_t, use_https: bool) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder.config_builder.use_https(use_https);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 指定客户端配置中的 UC 服务器地址
/// @param[in] builder 客户端配置生成器实例
/// @param[in] uc_host UC 服务器地址（仅需要指定主机地址和端口，无需包含协议）
/// @note 默认将会使用七牛公有云的 UC 服务器地址，因此仅在使用私有云时才需要配置
/// @note 调用该方法时，输入的 `uc_host` 将被复制并存储，因此 `uc_host` 的调用完毕后即可释放
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_uc_host(builder: qiniu_ng_config_builder_t, uc_host: *const qiniu_ng_char_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .uc_host(unsafe { ucstr::from_ptr(uc_host) }.to_string().unwrap());
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 指定客户端配置中的 RS 服务器地址
/// @param[in] builder 客户端配置生成器实例
/// @param[in] rs_host RS 服务器地址（仅需要指定主机地址和端口，无需包含协议）
/// @note 默认将会使用七牛公有云的 RS 服务器地址，因此仅在使用私有云时才需要配置
/// @note 调用该方法时，输入的 `rs_host` 将被复制并存储，因此 `rs_host` 的调用完毕后即可释放
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_rs_host(builder: qiniu_ng_config_builder_t, rs_host: *const qiniu_ng_char_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .rs_host(unsafe { ucstr::from_ptr(rs_host) }.to_string().unwrap());
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 指定客户端配置中的 RSF 服务器地址
/// @param[in] builder 客户端配置生成器实例
/// @param[in] rsf_host RSF 服务器地址（仅需要指定主机地址和端口，无需包含协议）
/// @note 默认将会使用七牛公有云的 RSF 服务器地址，因此仅在使用私有云时才需要配置
/// @note 调用该方法时，输入的 `rsf_host` 将被复制并存储，因此 `rsf_host` 的调用完毕后即可释放
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

/// @brief 指定客户端配置中的 API 服务器地址
/// @param[in] builder 客户端配置生成器实例
/// @param[in] api_host API 服务器地址（仅需要指定主机地址和端口，无需包含协议）
/// @note 默认将会使用七牛公有云的 API 服务器地址，因此仅在使用私有云时才需要配置
/// @note 调用该方法时，输入的 `api_host` 将被复制并存储，因此 `api_host` 的调用完毕后即可释放
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

/// @brief 指定客户端配置中的 UpLog 服务器地址
/// @param[in] builder 客户端配置生成器实例
/// @param[in] uplog_host UpLog 服务器地址（仅需要指定主机地址和端口，无需包含协议）
/// @note 默认将会使用七牛公有云的 UpLog 服务器地址，因此仅在使用私有云时才需要配置
/// @note 调用该方法时，输入的 `uplog_host` 将被复制并存储，因此 `uplog_host` 的调用完毕后即可释放
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

/// @brief 指定客户端配置中的上传凭证有效期
/// @param[in] builder 客户端配置生成器实例
/// @param[in] upload_token_lifetime 上传凭证有效期
/// @note 默认为 1 小时
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

/// @brief 指定客户端配置中的最大批量操作数
/// @param[in] builder 客户端配置生成器实例
/// @param[in] batch_max_operation_size 最大批量操作数
/// @note 默认为 1000
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

/// @brief 指定客户端配置中的分片上传策略阙值
/// @details 如果上传文件尺寸大于该值，将自动使用分片上传，否则，使用表单上传
/// @param[in] builder 客户端配置生成器实例
/// @param[in] upload_threshold 阙值，单位为字节
/// @note 默认为 4 MB
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_threshold(builder: qiniu_ng_config_builder_t, upload_threshold: u32) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder.config_builder.upload_threshold(upload_threshold);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 指定客户端配置中的上传分块尺寸
/// @param[in] builder 客户端配置生成器实例
/// @param[in] upload_block_size 上传分块尺寸，单位为字节
/// @note 默认为 4 MB
/// @note 尺寸越小越适合弱网环境
/// @warning 必须是 4 MB 的倍数
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_block_size(
    builder: qiniu_ng_config_builder_t,
    upload_block_size: u32,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder.config_builder.upload_block_size(upload_block_size);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 指定客户端配置中的 HTTP 请求连接超时时长
/// @details 对 SDK 所有发出的 HTTP 请求均有效
/// @param[in] builder 客户端配置生成器实例
/// @param[in] http_connect_timeout 超时时长，单位为秒
/// @note 默认为 5 秒
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

/// @brief 指定客户端配置中的 HTTP 请求超时时长
/// @details 对 SDK 所有发出的 HTTP 请求均有效
/// @param[in] builder 客户端配置生成器实例
/// @param[in] http_request_timeout 超时时长，单位为秒
/// @note 默认为 5 分钟
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

/// @brief 指定客户端配置中的 TCP KeepAlive 空闲时长
/// @details 对 SDK 所有发出的 HTTP 请求均有效
/// @param[in] builder 客户端配置生成器实例
/// @param[in] tcp_keepalive_idle_timeout 空闲时长，单位为秒
/// @note 默认为 5 分钟
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

/// @brief 指定客户端配置中的 TCP KeepAlive 探测包的发送间隔
/// @details 对 SDK 所有发出的 HTTP 请求均有效
/// @param[in] builder 客户端配置生成器实例
/// @param[in] tcp_keepalive_probe_interval 发送间隔，单位为秒
/// @note 默认为 5 秒
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

/// @brief 指定客户端配置中的 HTTP 最低传输速度
/// @details 对 SDK 所有发出的 HTTP 请求均有效
/// @details
///     与 `http_low_transfer_speed_timeout` 配合使用。
///     当 HTTP 传输速度低于最低传输速度 `http_low_transfer_speed` 并维持超过 `http_low_transfer_speed_timeout` 的时长，则出错。
///     SDK 会自动重试，或出错退出
/// @param[in] builder 客户端配置生成器实例
/// @param[in] low_transfer_speed 最低传输速度，单位为字节/秒
/// @note 默认为 1024 字节/秒
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_http_low_transfer_speed(
    builder: qiniu_ng_config_builder_t,
    low_transfer_speed: u32,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder.config_builder.http_low_transfer_speed(low_transfer_speed);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 指定客户端配置中的 HTTP 最低传输速度维持时长
/// @details 对 SDK 所有发出的 HTTP 请求均有效
/// @details
///     与 `http_low_transfer_speed` 配合使用。
///     当 HTTP 传输速度低于最低传输速度 `http_low_transfer_speed` 并维持超过 `http_low_transfer_speed_timeout` 的时长，则出错。
///     SDK 会自动重试，或出错退出
/// @param[in] builder 客户端配置生成器实例
/// @param[in] low_transfer_speed_timeout 最低传输速度，单位为秒
/// @note 默认为 30 秒
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

/// @brief 指定客户端配置中的 HTTP 请求重试次数
/// @details 对 SDK 所有发出的 HTTP 请求均有效
/// @details 当 SDK 发送 HTTP 请求时发生错误，且该错误可以通过重试来解决时，SDK 将重试的次数
/// @param[in] builder 客户端配置生成器实例
/// @param[in] http_request_retries 重试次数
/// @note 默认为 3 次
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_http_request_retries(
    builder: qiniu_ng_config_builder_t,
    http_request_retries: size_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder.config_builder.http_request_retries(http_request_retries);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 指定客户端配置中的 HTTP 请求重试前等待时间
/// @details 对 SDK 所有发出的 HTTP 请求均有效
/// @details
///     当 SDK 发送 HTTP 请求时发生错误，且该错误可以通过重试来解决时，SDK 将等待一段时间并且重试
///     每次实际等待时长为该项值的 50% - 100% 之间的随机时长
/// @param[in] builder 客户端配置生成器实例
/// @param[in] http_request_retry_delay 等待时间，单位为秒
/// @note 默认为 1 秒，也就是说每次等待 500 毫秒至 1 秒间不等
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

/// @brief 禁用上传日志记录仪
/// @param[in] builder 客户端配置生成器实例
/// @note 默认上传日志记录仪将被启用
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_disable_uplog(builder: qiniu_ng_config_builder_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.upload_logger_builder = None;
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 启用上传日志记录仪
/// @param[in] builder 客户端配置生成器实例
/// @note 默认上传日志记录仪将被启用
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_enable_uplog(builder: qiniu_ng_config_builder_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.upload_logger_builder = Some(Default::default());
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 设置日志文件路径
/// @param[in] builder 客户端配置生成器实例
/// @param[in] file_path 日志文件路径
/// @details
///     默认的日志文件路径规则如下：
///       1. 尝试在[操作系统特定的缓存目录](https://docs.rs/dirs/2.0.2/dirs/fn.cache_dir.html)下创建 `qiniu_sdk` 目录。
///       2. 如果成功，则使用 `qiniu_sdk` 目录下的 `upload.log`。
///       3. 如果失败，则直接使用临时目录下的 `upload.log`。
/// @note 调用该方法时，输入的 `file_path` 将被复制并存储，因此 `file_path` 的调用完毕后即可释放
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

/// @brief 设置日志文件锁策略
/// @param[in] builder 客户端配置生成器实例
/// @param[in] lock_policy 日志文件锁策略
/// @details
///     为了防止上传文件的过程中，上传日志文件被多个进程同时修改引发竞争，因此需要在操作日志文件时使用文件锁保护。
///     默认策略为在追加日志时为日志文件加共享锁，而上传时使用排他锁，尽可能做到安全和性能之间的平衡。
///
///     但在有些场景下中，并发追加日志文件同样会引发竞争，此时需要改用 `qiniu_ng_lock_policy_always_lock_exclusive` 策略。
///     此外，如果确定当前操作系统内不会有多个进程同时上传文件，或不同进程不会使用相同路径的日志时，
///     也可以使用 `qiniu_ng_lock_policy_none` 策略，减少文件锁的性能影响。
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

/// @brief 设置日志文件的上传阙值
/// @details 当且仅当日志文件尺寸大于阙值时才会上传日志
/// @param[in] builder 客户端配置生成器实例
/// @param[in] upload_threshold 上传阙值，单位为字节
/// @note 默认为 4 KB
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

/// @brief 设置日志文件的最大尺寸
/// @details
///     当日志文件尺寸大于指定尺寸时，将不会再记录任何数据到日志内。
///     防止在上传发生困难时日志文件无限制膨胀。
/// @param[in] builder 客户端配置生成器实例
/// @param[in] max_size 日志文件最大尺寸，单位为字节
/// @warning 该值必须大于 `upload_threshold`
/// @note 默认为 4 MB
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_uplog_file_max_size(builder: qiniu_ng_config_builder_t, max_size: u32) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.upload_logger_builder = Some(builder.upload_logger_builder.unwrap_or_default().max_size(max_size));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 设置上传进度记录仪文件根目录
/// @details 每个上传的文件都会有一个对应的上传进度记录仪文件，因此需要设置根目录存储
/// @param[in] builder 客户端配置生成器实例
/// @param[in] root_directory 文件根目录
/// @details
///     默认的文件系统记录仪目录规则如下：
///       1. 尝试在[操作系统特定的缓存目录](https://docs.rs/dirs/2.0.2/dirs/fn.cache_dir.html)下创建 `qiniu_sdk/records` 目录。
///       2. 如果成功，则使用 `qiniu_sdk/records` 目录。
///       3. 如果失败，则直接使用临时目录。
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_recorder_root_directory(
    builder: qiniu_ng_config_builder_t,
    root_directory: *const qiniu_ng_char_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    let recorder = FileSystemRecorder::from(unsafe { UCString::from_ptr(root_directory) }.into_path_buf());
    builder.upload_recorder_builder.recorder(recorder);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 设置文件分块有效期
/// @details 对于超过有效期的分块，SDK 将重新上传，确保所有分块在创建文件时均有效
/// @param[in] builder 客户端配置生成器实例
/// @param[in] upload_block_lifetime 文件分块有效期，单位为秒
/// @note 默认为 7 天，这是七牛公有云默认的配置。对于私有云的情况，需要参照私有云的配置来设置。
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_recorder_upload_block_lifetime(
    builder: qiniu_ng_config_builder_t,
    upload_block_lifetime: u64,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder
        .upload_recorder_builder
        .upload_block_lifetime(Duration::from_secs(upload_block_lifetime));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 设置进度记录文件始终刷新
/// @details 当记录上传进度后，是否始终刷新 IO 确保数据已经被持久化
/// @param[in] builder 客户端配置生成器实例
/// @param[in] always_flush_records 是否始终刷新
/// @note 默认为否
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_upload_recorder_always_flush_records(
    builder: qiniu_ng_config_builder_t,
    always_flush_records: bool,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder
        .upload_recorder_builder
        .always_flush_records(always_flush_records);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 从指定路径加载域名管理器
/// @details 加载后，该路径将作为域名管理器的持久化路径
/// @param[in] builder 客户端配置生成器实例
/// @param[in] persistent_file 域名管理文件路径
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示加载正确，如果返回 `false`，则表示可以读取 `error` 获得错误信息
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

/// @brief 创建一个新的域名管理器
/// @param[in] builder 客户端配置生成器实例
/// @param[in] persistent_file 新的域名管理器的持久化路径
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否运行正常，如果返回 `true`，则表示创建正常，如果返回 `false`，则表示可以读取 `error` 获得错误信息
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

/// @brief 设置域名管理器的 URL 冻结时长
/// @details
///     当 SDK 发送 HTTP 请求时，如果发现网络或服务异常，靠重试无法解决的，则冻结所访问的服务器 URL。
///     被冻结的服务器在冻结期间将无法被访问
/// @param[in] builder 客户端配置生成器实例
/// @param[in] url_frozen_duration URL 冻结时长，单位为秒
/// @note 默认冻结十分钟
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

/// @brief 设置域名管理器的域名解析缓存生命周期
/// @param[in] builder 客户端配置生成器实例
/// @param[in] resolutions_cache_lifetime 域名解析缓存生命周期，单位为秒
/// @note 默认缓存一小时
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

/// @brief 设置域名管理器禁用 URL 域名预解析
/// @param[in] builder 客户端配置生成器实例
/// @note 默认启用 URL 域名预解析
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_disable_url_resolution(builder: qiniu_ng_config_builder_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder.domains_manager_builder.disable_url_resolution();
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 设置域名管理器启用 URL 域名预解析
/// @param[in] builder 客户端配置生成器实例
/// @note 默认启用 URL 域名预解析
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_enable_url_resolution(builder: qiniu_ng_config_builder_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder.domains_manager_builder.enable_url_resolution();
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 设置域名管理器的自动持久化间隔时间
/// @details 当自动持久化被启用，且存在持久化路径时，域名管理器将定期自动保存自身状态
/// @param[in] builder 客户端配置生成器实例
/// @param[in] persistent_interval 间隔时间，单位为秒
/// @note 默认间隔时间为三十分钟
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

/// @brief 设置域名管理器禁止自动持久化
/// @param[in] builder 客户端配置生成器实例
/// @note 默认启用自动持久化
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_disable_auto_persistent(builder: qiniu_ng_config_builder_t) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder.domains_manager_builder.disable_auto_persistent();
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 设置域名管理器的 URL 域名预解析重试次数
/// @details 当 SDK 预解析域名时发送错误时，SDK 将重试的次数
/// @param[in] builder 客户端配置生成器实例
/// @param[in] url_resolve_retries 重试次数
/// @note 默认为 10 次
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_domains_manager_url_resolve_retries(
    builder: qiniu_ng_config_builder_t,
    url_resolve_retries: size_t,
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.domains_manager_builder = builder.domains_manager_builder.url_resolve_retries(url_resolve_retries);
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 指定域名管理器的 URL 域名预解析重试前等待时间
/// @details
///     当 SDK 预解析域名时发送错误时，SDK 将等待一段时间并且重试。
///     每次实际等待时长为该项值的 50% - 100% 之间的随机时长。
/// @param[in] builder 客户端配置生成器实例
/// @param[in] url_resolve_retry_delay 等待时间，单位为秒
/// @note 默认为 1 秒，也就是说每次等待 500 毫秒至 1 秒间不等
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

/// @brief 设置域名管理器的持久化路径
/// @param[in] builder 客户端配置生成器实例
/// @param[in] persistent_file_path 持久化路径，如果传入 `NULL` 则表示禁止持久化
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否设置正常，如果返回 `false`，则表示可以读取 `error` 获得错误信息
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

/// @brief 添加域名预解析 URL
/// @details
///     当客户端配置生成器生成前，可以指定多个预解析 URL 域名。
///     而生成时，将以异步的方式预解析 URL 域名，并将结果缓存在域名管理器内
/// @param[in] builder 客户端配置生成器实例
/// @param[in] pre_resolve_url 域名预解析 URL
/// @note 调用该方法时，输入的 `pre_resolve_url` 将被复制并存储，因此 `pre_resolve_url` 的调用完毕后即可释放
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

/// @brief HTTP 处理相关错误结构体
/// @details 该结构体封装 HTTP 处理相关错误字段
/// @note 无需对该结构体进行内存释放
/// @note 该结构体不可以跨线程使用
#[repr(C)]
#[derive(Copy, Clone)]
pub struct qiniu_ng_callback_err_t {
    /// @brief 错误信息，查看 `qiniu_ng_err_t` 相关创建函数
    pub error: qiniu_ng_err_t,
    /// @brief 重试类型，查看 `qiniu_ng_retry_kind_t` 的文档了解相关信息
    pub retry_kind: qiniu_ng_retry_kind_t,
    /// @brief 是否重试安全
    pub is_retry_safe: bool,
}

impl Default for qiniu_ng_callback_err_t {
    fn default() -> Self {
        Self {
            error: Default::default(),
            retry_kind: qiniu_ng_retry_kind_t::qiniu_ng_retry_kind_unretryable_error,
            is_retry_safe: false,
        }
    }
}

type QiniuNgHTTPBeforeActionFunc = fn(request: qiniu_ng_http_request_t, err: *mut qiniu_ng_callback_err_t);

struct QiniuNgHTTPBeforeActionHandler {
    handler: QiniuNgHTTPBeforeActionFunc,
}

impl QiniuNgHTTPBeforeActionHandler {
    fn new(handler: QiniuNgHTTPBeforeActionFunc) -> Self {
        QiniuNgHTTPBeforeActionHandler { handler }
    }
}

impl HTTPBeforeAction for QiniuNgHTTPBeforeActionHandler {
    fn before_call(&self, request: &mut HTTPRequest) -> HTTPResult<()> {
        let request = qiniu_ng_http_request_t::from(request);
        let mut err = qiniu_ng_callback_err_t::default();
        (self.handler)(request, &mut err);
        if let Some(e) = Option::<HTTPErrorKind>::from(&err.error) {
            qiniu_ng_err_ignore(&mut err.error);
            Err(HTTPError::new(
                err.retry_kind.into(),
                e,
                err.is_retry_safe,
                request.into(),
                None,
            ))
        } else {
            Ok(())
        }
    }
}

/// @brief 追加 HTTP 请求前回调函数
/// @details
///     您可以利用该特性输出 HTTP 日志或对 HTTP 请求内容进行修改。
///     但注意，您必须确保不破坏请求中必要的内容，否则七牛服务器可能无法处理该请求。
/// @param[in] builder 客户端配置生成器实例
/// @param[in] handler 回调函数。回调函数的第一个参数是即将发送的 HTTP 请求，可以参考 `qiniu_ng_http_request_t` 的文档了解其用法。而第二个参数则用来填充具体的错误信息，可以参考 `qiniu_ng_callback_err_t` 的文档了解其用法，如果没有错误，则无需修改 `err` 参数的值
/// @note 如果发生错误，您需要调用 `qiniu_ng_err_t` 的创建函数对 `err` 中的 `error` 字段赋值，但内存释放函数无需您调用，将由 SDK 负责内存回收
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_append_http_request_before_action_handler(
    builder: qiniu_ng_config_builder_t,
    handler: fn(request: qiniu_ng_http_request_t, err: *mut qiniu_ng_callback_err_t),
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .append_http_request_before_action_handler(QiniuNgHTTPBeforeActionHandler::new(handler));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 增加 HTTP 请求前回调函数
/// @details
///     您可以利用该特性输出 HTTP 日志或对 HTTP 请求内容进行修改。
///     但注意，您必须确保不破坏请求中必要的内容，否则七牛服务器可能无法处理该请求。
/// @param[in] builder 客户端配置生成器实例
/// @param[in] handler 回调函数。回调函数的第一个参数是即将发送的 HTTP 请求，可以参考 `qiniu_ng_http_request_t` 的文档了解其用法。而第二个参数则用来填充具体的错误信息，可以参考 `qiniu_ng_callback_err_t` 的文档了解其用法，如果没有错误，则无需修改 `err` 参数的值
/// @note 如果发生错误，您需要调用 `qiniu_ng_err_t` 的创建函数对 `err` 中的 `error` 字段赋值，但内存释放函数无需您调用，将由 SDK 负责内存回收
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_prepend_http_request_before_action_handler(
    builder: qiniu_ng_config_builder_t,
    handler: fn(request: qiniu_ng_http_request_t, err: *mut qiniu_ng_callback_err_t),
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .prepend_http_request_before_action_handler(QiniuNgHTTPBeforeActionHandler::new(handler));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

type QiniuNgHTTPAfterActionFunc =
    fn(request: qiniu_ng_http_request_t, response: qiniu_ng_http_response_t, err: *mut qiniu_ng_callback_err_t);

struct QiniuNgHTTPAfterActionHandler {
    handler: QiniuNgHTTPAfterActionFunc,
}

impl QiniuNgHTTPAfterActionHandler {
    fn new(
        handler: fn(
            request: qiniu_ng_http_request_t,
            response: qiniu_ng_http_response_t,
            err: *mut qiniu_ng_callback_err_t,
        ),
    ) -> Self {
        QiniuNgHTTPAfterActionHandler { handler }
    }
}

impl HTTPAfterAction for QiniuNgHTTPAfterActionHandler {
    fn after_call(&self, request: &mut HTTPRequest, response: &mut HTTPResponse) -> HTTPResult<()> {
        let request = qiniu_ng_http_request_t::from(request);
        let response = qiniu_ng_http_response_t::from(response);
        let mut err = qiniu_ng_callback_err_t::default();
        (self.handler)(request, response, &mut err);
        if let Some(e) = Option::<HTTPErrorKind>::from(&err.error) {
            qiniu_ng_err_ignore(&mut err.error);
            Err(HTTPError::new(
                err.retry_kind.into(),
                e,
                err.is_retry_safe,
                request.into(),
                Some(response.into()),
            ))
        } else {
            Ok(())
        }
    }
}

/// @brief 追加 HTTP 响应后回调函数
/// @details
///     您可以利用该特性输出 HTTP 日志或对 HTTP 响应内容进行修改。
///     但注意，您必须确保不破坏响应中必要的内容，否则 SDK 可能无法处理该响应。
/// @param[in] builder 客户端配置生成器实例
/// @param[in] handler 回调函数。回调函数的第一个参数是即将发送的 HTTP 请求，可以参考 `qiniu_ng_http_request_t` 的文档了解其用法，回调函数的第二个参数是即将发送的 HTTP 请求，可以参考 `qiniu_ng_http_response_t` 的文档了解其用法，而第三个参数则用来填充具体的错误信息，可以参考 `qiniu_ng_callback_err_t` 的文档了解其用法，如果没有错误，则无需修改 `err` 参数的值
/// @note 如果发生错误，您需要调用 `qiniu_ng_err_t` 的创建函数对 `err` 中的 `error` 字段赋值，但内存释放函数无需您调用，将由 SDK 负责内存回收
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_append_http_request_after_action_handler(
    builder: qiniu_ng_config_builder_t,
    handler: fn(
        request: qiniu_ng_http_request_t,
        response: qiniu_ng_http_response_t,
        err: *mut qiniu_ng_callback_err_t,
    ),
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .append_http_request_after_action_handler(QiniuNgHTTPAfterActionHandler::new(handler));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 新增 HTTP 响应后回调函数
/// @details
///     您可以利用该特性输出 HTTP 日志或对 HTTP 响应内容进行修改。
///     但注意，您必须确保不破坏响应中必要的内容，否则 SDK 可能无法处理该响应。
/// @param[in] builder 客户端配置生成器实例
/// @param[in] handler 回调函数。回调函数的第一个参数是即将发送的 HTTP 请求，可以参考 `qiniu_ng_http_request_t` 的文档了解其用法，回调函数的第二个参数是即将发送的 HTTP 请求，可以参考 `qiniu_ng_http_response_t` 的文档了解其用法，而第三个参数则用来填充具体的错误信息，可以参考 `qiniu_ng_callback_err_t` 的文档了解其用法，如果没有错误，则无需修改 `err` 参数的值
/// @note 如果发生错误，您需要调用 `qiniu_ng_err_t` 的创建函数对 `err` 中的 `error` 字段赋值，但内存释放函数无需您调用，将由 SDK 负责内存回收
#[no_mangle]
pub extern "C" fn qiniu_ng_config_builder_prepend_http_request_after_action_handler(
    builder: qiniu_ng_config_builder_t,
    handler: fn(
        request: qiniu_ng_http_request_t,
        response: qiniu_ng_http_response_t,
        err: *mut qiniu_ng_callback_err_t,
    ),
) {
    let mut builder = Option::<Box<Builder>>::from(builder).unwrap();
    builder.config_builder = builder
        .config_builder
        .prepend_http_request_after_action_handler(QiniuNgHTTPAfterActionHandler::new(handler));
    let _ = qiniu_ng_config_builder_t::from(builder);
}

/// @brief 生成客户端配置实例
/// @param[in] builder_ptr 客户端配置生成器实例
/// @param[out] config 用来返回客户端配置实例，如果传入 `NULL` 表示不获取 `config`。但如果运行正常，返回值将依然是 `true`
/// @param[out] error 用于返回错误，如果传入 `NULL` 表示不获取 `error`。但如果运行发生错误，返回值将依然是 `false`
/// @retval bool 是否生成正常，如果返回 `true`，则表示可以读取 `config` 获得生成的客户端配置实例，如果返回 `false`，则表示可以读取 `error` 获得错误信息
/// @warning 务必在使用 `qiniu_ng_config_t` 完毕后调用 `qiniu_ng_config_free()` 方法释放 `qiniu_ng_config_t`
/// @warning 在调用完毕后 `qiniu_ng_config_builder_t` 无需被 `qiniu_ng_config_builder_free()` 释放，且该生成器实例将不再可用
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

/// @brief 客户端配置
/// @details 提供客户端必要的配置信息
/// @note
///   * 调用 `qiniu_ng_config_new_default()` 的方法，或使用 `qiniu_ng_config_builder_t` 生成器生成 `qiniu_ng_config_t` 实例。
///   * 当 `qiniu_ng_config_t` 使用完毕后，请务必调用 `qiniu_ng_config_free()` 方法释放内存。
/// @note
///   所有客户端配置均为只读，因此可以跨线程使用
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

/// @brief 创建客户端配置生成器实例
/// @retval qiniu_ng_config_builder_t 获取创建的客户端配置生成器实例
/// @warning 务必在使用完毕后调用 `qiniu_ng_config_builder_free()` 方法释放 `qiniu_ng_config_builder_t`
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

/// @brief 获取客户端配置的用户代理
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t 用户代理
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_user_agent(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(config.user_agent()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的追加用户代理
/// @details 这里的追加用户代理指的是通过 `qiniu_ng_config_builder_set_appended_user_agent()` 函数设置的追加用户代理
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t 追加用户代理
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_appended_user_agent(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_optional_str_unchecked(config.appended_user_agent()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 客户端配置是否使用 HTTPS 协议
/// @param[in] config 客户端配置实例
/// @retval bool 是否使用 HTTPS 协议
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_use_https(config: qiniu_ng_config_t) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config.use_https().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 UC 服务器地址
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t UC 服务器地址
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uc_host(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(config.uc_host().as_ref()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 UC 服务器 URL
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t UC 服务器 URL
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uc_url(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(config.uc_url()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 RS 服务器地址
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t RS 服务器地址
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_rs_host(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(config.rs_host().as_ref()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 RS 服务器 URL
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t RS 服务器 URL
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_rs_url(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(config.rs_url()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 RSF 服务器地址
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t RSF 服务器地址
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_rsf_host(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(config.rsf_host().as_ref()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 RSF 服务器 URL
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t RSF 服务器 URL
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_rsf_url(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(config.rsf_url()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 API 服务器地址
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t API 服务器地址
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_api_host(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(config.api_host().as_ref()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 API 服务器 URL
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t API 服务器 URL
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_api_url(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(config.api_url()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 UpLog 服务器地址
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t UpLog 服务器地址
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_host(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_str_unchecked(config.uplog_host()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 UpLog 服务器 URL
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t UpLog 服务器 URL
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_uplog_url(config: qiniu_ng_config_t) -> qiniu_ng_str_t {
    let config = Option::<Config>::from(config).unwrap();
    unsafe { qiniu_ng_str_t::from_string_unchecked(config.uplog_url()) }.tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的上传凭证有效期
/// @param[in] config 客户端配置实例
/// @retval uint64_t 上传凭证有效期，单位为秒
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_token_lifetime(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.upload_token_lifetime().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的最大批量操作数
/// @param[in] config 客户端配置实例
/// @retval size_t 最大批量操作数
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_batch_max_operation_size(config: qiniu_ng_config_t) -> size_t {
    let config = Option::<Config>::from(config).unwrap();
    config.batch_max_operation_size().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的分片上传策略阙值
/// @param[in] config 客户端配置实例
/// @retval uint32_t 分片上传策略阙值，单位为字节
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_threshold(config: qiniu_ng_config_t) -> u32 {
    let config = Option::<Config>::from(config).unwrap();
    config.upload_threshold().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的上传分块尺寸
/// @param[in] config 客户端配置实例
/// @retval uint32_t 上传分块尺寸，单位为字节
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_block_size(config: qiniu_ng_config_t) -> u32 {
    let config = Option::<Config>::from(config).unwrap();
    config.upload_block_size().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 TCP KeepAlive 空闲时长
/// @param[in] config 客户端配置实例
/// @retval uint64_t TCP KeepAlive 空闲时长，单位为秒
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_tcp_keepalive_idle_timeout(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.tcp_keepalive_idle_timeout().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 TCP KeepAlive 探测包的发送间隔
/// @param[in] config 客户端配置实例
/// @retval uint64_t TCP KeepAlive 探测包的发送间隔，单位为秒
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_tcp_keepalive_probe_interval(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.tcp_keepalive_probe_interval().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 HTTP 最低传输速度
/// @details
///     与 `http_low_transfer_speed_timeout` 配合使用。
///     当 HTTP 传输速度低于最低传输速度 `http_low_transfer_speed` 并维持超过 `http_low_transfer_speed_timeout` 的时长，则出错。
///     SDK 会自动重试，或出错退出
/// @param[in] config 客户端配置实例
/// @retval uint32_t HTTP 最低传输速度，单位为字节/秒
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_low_transfer_speed(config: qiniu_ng_config_t) -> u32 {
    let config = Option::<Config>::from(config).unwrap();
    config.http_low_transfer_speed().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 HTTP 最低传输速度维持时长
/// @details
///     与 `http_low_transfer_speed` 配合使用。
///     当 HTTP 传输速度低于最低传输速度 `http_low_transfer_speed` 并维持超过 `http_low_transfer_speed_timeout` 的时长，则出错。
///     SDK 会自动重试，或出错退出
/// @param[in] config 客户端配置实例
/// @retval uint64_t HTTP 最低传输速度维持时长，单位为秒
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_low_transfer_speed_timeout(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.http_low_transfer_speed_timeout().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 HTTP 请求连接超时时长
/// @param[in] config 客户端配置实例
/// @retval uint64_t HTTP 请求连接超时时长，单位为秒
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_connect_timeout(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.http_connect_timeout().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 HTTP 请求超时时长
/// @param[in] config 客户端配置实例
/// @retval uint64_t HTTP 请求超时时长，单位为秒
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_request_timeout(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.http_request_timeout().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 HTTP 请求重试次数
/// @details 当 SDK 发送 HTTP 请求时发生错误，且该错误可以通过重试来解决时，SDK 将重试的次数
/// @param[in] config 客户端配置实例
/// @retval size_t HTTP 请求重试次数
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_request_retries(config: qiniu_ng_config_t) -> size_t {
    let config = Option::<Config>::from(config).unwrap();
    config.http_request_retries().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置的 HTTP 请求重试前等待时间
/// @details
///     当 SDK 发送 HTTP 请求时发生错误，且该错误可以通过重试来解决时，SDK 将等待一段时间并且重试
///     每次实际等待时长为该项值的 50% - 100% 之间的随机时长
/// @param[in] config 客户端配置实例
/// @retval uint64_t HTTP 请求重试前等待时间，单位为秒
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_http_request_retry_delay(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.http_request_retry_delay().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 客户端配置是否启用上传日志记录仪
/// @param[in] config 客户端配置实例
/// @retval bool 是否启用上传日志记录仪
#[no_mangle]
pub extern "C" fn qiniu_ng_config_is_uplog_enabled(config: qiniu_ng_config_t) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config.upload_logger().is_some().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置中的上传日志文件路径
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t 上传日志文件路径
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
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

/// @brief 获取客户端配置中的上传日志文件锁策略
/// @param[in] config 客户端配置实例
/// @param[out] lock_policy 用于返回上传日志文件锁策略。如果传入 `NULL` 表示不获取 `lock_policy`。但如果上传日志已经启用，返回值将依然是 `true`
/// @retval bool 如果上传日志已经启用，锁策略必然存在，则返回 `true`，否则返回 `false`
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

/// @brief 获取客户端配置中的上传日志文件的上传阙值
/// @param[in] config 客户端配置实例
/// @param[out] upload_threshold 用于返回上传日志文件的上传阙值，单位为字节。如果传入 `NULL` 表示不获取 `upload_threshold`。但如果上传日志已经启用，返回值将依然是 `true`
/// @retval bool 如果上传日志已经启用，上传阙值必然存在，则返回 `true`，否则返回 `false`
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

/// @brief 获取客户端配置中的上传日志文件的最大尺寸
/// @param[in] config 客户端配置实例
/// @param[out] max_size 用于返回上传日志文件的最大尺寸，单位为字节。如果传入 `NULL` 表示不获取 `max_size`。但如果上传日志已经启用，返回值将依然是 `true`
/// @retval bool 如果上传日志已经启用，最大尺寸必然存在，则返回 `true`，否则返回 `false`
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

/// @brief 获取客户端配置中的上传进度记录仪文件根目录
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t 文件根目录
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
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

/// @brief 获取客户端配置中的文件分块有效期
/// @details 对于超过有效期的分块，SDK 将重新上传，确保所有分块在创建文件时均有效
/// @param[in] config 客户端配置实例
/// @retval uint64_t 返回文件分块有效期，单位为秒
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_recorder_upload_block_lifetime(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.upload_recorder().upload_block_lifetime().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置中的进度记录文件始终刷新
/// @details 当记录上传进度后，是否始终刷新 IO 确保数据已经被持久化
/// @param[in] config 客户端配置实例
/// @retval bool 进度记录文件是否始终刷新
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_upload_recorder_always_flush_records(config: qiniu_ng_config_t) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config.upload_recorder().always_flush_records().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置中的域名管理器的 URL 冻结时长
/// @details
///     当 SDK 发送 HTTP 请求时，如果发现网络或服务异常，靠重试无法解决的，则冻结所访问的服务器 URL。
///     被冻结的服务器在冻结期间将无法被访问
/// @param[in] config 客户端配置实例
/// @retval uint64_t 返回域名管理器的 URL 冻结时长，单位为秒
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_url_frozen_duration(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.domains_manager().url_frozen_duration().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置中的域名管理器的域名解析缓存生命周期
/// @param[in] config 客户端配置实例
/// @retval uint64_t 返回域名管理器的域名解析缓存生命周期，单位为秒
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

/// @brief 获取客户端配置中的域名管理器是否禁用 URL 域名预解析
/// @param[in] config 客户端配置实例
/// @retval uint64_t 域名管理器是否禁用 URL 域名预解析
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_url_resolution_disabled(config: qiniu_ng_config_t) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config.domains_manager().url_resolution_disabled().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置中的域名管理器的自动持久化间隔时间
/// @param[in] config 客户端配置实例
/// @retval uint64_t 返回域名管理器的自动持久化间隔时间，单位为秒
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

/// @brief 获取客户端配置中的域名管理器是否禁用自动持久化
/// @param[in] config 客户端配置实例
/// @retval uint64_t 域名管理器是否禁用自动持久化
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_auto_persistent_disabled(config: qiniu_ng_config_t) -> bool {
    let config = Option::<Config>::from(config).unwrap();
    config.domains_manager().auto_persistent_disabled().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置中的域名管理器的 URL 域名预解析重试次数
/// @details 当 SDK 预解析域名时发送错误时，SDK 将重试的次数
/// @param[in] config 客户端配置实例
/// @retval uint64_t 返回域名管理器的 URL 域名预解析重试次数
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_url_resolve_retries(config: qiniu_ng_config_t) -> size_t {
    let config = Option::<Config>::from(config).unwrap();
    config.domains_manager().url_resolve_retries().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置中的域名管理器的 URL 域名预解析重试前等待时间
/// @details
///     当 SDK 预解析域名时发送错误时，SDK 将等待一段时间并且重试。
///     每次实际等待时长为该项值的 50% - 100% 之间的随机时长。
/// @param[in] config 客户端配置实例
/// @retval uint64_t 返回域名管理器的 URL 域名预解析重试前等待时间
#[no_mangle]
pub extern "C" fn qiniu_ng_config_get_domains_manager_url_resolve_retry_delay(config: qiniu_ng_config_t) -> u64 {
    let config = Option::<Config>::from(config).unwrap();
    config.domains_manager().url_resolve_retry_delay().as_secs().tap(|_| {
        let _ = qiniu_ng_config_t::from(config);
    })
}

/// @brief 获取客户端配置中的域名管理器的持久化路径
/// @param[in] config 客户端配置实例
/// @retval qiniu_ng_str_t 持久化路径，如果持久化已经被禁用，则返回的字符串实例中将封装 `NULL`
/// @note 这里返回的 `qiniu_ng_str_t` 有可能封装的是 `NULL`，请调用 `qiniu_ng_str_is_null()` 进行判断
/// @warning 当 `qiniu_ng_str_t` 使用完毕后，请务必调用 `qiniu_ng_str_free()` 方法释放内存
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

/// @brief 释放客户端配置实例
/// @param[in,out] config 客户端配置实例地址，释放完毕后该实例将不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_config_free(config: *mut qiniu_ng_config_t) {
    if let Some(config) = unsafe { config.as_mut() } {
        let _ = Option::<Config>::from(*config);
        *config = qiniu_ng_config_t::new_freed();
    }
}

/// @brief 判断客户端配置实例是否已经被释放
/// @param[in] config 客户端配置实例
/// @retval bool 如果返回 `true` 则表示客户端配置实例已经被释放，该实例不再可用
#[no_mangle]
pub extern "C" fn qiniu_ng_config_is_freed(config: qiniu_ng_config_t) -> bool {
    config.is_null()
}

/// @brief 上传日志文件的锁策略
/// @details 为了防止上传文件的过程中，上传日志文件被多个进程同时修改引发竞争，因此需要在操作日志文件时使用文件锁保护
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum qiniu_ng_upload_logger_lock_policy_t {
    /// @brief 追加日志时为日志文件加共享锁，而上传时使用排他锁，相较其他策略可以实现安全和性能之间的平衡，因此是默认策略
    qiniu_ng_lock_policy_lock_shared_duration_appending_and_lock_exclusive_duration_uploading,
    /// @brief 始终使用排他锁保护文件，性能较差
    qiniu_ng_lock_policy_always_lock_exclusive,
    /// @brief 不使用任何锁保护文件，安全性差。
    /// @details
    ///     建议仅在能确保当前操作系统内不会有多个进程同时上传文件时，
    ///     或不同进程不会使用相同路径的上传日志时才使用这种策略
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
