use super::super::{region::Region, upload_token::UploadToken};
use crate::{
    config::Config,
    http::{Client, Response},
};
use derive_builder::Builder;
use qiniu_http::{Error as HTTPError, ErrorKind as HTTPErrorKind, HTTPCallerErrorKind, Method, Result as HTTPResult};
use std::{
    borrow::Cow,
    error::Error as StdError,
    fmt,
    fs::File,
    io::{Read, Result, Seek, SeekFrom, Write},
    net::IpAddr,
    sync::{Arc, RwLock},
    thread::spawn,
    time::{Duration, SystemTime},
};
use tempfile::{Builder as TempfileBuilder, NamedTempFile};
use url::Url;

struct UploadLoggerInner {
    server_url: &'static str,
    config: Config,
    http_client: Client,
    log_file: RwLock<NamedTempFile>,
    upload_token: Box<str>,
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
        spawn(move || {
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
                upload_token: self.upload_token.expect("upload_token must be set").into(),
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
    sent: usize,
    error_message: Cow<'a, str>,
    total_size: usize,
    timestamp: u64,
}

impl<'a> UploadLoggerRecordBuilder<'a> {
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
                    HTTPCallerErrorKind::ResolveError => self
                        .status_code(UNKNOWN_HOST)
                        .error_message(Cow::Borrowed(err.description())),
                    HTTPCallerErrorKind::ProxyError => self
                        .status_code(PROXY_ERROR)
                        .error_message(Cow::Borrowed(err.description())),
                    HTTPCallerErrorKind::SSLError => self
                        .status_code(SSL_ERROR)
                        .error_message(Cow::Borrowed(err.description())),
                    HTTPCallerErrorKind::ConnectionError => self
                        .status_code(CANNOT_CONNECT_TO_HOST)
                        .error_message(Cow::Borrowed(err.description())),
                    HTTPCallerErrorKind::RequestError => self
                        .status_code(NETWORK_CONNECTION_LOST)
                        .error_message(Cow::Borrowed(err.description())),
                    HTTPCallerErrorKind::ResponseError => self
                        .status_code(NETWORK_CONNECTION_LOST)
                        .error_message(Cow::Borrowed(err.description())),
                    HTTPCallerErrorKind::TimeoutError => self
                        .status_code(TIMED_OUT)
                        .error_message(Cow::Borrowed(err.description())),
                    HTTPCallerErrorKind::UnknownError => self
                        .status_code(NETWORK_ERROR)
                        .error_message(Cow::Borrowed(err.description())),
                }
            }
            HTTPErrorKind::JSONError(err) => self.error_message(Cow::Borrowed(err.description())),
            HTTPErrorKind::MaliciousResponse => self.error_message(Cow::Borrowed(err.description())),
            HTTPErrorKind::UnexpectedRedirect => self.error_message(Cow::Borrowed(err.description())),
            HTTPErrorKind::IOError(err) => self.error_message(Cow::Borrowed(err.description())),
            HTTPErrorKind::UnknownError(err) => self.error_message(Cow::Borrowed(err.description())),
            HTTPErrorKind::ResponseStatusCodeError(status_code, error_message) => {
                self.status_code(*status_code).error_message(error_message.to_string())
            }
        }
    }
}

