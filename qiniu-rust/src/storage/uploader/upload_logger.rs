use crate::{
    http::{
        Client, Error as HTTPError, ErrorKind as HTTPErrorKind, HTTPCallerErrorKind, Response, Result as HTTPResult,
    },
    utils::global_thread_pool,
};
use assert_impl::assert_impl;
use derive_builder::Builder;
use dirs::cache_dir;
use fs2::FileExt;
use std::{
    borrow::Cow,
    convert::TryInto,
    env::temp_dir,
    error::Error,
    fmt,
    fs::{create_dir_all, File, OpenOptions},
    io::{Error as IOError, Read, Result as IOResult, Seek, SeekFrom, Write},
    net::IpAddr,
    path::Path,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};
use tap::TapOps;
use thiserror::Error;
use url::Url;

/// 上传日志文件的锁策略
///
/// 为了防止上传文件的过程中，上传日志文件被多个进程同时修改引发竞争，因此需要在操作日志文件时使用文件锁保护
#[derive(Debug, Clone, Copy)]
pub enum LockPolicy {
    /// 追加日志时为日志文件加共享锁，而上传时使用排他锁，相较其他策略可以实现安全和性能之间的平衡，因此是默认策略
    LockSharedDuringAppendingAndLockExclusiveDuringUploading,
    /// 始终使用排他锁保护文件，性能较差
    AlwaysLockExclusive,
    /// 不使用任何锁保护文件，安全性差。
    /// 建议仅在能确保当前操作系统内不会有多个进程同时上传文件时，
    /// 或不同进程不会使用相同路径的上传日志时才使用这种策略
    None,
}

impl LockPolicy {
    #[allow(dead_code)]
    fn lock_for_appending(self, file: &File) -> IOResult<()> {
        match self {
            LockPolicy::LockSharedDuringAppendingAndLockExclusiveDuringUploading => file.lock_shared(),
            LockPolicy::AlwaysLockExclusive => file.lock_exclusive(),
            LockPolicy::None => Ok(()),
        }
    }
    #[allow(dead_code)]
    fn lock_for_uploading(self, file: &File) -> IOResult<()> {
        match self {
            LockPolicy::LockSharedDuringAppendingAndLockExclusiveDuringUploading | LockPolicy::AlwaysLockExclusive => {
                file.lock_exclusive()
            }
            LockPolicy::None => Ok(()),
        }
    }
    #[allow(dead_code)]
    fn try_lock_for_appending(self, file: &File) -> IOResult<()> {
        match self {
            LockPolicy::LockSharedDuringAppendingAndLockExclusiveDuringUploading => file.try_lock_shared(),
            LockPolicy::AlwaysLockExclusive => file.try_lock_exclusive(),
            LockPolicy::None => Ok(()),
        }
    }
    #[allow(dead_code)]
    fn try_lock_for_uploading(self, file: &File) -> IOResult<()> {
        match self {
            LockPolicy::LockSharedDuringAppendingAndLockExclusiveDuringUploading | LockPolicy::AlwaysLockExclusive => {
                file.try_lock_exclusive()
            }
            LockPolicy::None => Ok(()),
        }
    }
    fn unlock(self, file: &File) -> IOResult<()> {
        match self {
            LockPolicy::LockSharedDuringAppendingAndLockExclusiveDuringUploading | LockPolicy::AlwaysLockExclusive => {
                file.unlock()
            }
            LockPolicy::None => Ok(()),
        }
    }
}

/// 上传日志记录仪生成器
///
/// 用于配置并生成上传日志记录仪
#[derive(Builder)]
#[builder(
    name = "UploadLoggerBuilder",
    pattern = "owned",
    public,
    build_fn(name = "inner_build", private)
)]
struct UploadLoggerValue {
    /// 日志文件路径
    ///
    /// 默认的日志文件路径规则如下：
    ///   1. 尝试在[操作系统特定的缓存目录](https://docs.rs/dirs/2.0.2/dirs/fn.cache_dir.html)下创建 `qiniu_sdk` 目录。
    ///   2. 如果成功，则使用 `qiniu_sdk` 目录下的 `upload.log`。
    ///   3. 如果失败，则直接使用临时目录下的 `upload.log`。
    #[builder(default = "default::log_file_path()")]
    log_file_path: Cow<'static, Path>,

