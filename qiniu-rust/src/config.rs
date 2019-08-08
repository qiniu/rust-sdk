use derive_builder::Builder;
use http::{Request, Response};
use std::{
    boxed::Box,
    error::Error,
    io::{Read, Write},
    result::Result,
    time::Duration,
};

#[derive(Builder)]
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

    http_request_call: Box<Fn(Request<Box<Write>>) -> Result<Response<Box<Read>>, Box<Error>>>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            use_https: false,
            batch_max_operation_size: 1000,
            upload_threshold: 1 << 22,
            http_request_retries: 3,
            http_request_retry_delay: Duration::from_millis(500),
            http_request_call: Box::new(|_| {
                panic!("Must define config.http_request_call");
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;
    use std::{io, iter};
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

    #[test]
    fn test_config_with_set_http_request_call() {
        let mut config: Config = ConfigBuilder::default()
            .http_request_retries(5)
            .http_request_retry_delay(Duration::from_secs(1))
            .http_request_call(Box::new(|_| {
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(Box::new(StringReader::new("It's HTTP Body")) as Box<Read>)
                    .unwrap())
            }))
            .build()
            .unwrap();

        let mut http_response = (config.http_request_call)(
            Request::builder()
                .uri("http://fake.qiniu.com")
                .body(Box::new(io::sink()) as Box<Write>)
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
}