impl Default for UploadLoggerRecord<'_> {
    fn default() -> Self {
        UploadLoggerRecord {
            status_code: None,
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

#[cfg(test)]
mod tests {
    use super::{super::super::upload_policy::UploadPolicyBuilder, *};
    use crate::{config::ConfigBuilder, credential::Credential, http::DomainsManagerBuilder};
    use qiniu_http::Headers;
    use qiniu_test_utils::http_call_mock::{CounterCallMock, JSONCallMock};
    use serde_json::json;
    use std::{boxed::Box, error::Error, net::Ipv4Addr, result::Result, thread::sleep};

    #[test]
    fn test_storage_uploader_upload_logger_upload_and_clean() -> Result<(), Box<dyn Error>> {
        let mock = CounterCallMock::new(JSONCallMock::new(200, Headers::new(), json!({})));
        let config = ConfigBuilder::default()
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .uplog_upload_threshold(100)
            .build()?;
        let upload_logger = UploadLoggerBuilder::default()
            .upload_token(&UploadToken::from_policy(
                UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build(),
                get_credential(),
            ))
            .build_by(config)
            .unwrap()?;
        upload_logger.log(
            UploadLoggerRecordBuilder::default()
                .status_code(200)
                .request_id("dPgAAABCOSlIU84V")
                .host("upload.qiniup.com")
                .up_type(UpType::Form)
                .server_ip(IpAddr::V4(Ipv4Addr::new(115, 238, 101, 49)))
                .server_port(80u16)
                .duration(Duration::from_millis(123))
                .sent(123123usize)
                .total_size(123123usize)
                .build()
                .unwrap(),
        );
        sleep(Duration::from_millis(500));
        assert_eq!(mock.call_called(), 0);
        assert!(upload_logger.inner.log_file.read().unwrap().as_file().metadata()?.len() > 0);
        upload_logger.log(
            UploadLoggerRecordBuilder::default()
                .status_code(200)
                .request_id("LC8AAAAlCUJIU84V")
                .host("upload.qiniup.com")
                .up_type(UpType::Form)
                .server_ip(IpAddr::V4(Ipv4Addr::new(115, 238, 101, 49)))
                .server_port(80u16)
                .duration(Duration::from_millis(456))
                .sent(456usize)
                .total_size(456usize)
                .build()
                .unwrap(),
        );
        sleep(Duration::from_millis(500));
        assert_eq!(mock.call_called(), 1);
        assert_eq!(
            upload_logger.inner.log_file.read().unwrap().as_file().metadata()?.len(),
            0
        );
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_logger_max_uplog_file_size() -> Result<(), Box<dyn Error>> {
        let mock = CounterCallMock::new(JSONCallMock::new(200, Headers::new(), json!({})));
        let config = ConfigBuilder::default()
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .uplog_upload_threshold(100)
            .max_uplog_file_size(50)
            .build()?;
        let upload_logger = UploadLoggerBuilder::default()
            .upload_token(&UploadToken::from_policy(
                UploadPolicyBuilder::new_policy_for_bucket("test_bucket", &config).build(),
                get_credential(),
            ))
            .build_by(config)
            .unwrap()?;
        upload_logger.log(
            UploadLoggerRecordBuilder::default()
                .status_code(200)
                .request_id("dPgAAABCOSlIU84V")
                .host("upload.qiniup.com")
                .up_type(UpType::Form)
                .server_ip(IpAddr::V4(Ipv4Addr::new(115, 238, 101, 49)))
                .server_port(80u16)
                .duration(Duration::from_millis(123))
                .sent(123123usize)
                .total_size(123123usize)
                .build()
                .unwrap(),
        );
        sleep(Duration::from_millis(500));
        assert_eq!(mock.call_called(), 0);
        assert!(upload_logger.inner.log_file.read().unwrap().as_file().metadata()?.len() > 0);
        upload_logger.log(
            UploadLoggerRecordBuilder::default()
                .status_code(200)
                .request_id("LC8AAAAlCUJIU84V")
                .host("upload.qiniup.com")
                .up_type(UpType::Form)
                .server_ip(IpAddr::V4(Ipv4Addr::new(115, 238, 101, 49)))
                .server_port(80u16)
                .duration(Duration::from_millis(456))
                .sent(456usize)
                .total_size(456usize)
                .build()
                .unwrap(),
        );
        sleep(Duration::from_millis(500));
        assert_eq!(mock.call_called(), 0);
        assert!(upload_logger.inner.log_file.read().unwrap().as_file().metadata()?.len() > 0);
        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
