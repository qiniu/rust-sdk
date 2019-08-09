use super::http::PanickedHTTPCaller;
use derive_builder::Builder;
use getset::{Getters, MutGetters, Setters};
use qiniu_http::HTTPCaller;
use std::{boxed::Box, default::Default, time::Duration};

#[derive(Builder, Getters, Setters, MutGetters)]
#[get = "pub"]
#[set = "pub"]
#[get_mut = "pub"]
#[builder(pattern = "owned")]
pub struct Config {
    #[builder(default)]
    use_https: bool,

    #[builder(default)]
    batch_max_operation_size: usize,

    #[builder(default)]
    upload_threshold: usize,

    #[builder(default)]
    http_request_retries: usize,

    #[builder(default)]
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
            Box::new(PanickedHTTPCaller("Must define config.http_request_call"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{Request, Response, StatusCode};
    use std::{error::Error, io::Read, iter};
    use stringreader::StringReader;

    #[test]
    #[should_panic]
    fn test_config_without_set_http_request_call() {
        ConfigBuilder::default()
            .use_https(true)
            .batch_max_operation_size(10000)
            .build()
            .unwrap();
    }

    struct FakeHTTPRequester;

    impl HTTPCaller for FakeHTTPRequester {
        fn call(&self, _: Request<Vec<u8>>) -> Result<Response<Box<Read>>, Box<Error>> {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Box::new(StringReader::new("It's HTTP Body")) as Box<Read>)
                .unwrap())
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

        let mut http_response = config
            .http_request_call()
            .call(
                Request::builder()
                    .uri("http://fake.qiniu.com")
                    .body(Vec::new())
                    .unwrap(),
            )
            .unwrap();

        let mut http_body = iter::repeat(0)
            .take("It's HTTP Body".len())
            .collect::<Vec<u8>>();
        assert_eq!(http_response.status(), StatusCode::OK);
        http_response
            .body_mut()
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