    /// 日志文件的锁策略
    ///
    /// 为了防止上传文件的过程中，上传日志文件被多个进程同时修改引发竞争，因此需要在操作日志文件时使用文件锁保护。
    /// 默认策略为在追加日志时为日志文件加共享锁，而上传时使用排他锁，尽可能做到安全和性能之间的平衡。
    ///
    /// 但在有些场景下中，并发追加日志文件同样会引发竞争，此时需要改用 `AlwaysLockExclusive` 策略。
    /// 此外，如果确定当前操作系统内不会有多个进程同时上传文件，或不同进程不会使用相同路径的日志时，
    /// 也可以使用 `None` 策略，减少文件锁的性能影响。
    #[builder(default = "default::lock_policy()")]
    lock_policy: LockPolicy,

    /// 日志文件的阙值
    ///
    /// 当且仅当日志文件尺寸大于阙值时才会上传日志。
    /// 单位为字节
    #[builder(default = "default::upload_threshold()")]
    upload_threshold: u32,

    /// 日志文件最大尺寸
    ///
    /// 当日志文件尺寸大于指定尺寸时，将不会再记录任何数据到日志内。
    /// 防止在上传发生困难时日志文件无限制膨胀。
    /// 单位为字节。该值必须大于 `upload_threshold`
    #[builder(default = "default::max_size()")]
    max_size: u32,
}

struct UploadLoggerInner {
    log_buffer: RwLock<Vec<u8>>,
    log_file: RwLock<File>,
    value: UploadLoggerValue,
}

/// 上传日志记录仪
///
/// 收集文件上传相关日志信息，并自动以异步的形式上传到 Uplog 服务器，并由七牛工作人员进行统计或定位问题
#[derive(Clone)]
pub struct UploadLogger {
    inner: Arc<UploadLoggerInner>,
}

#[derive(Clone)]
pub(crate) struct TokenizedUploadLogger {
    upload_logger: UploadLogger,
    http_client: Client,
    upload_token: Box<str>,
    dropped: bool,
}

impl UploadLogger {
    pub(crate) fn tokenize(&self, upload_token: Box<str>, http_client: Client) -> TokenizedUploadLogger {
        TokenizedUploadLogger {
            upload_logger: self.clone(),
            http_client,
            upload_token,
            dropped: false,
        }
    }

    /// 获取日志文件路径
    pub fn log_file_path(&self) -> &Path {
        self.inner.value.log_file_path.as_ref()
    }

    /// 获取日志文件锁策略
    pub fn lock_policy(&self) -> LockPolicy {
        self.inner.value.lock_policy
    }

    /// 日志文件的阙值
    pub fn upload_threshold(&self) -> u32 {
        self.inner.value.upload_threshold
    }

    /// 日志文件的最大尺寸
    pub fn max_size(&self) -> u32 {
        self.inner.value.max_size
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl UploadLoggerBuilder {
    /// 生成上传日志记录仪
    pub fn build(self) -> IOResult<UploadLogger> {
        let value = self.inner_build().unwrap();
        let log_file = RwLock::new(
            OpenOptions::new()
                .read(true)
                .write(true)
                .append(true)
                .create(true)
                .open(value.log_file_path.as_ref())?,
        );
        let log_buffer = RwLock::new(Vec::with_capacity(value.max_size as usize));
        Ok(UploadLogger {
            inner: Arc::new(UploadLoggerInner {
                log_file,
                log_buffer,
                value,
            }),
        })
    }
}

mod default {
    use super::*;

    pub fn log_file_path() -> Cow<'static, Path> {
        let mut default_path = cache_dir().unwrap_or_else(temp_dir);
        default_path.push("qiniu_sdk");
        default_path = create_dir_all(&default_path)
            .map(|_| default_path)
            .unwrap_or_else(|_| temp_dir());
        default_path.push("upload.log");
        default_path.into()
    }

    #[inline]
    pub const fn upload_threshold() -> u32 {
        1 << 12
    }

    #[inline]
    pub const fn max_size() -> u32 {
        1 << 22
    }

    #[inline]
    pub const fn lock_policy() -> LockPolicy {
        LockPolicy::LockSharedDuringAppendingAndLockExclusiveDuringUploading
    }
}

impl fmt::Debug for UploadLogger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("UploadLogger")
            .field("upload_threshold", &self.inner.value.upload_threshold)
            .field("max_size", &self.inner.value.max_size)
            .finish()
    }
}

