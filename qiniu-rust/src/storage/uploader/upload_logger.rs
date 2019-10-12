use super::super::{region::Region, upload_token::UploadToken};
use crate::{config::Config, http::Client};
use derive_builder::Builder;
use qiniu_http::{Error as HTTPError, ErrorKind as HTTPErrorKind, Method, Result as HTTPResult};
use std::{
    borrow::Cow,
    fmt,
    fs::File,
    io::{Read, Result, Seek, SeekFrom, Write},
    net::IpAddr,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, SystemTime},
};
use tempfile::{Builder as TempfileBuilder, NamedTempFile};

struct UploadLoggerInner {
    server_url: &'static str,
    config: Config,
    http_client: Client,
    log_file: RwLock<NamedTempFile>,
    upload_token: String,
}

#[derive(Clone)]
pub(crate) struct UploadLogger {
    inner: Arc<UploadLoggerInner>,
}

impl UploadLogger {
    pub(crate) fn log(&self, record: UploadLoggerRecord) {
        if self.log_file_size() < self.inner.config.max_uplog_file_size() {
            writeln!(self.inner.log_file.write().unwrap().as_file_mut(), "{}", record).ok();
        }
        if self.log_file_size() >= self.inner.config.uplog_upload_threshold() {
            self.async_upload_log_file_and_clean();
        }
    }

    fn async_upload_log_file_and_clean(&self) {
        let upload_logger = self.clone();
        thread::spawn(move || {
            upload_logger.upload_log_file_and_clean().ok();
        });
    }

    fn upload_log_file_and_clean(&self) -> HTTPResult<()> {
        let mut log_file = self.inner.log_file.write().unwrap();
        self.upload_log_file(log_file.as_file_mut())?;
        log_file.as_file_mut().seek(SeekFrom::Start(0)).ok();
        log_file.as_file_mut().set_len(0).ok();
        Ok(())
    }

    fn upload_log_file(&self, log_file: &mut File) -> HTTPResult<()> {
        let request_body = self.read_log_file(log_file).map_err(|err| {
            HTTPError::new_host_unretryable_error_from_parts(
                HTTPErrorKind::IOError(err),
                true,
                Some(Method::POST),
                Some((self.inner.server_url.to_owned() + "/log/3").into()),
            )
        })?;

        if request_body.len() > 0 {
            self.inner
                .http_client
                .post("/log/3", &[self.inner.server_url])
                .header("Authorization", "UpToken ".to_owned() + &self.inner.upload_token)
                .raw_body("text/plain", request_body)
                .send()?
                .ignore_body();
        }
        Ok(())
    }

    fn read_log_file(&self, log_file: &mut File) -> Result<Vec<u8>> {
        log_file.seek(SeekFrom::Start(0)).ok();
        let mut upload_log_file_content = Vec::new();
        log_file.read_to_end(&mut upload_log_file_content)?;
        Ok(upload_log_file_content)
    }

    fn log_file_size(&self) -> u64 {
        self.inner
            .log_file
            .read()
            .unwrap()
            .as_file()
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0)
    }
}

impl Drop for UploadLogger {
    fn drop(&mut self) {
        if self.log_file_size() > 0 {
            self.upload_log_file_and_clean().ok();
        }
    }
}

pub(crate) struct UploadLoggerBuilder {
    server_url: &'static str,
    upload_token: Option<String>,
}

impl UploadLoggerBuilder {
    pub(crate) fn server_url(mut self, url: &'static str) -> UploadLoggerBuilder {
        self.server_url = url;
        self
    }

    pub(crate) fn upload_token(mut self, upload_token: &UploadToken) -> UploadLoggerBuilder {
        self.upload_token = Some(upload_token.token().into_owned());
        self
    }

    pub(crate) fn build_by(self, config: Config) -> Option<Result<UploadLogger>> {
        if config.uplog_disabled() {
            return None;
        }
        Some(self.build(config))
    }

    fn build(self, config: Config) -> Result<UploadLogger> {
        Ok(UploadLogger {
            inner: Arc::new(UploadLoggerInner {
                server_url: self.server_url,
                http_client: Client::new(config.clone()),
                config: config,
                upload_token: self.upload_token.expect("upload_token must be set"),
                log_file: RwLock::new(
                    TempfileBuilder::new()
                        .prefix("uplog")
                        .suffix(".log")
                        .append(true)
                        .tempfile()?,
                ),
            }),
        })
    }
}

impl Default for UploadLoggerBuilder {
    fn default() -> Self {
        UploadLoggerBuilder {
            server_url: Region::uplog_url(),
            upload_token: None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum UpType {
    Form,
    Chunkedv2,
    InitParts,
    UploadPart,
    CompleteParts,
}

impl UpType {
    fn as_str(&self) -> &'static str {
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
#[builder(pattern = "owned", default, setter(into, strip_option))]
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

impl Default for UploadLoggerRecord<'_> {
    fn default() -> Self {
        UploadLoggerRecord {
            status_code: Some(0),
            request_id: Cow::Borrowed(""),
            host: Cow::Borrowed(""),
            up_type: None,
            server_ip: None,
            server_port: 0,
            duration: None,
            sent: 0,
            error_message: Cow::Borrowed(""),
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
