use derive_builder::Builder;
use getset::{Getters, MutGetters, Setters};
use qiniu_http::HTTPCaller;
use std::{boxed::Box, default::Default, time::Duration};

#[derive(Builder, Getters, Setters, MutGetters)]
#[get = "pub"]
#[set = "pub"]
#[get_mut = "pub"]
#[builder(pattern = "owned", default)]
pub struct Config {
    use_https: bool,
    batch_max_operation_size: usize,
    upload_threshold: usize,
    http_request_retries: usize,
    http_request_retry_delay: Duration,
    http_request_call: Box<HTTPCaller>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            use_https: false,
            batch_max_operation_size: 1000,
            upload_threshold: 1 << 22,
            http_request_retries: 3,
            http_request_retry_delay: Duration::from_millis(500),
            http_request_call: Self::default_http_request_call(),
        }
    }
}

impl Config {
    fn default_http_request_call() -> Box<HTTPCaller> {
        #[cfg(any(feature = "use-reqwest"))]
        {
            use qiniu_with_reqwest::ReqwestClient;
            Box::new(ReqwestClient::default())
        }
        #[cfg(not(feature = "use-reqwest"))]
        {
            use super::http::PanickedHTTPCaller;
            Box::new(PanickedHTTPCaller("Must define config.http_request_call"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qiniu_http::{Request, RequestBuilder, Response, ResponseBuilder, Result};
    use std::{io::Read, iter};
    use stringreader::StringReader;

    struct FakeHTTPRequester;
    impl HTTPCaller for FakeHTTPRequester {
        fn call(&self, _: Request) -> Result<Response> {
            Ok(ResponseBuilder::default()
                .status_code(612u16)
                .body(Box::new(StringReader::new("It's HTTP Body")) as Box<Read>)
                .build())
        }
    }

    #[test]
    fn test_config_with_set_http_request_call() {
        let config: Config = ConfigBuilder::default()
            .http_request_retries(5)
            .http_request_retry_delay(Duration::from_secs(1))
            .http_request_call(Box::new(FakeHTTPRequester))
            .build()
            .unwrap();

        let http_response = config
            .http_request_call()
            .call(
                RequestBuilder::default()
                    .url("http://fake.qiniu.com")
                    .body(Vec::new())
                    .build(),
            )
            .unwrap();

        let mut http_body = iter::repeat(0)
            .take("It's HTTP Body".len())
            .collect::<Vec<u8>>();
        assert_eq!(http_response.status_code(), &612u16);
        http_response
            .into_body()
            .unwrap()
            .read(http_body.as_mut_slice())
            .unwrap();
        assert_eq!(
            String::from_utf8(http_body).unwrap().as_str(),
            "It's HTTP Body"
        );
    }

    #[test]
    fn test_config_with_getters_setters() {
        let mut config: Config = ConfigBuilder::default()
            .http_request_retries(5)
            .http_request_retry_delay(Duration::from_secs(1))
            .http_request_call(Box::new(FakeHTTPRequester))
            .build()
            .unwrap();
        assert_eq!(config.http_request_retries(), &5);
        assert_eq!(config.http_request_retry_delay(), &Duration::from_secs(1));

        *config.http_request_retries_mut() = 10;
        config.set_http_request_retry_delay(Duration::from_secs(2));
        assert_eq!(config.http_request_retries(), &10);
        assert_eq!(config.http_request_retry_delay(), &Duration::from_secs(2));
    }
}
