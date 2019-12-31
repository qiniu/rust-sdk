use crate::{
    http::{DomainsManager, HTTPAfterActionHandler, HTTPBeforeActionHandler, HTTPHandler},
    storage::uploader::{UploadLogger, UploadLoggerBuilder, UploadRecorder},
};
use assert_impl::assert_impl;
use derive_builder::Builder;
use getset::{CopyGetters, Getters};
use std::{
    borrow::Cow,
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
    #[builder(default)]
    user_agent: Option<Cow<'static, str>>,

    #[get_copy = "pub"]
    #[builder(default = "default::use_https()")]
    use_https: bool,

    #[get = "pub"]
    #[builder(default = "default::uc_host()")]
    uc_host: Cow<'static, str>,

    #[get = "pub"]
    #[builder(default = "default::rs_host()")]
    rs_host: Cow<'static, str>,

    #[get = "pub"]
    #[builder(default = "default::rsf_host()")]
    rsf_host: Cow<'static, str>,

    #[get = "pub"]
    #[builder(default = "default::api_host()")]
    api_host: Cow<'static, str>,

    #[get = "pub"]
    #[builder(default = "default::uplog_url()")]
    uplog_url: Cow<'static, str>,

    #[get_copy = "pub"]
    #[builder(default = "default::upload_token_lifetime()")]
    upload_token_lifetime: Duration,

    #[get_copy = "pub"]
    #[builder(default = "default::batch_max_operation_size()")]
    batch_max_operation_size: usize,

    #[get_copy = "pub"]
    #[builder(default = "default::upload_threshold()")]
    upload_threshold: u32,

    #[get_copy = "pub"]
    #[builder(default = "default::upload_block_size()")]
    upload_block_size: u32,

    #[get = "pub"]
    #[builder(default = "default::upload_logger()")]
    upload_logger: Option<UploadLogger>,

    #[get = "pub"]
    #[builder(default)]
    upload_recorder: UploadRecorder,

    #[get_copy = "pub"]
    #[builder(default = "default::http_request_retries()")]
    http_request_retries: usize,

    #[get_copy = "pub"]
    #[builder(default = "default::http_request_retry_delay()")]
    http_request_retry_delay: Duration,

    #[get = "pub"]
    #[builder(default)]
    http_request_before_action_handlers: Vec<HTTPBeforeActionHandler>,

    #[get = "pub"]
    #[builder(default)]
    http_request_after_action_handlers: Vec<HTTPAfterActionHandler>,

    #[get = "pub"]
    #[builder(default = "default::http_request_handler()")]
    http_request_handler: HTTPHandler,

    #[get = "pub"]
    #[builder(default)]
    domains_manager: DomainsManager,
}

mod default {
    use super::*;

    #[inline]
    pub const fn use_https() -> bool {
        true
    }

    #[inline]
    pub const fn uc_host() -> Cow<'static, str> {
        Cow::Borrowed("uc.qbox.me")
    }

    #[inline]
    pub const fn rs_host() -> Cow<'static, str> {
        Cow::Borrowed("rs.qbox.me")
    }

    #[inline]
    pub const fn rsf_host() -> Cow<'static, str> {
        Cow::Borrowed("rsf.qbox.me")
    }

    #[inline]
    pub const fn api_host() -> Cow<'static, str> {
        Cow::Borrowed("api.qiniu.com")
    }

    #[inline]
    pub const fn uplog_url() -> Cow<'static, str> {
        Cow::Borrowed("https://uplog.qbox.me")
    }

    #[inline]
    pub const fn upload_token_lifetime() -> Duration {
        Duration::from_secs(60 * 60)
    }

    #[inline]
    pub const fn batch_max_operation_size() -> usize {
        1000
    }

    #[inline]
    pub const fn upload_threshold() -> u32 {
        1 << 22
    }

    #[inline]
    pub const fn upload_block_size() -> u32 {
        1 << 22
    }

    #[inline]
    pub fn upload_logger() -> Option<UploadLogger> {
        UploadLoggerBuilder::default().build().map(Some).unwrap_or(None)
    }

    #[inline]
    pub const fn http_request_retries() -> usize {
        3
    }

    #[inline]
    pub const fn http_request_retry_delay() -> Duration {
        Duration::from_secs(1)
    }

    #[inline]
    pub fn http_request_handler() -> HTTPHandler {
        #[cfg(any(feature = "use-libcurl"))]
        {
            HTTPHandler::Dynamic(Box::new(qiniu_with_libcurl::CurlClient::default()))
        }
        #[cfg(not(feature = "use-libcurl"))]
        {
            use crate::http::PanickedHTTPCaller;
            HTTPHandler::Dynamic(Box::new(PanickedHTTPCaller("Must define config.http_request_call")))
        }
    }
}