impl TokenizedUploadLogger {
    pub(crate) fn log(&self, record: UploadLoggerRecord) -> IOResult<()> {
        self.append_record_to_log_buffer(record);
        self.append_log_buffer_to_record_or_upload_log_buffer(false)?;
        self.async_lock_log_file_and_update_then_clean_if_needed()?;
        Ok(())
    }

    fn append_record_to_log_buffer(&self, record: UploadLoggerRecord) {
        let log_buffer_size: u32 = self
            .upload_logger
            .inner
            .log_buffer
            .read()
            .unwrap()
            .len()
            .try_into()
            .unwrap_or(u32::max_value());
        if log_buffer_size < self.upload_logger.inner.value.max_size {
            let record = record.to_string() + "\n";
            let record_bytes = record.as_bytes();
            let record_size: u32 = record_bytes.len().try_into().unwrap_or(u32::max_value());
            if log_buffer_size + record_size < self.upload_logger.inner.value.max_size {
                self.upload_logger
                    .inner
                    .log_buffer
                    .write()
                    .unwrap()
                    .extend_from_slice(record_bytes);
            }
        }
    }

    fn append_log_buffer_to_record_or_upload_log_buffer(&self, ignore_upload_threshold: bool) -> IOResult<()> {
        let log_file_size: u32 = self
            .upload_logger
            .inner
            .log_file
            .read()
            .unwrap()
            .metadata()?
            .len()
            .try_into()
            .unwrap_or(u32::max_value());
        if log_file_size < self.upload_logger.inner.value.max_size {
            let mut log_buffer_size: u32 = self
                .upload_logger
                .inner
                .log_buffer
                .read()
                .unwrap()
                .len()
                .try_into()
                .unwrap_or(u32::max_value());
            if log_buffer_size > 0 {
                if log_file_size + log_buffer_size < self.upload_logger.inner.value.max_size {
                    if let Ok(mut log_file) = self.upload_logger.inner.log_file.try_write() {
                        if self
                            .upload_logger
                            .inner
                            .value
                            .lock_policy
                            .try_lock_for_appending(&log_file)
                            .is_ok()
                        {
                            let log_buffer_content = {
                                let mut log_buffer = self.upload_logger.inner.log_buffer.write().unwrap();
                                log_buffer.clone().tap(|_| log_buffer.clear())
                            };
                            log_file.write_all(&log_buffer_content).tap(|_| {
                                let _ = self.upload_logger.inner.value.lock_policy.unlock(&log_file);
                            })?;
                            return Ok(());
                        }
                    }
                }
                if ignore_upload_threshold || log_buffer_size > self.upload_logger.inner.value.upload_threshold {
                    let log_buffer_content = {
                        let mut log_buffer = self.upload_logger.inner.log_buffer.write().unwrap();
                        log_buffer.clone().tap(|_| log_buffer.clear())
                    };
                    log_buffer_size = log_buffer_content.len().try_into().unwrap_or(u32::max_value());
                    if ignore_upload_threshold && !log_buffer_content.is_empty()
                        || log_buffer_size > self.upload_logger.inner.value.upload_threshold
                    {
                        self.async_upload_log_buffer(log_buffer_content);
                    }
                }
            }
        }
        Ok(())
    }

    fn async_lock_log_file_and_update_then_clean_if_needed(&self) -> IOResult<()> {
        let log_file_size: u32 = self
            .upload_logger
            .inner
            .log_file
            .read()
            .unwrap()
            .metadata()?
            .len()
            .try_into()
            .unwrap_or(u32::max_value());
        if log_file_size > self.upload_logger.inner.value.upload_threshold {
            self.async_lock_log_file_and_update_then_clean();
        }
        Ok(())
    }

    fn async_lock_log_file_and_update_then_clean(&self) {
        let upload_logger = self.clone();
        global_thread_pool.read().unwrap().spawn(move || {
            let _ = upload_logger.lock_log_file_and_update_then_clean();
        });
    }

