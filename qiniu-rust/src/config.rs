use crate::{
    http::DomainsManager,
    storage::{region::Region, uploader::UploadRecorder},
};
use assert_impl::assert_impl;
use derive_builder::Builder;
use getset::{CopyGetters, Getters};
use qiniu_http::HTTPCaller;
use std::{
    boxed::Box,
    default::Default,
    fmt,
    marker::{Send, Sync},
    ops::Deref,
    sync::Arc,
    time::Duration,
};

#[derive(Builder, Getters, CopyGetters)]
#[builder(
    name = "ConfigBuilder",
    pattern = "owned",
    public,
    build_fn(name = "inner_build", private)
)]
pub struct ConfigInner {
    #[get_copy = "pub"]
    #[builder(default = "default::use_https()")]
    use_https: bool,

    #[get_copy = "pub"]
    #[builder(default = "default::upload_token_lifetime()")]
    upload_token_lifetime: Duration,

    #[get_copy = "pub"]
    #[builder(default = "default::batch_max_operation_size()")]
    batch_max_operation_size: usize,

    #[get_copy = "pub"]
    #[builder(default = "default::upload_threshold()")]
    upload_threshold: u64,

    #[get_copy = "pub"]
    #[builder(default = "default::upload_block_size()")]
    upload_block_size: usize,

    #[get_copy = "pub"]
    #[builder(default = "default::uplog_disabled()")]
    uplog_disabled: bool,

    #[get_copy = "pub"]
    #[builder(setter(into))]
    #[builder(default = "default::uplog_server_url()")]
    uplog_server_url: &'static str,

    #[get_copy = "pub"]
    #[builder(default = "default::uplog_upload_threshold()")]
    uplog_upload_threshold: usize,

    #[get_copy = "pub"]
    #[builder(default = "default::uplog_max_size()")]
    uplog_max_size: usize,

    #[get = "pub"]
    #[builder(default = "default::recorder()")]
    upload_recorder: UploadRecorder,

    #[get_copy = "pub"]
    #[builder(default = "default::http_request_retries()")]
    http_request_retries: usize,

    #[get_copy = "pub"]
    #[builder(default = "default::http_request_retry_delay()")]
    http_request_retry_delay: Duration,

    #[get = "pub"]
    #[builder(default = "default::http_request_call()")]
    http_request_call: Box<dyn HTTPCaller + Send + Sync>,

    #[get = "pub"]
    #[builder(default = "default::domains_manager()")]
    domains_manager: DomainsManager,
}

pub mod default {
    use super::*;

    pub fn use_https() -> bool {
        false
    }

    pub fn upload_token_lifetime() -> Duration {
        Duration::from_secs(60 * 60)
    }

    pub fn batch_max_operation_size() -> usize {
        1000
    }

    pub fn upload_threshold() -> u64 {
        1 << 22
    }

    pub fn upload_block_size() -> usize {
        1 << 22
    }

    pub fn uplog_disabled() -> bool {
        false
    }

    pub fn uplog_server_url() -> &'static str {
        Region::uplog_url()
    }

    pub fn uplog_upload_threshold() -> usize {
        1 << 12
    }

    pub fn uplog_max_size() -> usize {
        1 << 22
    }

    pub fn recorder() -> UploadRecorder {
        Default::default()
    }

    pub fn http_request_retries() -> usize {
        3
    }

    pub fn http_request_retry_delay() -> Duration {
        Duration::from_secs(1)
    }

    pub fn domains_manager() -> DomainsManager {
        Default::default()
    }

    pub fn http_request_call() -> Box<dyn HTTPCaller + Sync + Send> {
        #[cfg(any(feature = "use-libcurl"))]
        {
            Box::new(qiniu_with_libcurl::CurlClient::default())
        }
        #[cfg(not(feature = "use-libcurl"))]
        {
            use super::super::http::PanickedHTTPCaller;
            Box::new(PanickedHTTPCaller("Must define config.http_request_call"))
        }
    }
}

impl fmt::Debug for ConfigInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Config")
            .field("use_https", &self.use_https)
            .field("upload_token_lifetime", &self.upload_token_lifetime)
            .field("batch_max_operation_size", &self.batch_max_operation_size)
            .field("upload_threshold", &self.upload_threshold)
            .field("upload_block_size", &self.upload_block_size)
            .field("uplog_server_url", &self.uplog_server_url)
            .field("upload_recorder", &self.upload_recorder)
            .field("uplog_disabled", &self.uplog_disabled)
            .field("uplog_upload_threshold", &self.uplog_upload_threshold)
            .field("uplog_max_size", &self.uplog_max_size)
            .field("http_request_retries", &self.http_request_retries)
            .field("http_request_retry_delay", &self.http_request_retry_delay)
            .field("domains_manager", &self.domains_manager)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct Config(Arc<ConfigInner>);

impl ConfigBuilder {
    pub fn build(self) -> Config {
        Config(Arc::new(self.inner_build().unwrap()))
    }
}

impl Default for Config {
    fn default() -> Self {
        ConfigBuilder::default().build()
    }
}

impl Deref for Config {
    type Target = ConfigInner;

    #[inline]
    fn deref(&self) -> &ConfigInner {
        self.0.deref()
    }
}

impl Config {
    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qiniu_http::{Request, RequestBuilder, Response, ResponseBuilder, Result};
    use std::{
        error::Error,
        io::{Cursor, Read},
        result::Result as StdResult,
    };

    struct FakeHTTPRequester;
    impl HTTPCaller for FakeHTTPRequester {
        fn call(&self, _: &Request) -> Result<Response> {
            Ok(ResponseBuilder::default()
                .status_code(612u16)
                .stream(Cursor::new(Vec::from("It's HTTP Body".as_bytes())))
                .build())
        }
    }

    #[test]
    fn test_config_with_set_http_request_call() -> StdResult<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .http_request_retries(5)
            .http_request_retry_delay(Duration::from_secs(1))
            .http_request_call(Box::new(FakeHTTPRequester))
            .build();

        let http_response = config
            .http_request_call()
            .call(&RequestBuilder::default().url("http://fake.qiniu.com").build())?;

        let mut http_body = String::new();
        assert_eq!(http_response.status_code(), 612);
        http_response.into_body().unwrap().read_to_string(&mut http_body)?;
        assert_eq!(http_body, "It's HTTP Body");
        Ok(())
    }

    #[test]
    fn test_config_with_getters() -> StdResult<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .http_request_retries(5)
            .http_request_retry_delay(Duration::from_secs(1))
            .http_request_call(Box::new(FakeHTTPRequester))
            .build();
        assert_eq!(config.http_request_retries(), 5);
        assert_eq!(config.http_request_retry_delay(), Duration::from_secs(1));
        Ok(())
    }
}
