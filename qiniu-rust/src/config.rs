use super::http::DomainsManager;
use derive_builder::Builder;
use getset::{CopyGetters, Getters};
use qiniu_http::HTTPCaller;
use std::{
    boxed::Box,
    default::Default,
    fmt,
    marker::{Send, Sync},
    ops::Deref,
    result,
    sync::Arc,
    time::Duration,
};

#[derive(Builder, Getters, CopyGetters)]
#[builder(
    name = "ConfigBuilder",
    pattern = "owned",
    default,
    public,
    build_fn(name = "inner_build", private)
)]
pub struct ConfigInner {
    #[get_copy = "pub"]
    use_https: bool,

    #[get_copy = "pub"]
    upload_token_lifetime: Duration,

    #[get_copy = "pub"]
    batch_max_operation_size: usize,

    #[get_copy = "pub"]
    upload_threshold: usize,

    #[get_copy = "pub"]
    http_request_retries: usize,

    #[get_copy = "pub"]
    http_request_retry_delay: Duration,

    #[get = "pub"]
    http_request_call: Box<dyn HTTPCaller + Send + Sync>,

    #[get = "pub"]
    domains_manager: DomainsManager,

    #[get_copy = "pub"]
    host_freeze_duration: Duration,
}

impl Default for ConfigInner {
    fn default() -> Self {
        ConfigInner {
            use_https: false,
            upload_token_lifetime: Duration::from_secs(60 * 60),
            batch_max_operation_size: 1000,
            upload_threshold: 1 << 22,
            http_request_retries: 3,
            http_request_retry_delay: Duration::from_millis(500),
            http_request_call: Self::default_http_request_call(),
            domains_manager: DomainsManager::new(),
            host_freeze_duration: Duration::from_secs(60 * 10),
        }
    }
}

impl ConfigInner {
    fn default_http_request_call() -> Box<dyn HTTPCaller + Sync + Send> {
        #[cfg(any(feature = "use-libcurl"))]
        {
            Box::new(qiniu_with_libcurl::CurlClientBuilder::default().build().unwrap())
        }
        #[cfg(not(feature = "use-libcurl"))]
        {
            use super::http::PanickedHTTPCaller;
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
            .field("http_request_retries", &self.http_request_retries)
            .field("http_request_retry_delay", &self.http_request_retry_delay)
            .field("host_freeze_duration", &self.host_freeze_duration)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct Config(Arc<ConfigInner>);

impl ConfigBuilder {
    pub fn build(self) -> result::Result<Config, String> {
        self.inner_build().map(|config| Config(Arc::new(config)))
    }
}

impl Default for Config {
    fn default() -> Self {
        Config(Arc::new(Default::default()))
    }
}

impl Deref for Config {
    type Target = ConfigInner;

    #[inline]
    fn deref(&self) -> &ConfigInner {
        self.0.deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qiniu_http::{Request, RequestBuilder, Response, ResponseBuilder, Result};
    use std::{io::Read, iter};

    struct FakeHTTPRequester;
    impl HTTPCaller for FakeHTTPRequester {
        fn call(&self, _: &Request) -> Result<Response> {
            Ok(ResponseBuilder::default()
                .status_code(612u16)
                .body(Box::new(stringreader::StringReader::new("It's HTTP Body")) as Box<dyn Read>)
                .build()
                .unwrap())
        }
    }

    #[test]
    fn test_config_with_set_http_request_call() {
        let config = ConfigBuilder::default()
            .http_request_retries(5)
            .http_request_retry_delay(Duration::from_secs(1))
            .http_request_call(Box::new(FakeHTTPRequester))
            .build()
            .unwrap();

        let http_response = config
            .http_request_call()
            .call(&RequestBuilder::default().url("http://fake.qiniu.com").build())
            .unwrap();

        let mut http_body = iter::repeat(0).take("It's HTTP Body".len()).collect::<Vec<u8>>();
        assert_eq!(http_response.status_code(), 612);
        http_response
            .into_body()
            .unwrap()
            .read(http_body.as_mut_slice())
            .unwrap();
        assert_eq!(String::from_utf8(http_body).unwrap().as_str(), "It's HTTP Body");
    }

    #[test]
    fn test_config_with_getters() {
        let config = ConfigBuilder::default()
            .http_request_retries(5)
            .http_request_retry_delay(Duration::from_secs(1))
            .http_request_call(Box::new(FakeHTTPRequester))
            .build()
            .unwrap();
        assert_eq!(config.http_request_retries(), 5);
        assert_eq!(config.http_request_retry_delay(), Duration::from_secs(1));
    }
}