    fn lock_log_file_and_update_then_clean(&self) -> UploadResult<()> {
        let log_file_size: u32 = self
            .upload_logger
            .inner
            .log_file
            .read()
            .unwrap()
            .metadata()?
            .len()
            .try_into()
            .unwrap_or(u32::max_value());
        if log_file_size > self.upload_logger.inner.value.upload_threshold {
            let mut log_file = self.upload_logger.inner.log_file.write().unwrap();

            self.upload_logger
                .inner
                .value
                .lock_policy
                .lock_for_uploading(&log_file)?;
            self.upload_log_file_and_clean(&mut log_file).tap(|_| {
                let _ = self.upload_logger.inner.value.lock_policy.unlock(&log_file);
            })?;
        }
        Ok(())
    }

    fn upload_log_file_and_clean(&self, log_file: &mut File) -> UploadResult<()> {
        let log_file_size: u32 = log_file.metadata()?.len().try_into().unwrap_or(u32::max_value());
        if log_file_size > self.upload_logger.inner.value.upload_threshold {
            let mut log_buffer = Vec::with_capacity(log_file_size as usize);
            log_file.seek(SeekFrom::Start(0))?;
            log_file.read_to_end(&mut log_buffer)?;
            self.upload_log_buffer(&log_buffer)?;
            log_file.set_len(0)?;
            log_file.seek(SeekFrom::Start(0))?;
        }
        Ok(())
    }

    fn async_upload_log_buffer(&self, log_buffer: Vec<u8>) {
        let upload_logger = self.clone();
        global_thread_pool.read().unwrap().spawn(move || {
            let _ = upload_logger.upload_log_buffer(&log_buffer);
        });
    }

    fn upload_log_buffer(&self, log_buffer: &[u8]) -> HTTPResult<()> {
        if !log_buffer.is_empty() {
            self.http_client
                .post("/log/3", &[self.http_client.config().uplog_url().as_ref()])
                .header("Authorization", "UpToken ".to_owned() + &self.upload_token)
                .raw_body("text/plain", log_buffer)
                .send()?
                .ignore_body();
        }
        Ok(())
    }

    #[cfg(test)]
    fn clear(&self) -> IOResult<()> {
        let mut log_file = self.upload_logger.inner.log_file.write().unwrap();
        log_file.set_len(0)?;
        log_file.seek(SeekFrom::Start(0))?;
        drop(log_file);

        let mut log_buffer = self.upload_logger.inner.log_buffer.write().unwrap();
        log_buffer.clear();
        drop(log_buffer);

        Ok(())
    }

    #[cfg(test)]
    fn size(&self) -> IOResult<u32> {
        let mut size = 0u32;

        let log_file = self.upload_logger.inner.log_file.read().unwrap();
        size += log_file.metadata()?.len().try_into().unwrap_or(u32::max_value());
        drop(log_file);

        let log_buffer = self.upload_logger.inner.log_buffer.read().unwrap();
        size += log_buffer.len().try_into().unwrap_or(u32::max_value());
        drop(log_buffer);

        Ok(size)
    }

    #[cfg(test)]
    fn lock_log_file(&self) -> IOResult<()> {
        self.upload_logger.inner.log_file.read().unwrap().lock_exclusive()
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl Drop for TokenizedUploadLogger {
    fn drop(&mut self) {
        if !self.dropped {
            self.dropped = true;
            let _ = self.append_log_buffer_to_record_or_upload_log_buffer(true);
        }
    }
}

#[derive(Error, Debug)]
enum UploadError {
    #[error("Qiniu API call error: {0}")]
    QiniuAPIError(#[from] HTTPError),
    #[error("Failed to do io operation for log file: {0}")]
    IOError(#[from] IOError),
}

type UploadResult<T> = Result<T, UploadError>;

#[derive(Copy, Clone, Debug)]
pub(crate) enum UpType {
    Form,
    Chunkedv2,
    InitParts,
    UploadPart,
    CompleteParts,
}

impl UpType {
    fn as_str(self) -> &'static str {
        match self {
            UpType::Form => "form",
            UpType::Chunkedv2 => "chunked_v2",
            UpType::InitParts => "init_parts",
            UpType::UploadPart => "upload_part",
            UpType::CompleteParts => "complete_parts",
        }
    }
}

#[derive(Builder, Debug)]
#[builder(
    pattern = "owned",
    default,
    setter(into, strip_option),
    build_fn(name = "inner_build", private)
)]
pub(crate) struct UploadLoggerRecord<'a> {
    status_code: Option<i32>,
    request_id: Cow<'a, str>,
    host: Cow<'a, str>,
    up_type: Option<UpType>,
    server_ip: Option<IpAddr>,
    server_port: u16,
    duration: Option<Duration>,
    sent: u64,
    error_message: Cow<'a, str>,
    total_size: u64,
    timestamp: u64,
}