impl fmt::Debug for ConfigInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Config")
            .field("use_https", &self.use_https)
            .field("uc_host", &self.uc_host)
            .field("rs_host", &self.rs_host)
            .field("rsf_host", &self.rsf_host)
            .field("api_host", &self.api_host)
            .field("uplog_url", &self.uplog_url)
            .field("upload_token_lifetime", &self.upload_token_lifetime)
            .field("batch_max_operation_size", &self.batch_max_operation_size)
            .field("upload_threshold", &self.upload_threshold)
            .field("upload_block_size", &self.upload_block_size)
            .field("upload_recorder", &self.upload_recorder)
            .field("upload_logger", &self.upload_logger)
            .field("http_request_retries", &self.http_request_retries)
            .field("http_request_retry_delay", &self.http_request_retry_delay)
            .field("domains_manager", &self.domains_manager)
            .finish()
    }
}

impl ConfigInner {
    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_ref().map(|user_agent| user_agent.as_ref())
    }

    pub fn uc_url(&self) -> String {
        if self.use_https {
            "https://".to_owned() + self.uc_host.as_ref()
        } else {
            "http://".to_owned() + self.uc_host.as_ref()
        }
    }

    pub fn rs_url(&self) -> String {
        if self.use_https {
            "https://".to_owned() + self.rs_host.as_ref()
        } else {
            "http://".to_owned() + self.rs_host.as_ref()
        }
    }

    pub fn rsf_url(&self) -> String {
        if self.use_https {
            "https://".to_owned() + self.rsf_host.as_ref()
        } else {
            "http://".to_owned() + self.rsf_host.as_ref()
        }
    }

    pub fn api_url(&self) -> String {
        if self.use_https {
            "https://".to_owned() + self.api_host.as_ref()
        } else {
            "http://".to_owned() + self.api_host.as_ref()
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config(Arc<ConfigInner>);

impl ConfigBuilder {
    pub fn append_http_request_before_action_handler(mut self, handler: HTTPBeforeActionHandler) -> Self {
        if let Some(before_action_handlers) = &mut self.http_request_before_action_handlers {
            before_action_handlers.push(handler);
        } else {
            let mut handlers = Vec::with_capacity(1);
            handlers.push(handler);
            self.http_request_before_action_handlers = Some(handlers);
        }
        self
    }

    pub fn prepend_http_request_before_action_handler(mut self, handler: HTTPBeforeActionHandler) -> Self {
        if let Some(before_action_handlers) = &mut self.http_request_before_action_handlers {
            before_action_handlers.insert(0, handler);
        } else {
            let mut handlers = Vec::with_capacity(1);
            handlers.push(handler);
            self.http_request_before_action_handlers = Some(handlers);
        }
        self
    }

    pub fn append_http_request_after_action_handler(mut self, handler: HTTPAfterActionHandler) -> Self {
        if let Some(after_action_handlers) = &mut self.http_request_after_action_handlers {
            after_action_handlers.push(handler);
        } else {
            let mut handlers = Vec::with_capacity(1);
            handlers.push(handler);
            self.http_request_after_action_handlers = Some(handlers);
        }
        self
    }

    pub fn prepend_http_request_after_action_handler(mut self, handler: HTTPAfterActionHandler) -> Self {
        if let Some(after_action_handlers) = &mut self.http_request_after_action_handlers {
            after_action_handlers.insert(0, handler);
        } else {
            let mut handlers = Vec::with_capacity(1);
            handlers.push(handler);
            self.http_request_after_action_handlers = Some(handlers);
        }
        self
    }

    pub fn build(self) -> Config {
        let mut config = self.inner_build().unwrap();
        config.user_agent = Some(
            format!(
                "QiniuRust/qiniu-ng-{}/rust-{}{}",
                env!("CARGO_PKG_VERSION"),
                rustc_version_runtime::version(),
                config.user_agent.map_or(Cow::Borrowed("/"), |user_agent| Cow::Owned(
                    "/".to_owned() + &user_agent + "/"
                ))
            )
            .into(),
        );
        Config(Arc::new(config))
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
    pub fn into_raw(self) -> *const ConfigInner {
        Arc::into_raw(self.0)
    }

    pub unsafe fn from_raw(ptr: *const ConfigInner) -> Config {
        Config(Arc::from_raw(ptr))
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qiniu_http::{HTTPCaller, Request, RequestBuilder, Response, ResponseBuilder, Result};
    use regex::Regex;
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
                .stream(Cursor::new(Vec::from(b"It's HTTP Body".as_ref())))
                .build())
        }
    }

    #[test]
    fn test_config_with_set_user_agent() -> StdResult<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .user_agent(Some("fake_for_test".into()))
            .build();
        assert!(Regex::new("QiniuRust/qiniu-ng-[^/]+/rust-[^/]+/fake_for_test/")
            .unwrap()
            .is_match(config.user_agent().as_ref().unwrap()));
        Ok(())
    }

    #[test]
    fn test_config_with_set_dynamic_http_request_call() -> StdResult<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .http_request_retries(5)
            .http_request_retry_delay(Duration::from_secs(1))
            .http_request_handler(HTTPHandler::Dynamic(Box::new(FakeHTTPRequester)))
            .build();

        let http_response = config
            .http_request_handler()
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
            .http_request_handler(HTTPHandler::Dynamic(Box::new(FakeHTTPRequester)))
            .build();
        assert_eq!(config.http_request_retries(), 5);
        assert_eq!(config.http_request_retry_delay(), Duration::from_secs(1));
        Ok(())
    }
}