impl<'a> UploadLoggerRecordBuilder<'a> {
    pub(crate) fn build(self) -> UploadLoggerRecord<'a> {
        self.inner_build().unwrap()
    }

    pub(crate) fn response(self, response: &'a Response) -> UploadLoggerRecordBuilder<'a> {
        let mut builder = self
            .status_code(response.status_code())
            .host(Url::parse(response.base_url()).unwrap().host_str().unwrap().to_owned())
            .server_port(response.server_port());
        if let Some(request_id) = response.request_id() {
            builder = builder.request_id(request_id);
        }
        if let Some(server_ip) = response.server_ip() {
            builder = builder.server_ip(server_ip);
        }
        builder
    }

    pub(crate) fn http_error(self, err: &'a HTTPError) -> UploadLoggerRecordBuilder<'a> {
        match err.error_kind() {
            HTTPErrorKind::HTTPCallerError(err) => {
                const NETWORK_ERROR: i32 = -1;
                const TIMED_OUT: i32 = -1001;
                const UNKNOWN_HOST: i32 = -1003;
                const CANNOT_CONNECT_TO_HOST: i32 = -1004;
                const NETWORK_CONNECTION_LOST: i32 = -1005;
                const PROXY_ERROR: i32 = -1006;
                const SSL_ERROR: i32 = -1007;
                match err.kind() {
                    HTTPCallerErrorKind::ResolveError => {
                        self.status_code(UNKNOWN_HOST).error_message(err.description())
                    }
                    HTTPCallerErrorKind::ProxyError => self.status_code(PROXY_ERROR).error_message(err.description()),
                    HTTPCallerErrorKind::SSLError => self.status_code(SSL_ERROR).error_message(err.description()),
                    HTTPCallerErrorKind::ConnectionError => self
                        .status_code(CANNOT_CONNECT_TO_HOST)
                        .error_message(err.description()),
                    HTTPCallerErrorKind::RequestError => self
                        .status_code(NETWORK_CONNECTION_LOST)
                        .error_message(err.description()),
                    HTTPCallerErrorKind::ResponseError => self
                        .status_code(NETWORK_CONNECTION_LOST)
                        .error_message(err.description()),
                    HTTPCallerErrorKind::TimeoutError => self.status_code(TIMED_OUT).error_message(err.description()),
                    HTTPCallerErrorKind::UnknownError => {
                        self.status_code(NETWORK_ERROR).error_message(err.description())
                    }
                }
            }
            HTTPErrorKind::JSONError(err) => self.error_message(err.description()),
            HTTPErrorKind::MaliciousResponse => self.error_message(err.description()),
            HTTPErrorKind::UnexpectedRedirect => self.error_message(err.description()),
            HTTPErrorKind::UserCanceled => self.error_message(err.description()),
            HTTPErrorKind::IOError(err) => self.error_message(err.description()),
            HTTPErrorKind::UnknownError(err) => self.error_message(err.description()),
            HTTPErrorKind::ResponseStatusCodeError(status_code, error_message) => {
                self.status_code(*status_code).error_message(error_message.as_ref())
            }
        }
    }
}

impl Default for UploadLoggerRecord<'_> {
    fn default() -> Self {
        UploadLoggerRecord {
            status_code: None,
            request_id: "".into(),
            host: "".into(),
            up_type: None,
            server_ip: None,
            server_port: 0,
            duration: None,
            sent: 0,
            error_message: "".into(),
            total_size: 0,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

impl fmt::Display for UploadLoggerRecord<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{},{},{},{},{},{},{},{},{},{}",
            self.status_code
                .map(|code| code.to_string())
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("null"),
            self.request_id,
            self.host,
            self.server_ip
                .map(|ip| ip.to_string())
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or(""),
            self.server_port,
            self.duration
                .map(|duration| duration.as_millis().to_string())
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("-1"),
            self.timestamp,
            self.sent,
            self.up_type.map(|t| t.as_str()).unwrap_or(""),
            self.total_size,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::super::uploader::{UploadPolicyBuilder, UploadToken},
        *,
    };
    use crate::{
        config::ConfigBuilder,
        credential::Credential,
        http::{DomainsManagerBuilder, Headers},
    };
    use qiniu_test_utils::http_call_mock::{CounterCallMock, JSONCallMock};
    use serde_json::json;
    use std::{boxed::Box, error::Error, mem::drop, net::Ipv4Addr, result::Result, thread::sleep};

    #[test]
    fn test_storage_uploader_upload_logger_upload_and_clean() -> Result<(), Box<dyn Error>> {
        let mock = CounterCallMock::new(JSONCallMock::new(200, Headers::new(), json!({})));
        let config = ConfigBuilder::default()
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .upload_logger(Some(UploadLoggerBuilder::default().upload_threshold(100).build()?))
            .build();
        let upload_logger = config.upload_logger().as_ref().unwrap().tokenize(
            UploadToken::new(
                UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build(),
                get_credential(),
            )
            .to_string()
            .into(),
            Client::new(config.to_owned()),
        );
        upload_logger.clear()?;
        upload_logger.log(
            UploadLoggerRecordBuilder::default()
                .status_code(200)
                .request_id("dPgAAABCOSlIU84V")
                .host("upload.qiniup.com")
                .up_type(UpType::Form)
                .server_ip(IpAddr::V4(Ipv4Addr::new(115, 238, 101, 49)))
                .server_port(80u16)
                .duration(Duration::from_millis(123))
                .sent(123_123u64)
                .total_size(123_123u64)
                .build(),
        )?;
        sleep(Duration::from_secs(1));
        assert_eq!(mock.call_called(), 0);
        assert!(upload_logger.size()? > 0);
        upload_logger.log(
            UploadLoggerRecordBuilder::default()
                .status_code(200)
                .request_id("LC8AAAAlCUJIU84V")
                .host("upload.qiniup.com")
                .up_type(UpType::Form)
                .server_ip(IpAddr::V4(Ipv4Addr::new(115, 238, 101, 49)))
                .server_port(80u16)
                .duration(Duration::from_millis(456))
                .sent(456u64)
                .total_size(456u64)
                .build(),
        )?;
        sleep(Duration::from_secs(1));
        assert_eq!(mock.call_called(), 1);
        assert_eq!(upload_logger.size()?, 0);
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_logger_uplog_max_size() -> Result<(), Box<dyn Error>> {
        let mock = CounterCallMock::new(JSONCallMock::new(200, Headers::new(), json!({})));
        let config = ConfigBuilder::default()
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .upload_logger(Some(
                UploadLoggerBuilder::default()
                    .upload_threshold(100)
                    .max_size(100)
                    .build()?,
            ))
            .build();
        let upload_logger = config.upload_logger().as_ref().unwrap().tokenize(
            UploadToken::new(
                UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build(),
                get_credential(),
            )
            .to_string()
            .into(),
            Client::new(config.to_owned()),
        );
        upload_logger.clear()?;
        upload_logger.log(
            UploadLoggerRecordBuilder::default()
                .status_code(200)
                .request_id("dPgAAABCOSlIU84V")
                .host("upload.qiniup.com")
                .up_type(UpType::Form)
                .server_ip(IpAddr::V4(Ipv4Addr::new(115, 238, 101, 49)))
                .server_port(80u16)
                .duration(Duration::from_millis(123))
                .sent(123_123u64)
                .total_size(123_123u64)
                .build(),
        )?;
        sleep(Duration::from_secs(1));
        assert_eq!(mock.call_called(), 0);
        assert!(upload_logger.size()? > 0);
        upload_logger.log(
            UploadLoggerRecordBuilder::default()
                .status_code(200)
                .request_id("LC8AAAAlCUJIU84V")
                .host("upload.qiniup.com")
                .up_type(UpType::Form)
                .server_ip(IpAddr::V4(Ipv4Addr::new(115, 238, 101, 49)))
                .server_port(80u16)
                .duration(Duration::from_millis(456))
                .sent(456u64)
                .total_size(456u64)
                .build(),
        )?;
        sleep(Duration::from_secs(1));
        assert_eq!(mock.call_called(), 0);
        assert!(upload_logger.size()? > 0);

        upload_logger.lock_log_file()?;
        drop(upload_logger);
        sleep(Duration::from_secs(1));
        assert_eq!(mock.call_called(), 1);

        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
