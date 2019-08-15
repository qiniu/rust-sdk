use super::Parts;
use error_chain::error_chain;
use qiniu_http::{RequestBuilder, Response};
use std::thread;

#[derive(Debug, Clone)]
pub struct Request {
    pub(super) parts: Parts,
}

impl Request {
    pub fn send(&self) -> qiniu_http::Result<Response> {
        let mut prev_err: Option<qiniu_http::Error> = None;
        for host in self.parts.hosts.iter() {
            match self.try_host(host) {
                Ok(response) => {
                    return Ok(response);
                }
                Err(err) => match err.kind() {
                    qiniu_http::ErrorKind::RetryableError
                    | qiniu_http::ErrorKind::HostUnretryableError => {
                        prev_err = Some(err);
                        // TODO: call callback function for retryable error
                        continue;
                    }
                    _ => {
                        return Err(err);
                    }
                },
            }
        }
        Err(prev_err.unwrap_or_else(|| {
            qiniu_http::Error::new_host_unretryable_error_from_parts(
                Error::from(ErrorKind::NoHostAvailable),
                Some(self.parts.method.to_owned()),
                None,
            )
        }))
    }

    fn try_host(&self, host: &String) -> qiniu_http::Result<Response> {
        let mut url = host.to_owned();
        url.push_str(&self.parts.path);
        let mut request = RequestBuilder::default()
            .method(self.parts.method)
            .url(&url)
            .headers(self.parts.headers.to_owned())
            .body(&self.parts.body)
            .build();
        self.parts.token.sign(&mut request, &self.parts.auth);
        let mut prev_err: Option<qiniu_http::Error> = None;
        let retries = *self.parts.config.http_request_retries();
        for _retried in 0..=retries {
            match self.parts.config.http_request_call().call(&request) {
                Ok(response) => {
                    return self.check_response(response);
                }
                Err(err) => {
                    match err.kind() {
                        qiniu_http::ErrorKind::RetryableError => {
                            prev_err = Some(err);
                            // TODO: call callback function for retryable error
                            if self.parts.config.http_request_retry_delay().as_nanos() > 0 {
                                thread::sleep(*self.parts.config.http_request_retry_delay());
                            }
                            continue;
                        }
                        _ => {
                            return Err(err);
                        }
                    }
                }
            }
        }
        Err(prev_err.unwrap())
    }

    fn check_response(&self, response: Response) -> qiniu_http::Result<Response> {
        Ok(response)
    }
}

error_chain! {
    errors {
        NoHostAvailable {
            description("no host is available"),
            display("no host is available"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::super::{
            super::{
                config::{Config, ConfigBuilder},
                utils::auth::Auth,
            },
            token::Token,
        },
        super::Builder,
        *,
    };
    use qiniu_http::{Error, ErrorKind, HTTPCaller, Method};
    use std::{boxed::Box, cell::Cell, io, sync::Arc, time::Duration};

    struct HTTPRequestCounter {
        counter: Arc<Cell<usize>>,
        error_kind: ErrorKind,
    }

    impl HTTPCaller for HTTPRequestCounter {
        fn call(&self, request: &qiniu_http::Request) -> qiniu_http::Result<Response> {
            assert!(request.headers().contains_key("Authorization"));
            self.counter.set(self.counter.get() + 1);
            Err(Error::new_from_parts(
                self.error_kind.clone(),
                io::Error::new(io::ErrorKind::Other, "Test Error"),
                None,
                None,
            ))
        }
    }

    const RETRIES: usize = 5;

    #[test]
    fn test_retryable_error() {
        let counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestCounter {
                counter: counter.clone(),
                error_kind: ErrorKind::RetryableError,
            }))
            .build()
            .unwrap();
        assert!(Builder::new(
            Arc::new(get_auth()),
            Arc::new(config),
            Method::GET,
            "/test_call",
            vec!["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body(b"{\"test\":123}".as_ref())
        .unwrap()
        .send()
        .is_err());
        assert_eq!(Arc::try_unwrap(counter).unwrap().get(), 2 * (RETRIES + 1));
    }

    #[test]
    fn test_host_unretryable_error() {
        let counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestCounter {
                counter: counter.clone(),
                error_kind: ErrorKind::HostUnretryableError,
            }))
            .build()
            .unwrap();
        assert!(Builder::new(
            Arc::new(get_auth()),
            Arc::new(config),
            Method::GET,
            "/test_call",
            vec!["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body(b"{\"test\":123}".as_ref())
        .unwrap()
        .send()
        .is_err());
        assert_eq!(Arc::try_unwrap(counter).unwrap().get(), 2);
    }

    #[test]
    fn test_zone_unretryable_error() {
        let counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestCounter {
                counter: counter.clone(),
                error_kind: ErrorKind::ZoneUnretryableError,
            }))
            .build()
            .unwrap();
        assert!(Builder::new(
            Arc::new(get_auth()),
            Arc::new(config),
            Method::GET,
            "/test_call",
            vec!["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body(b"{\"test\":123}".as_ref())
        .unwrap()
        .send()
        .is_err());
        assert_eq!(Arc::try_unwrap(counter).unwrap().get(), 1);
    }

    #[test]
    fn test_unretryable_error() {
        let counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestCounter {
                counter: counter.clone(),
                error_kind: ErrorKind::UnretryableError,
            }))
            .build()
            .unwrap();
        assert!(Builder::new(
            Arc::new(get_auth()),
            Arc::new(config),
            Method::GET,
            "/test_call",
            vec!["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body(b"{\"test\":123}".as_ref())
        .unwrap()
        .send()
        .is_err());
        assert_eq!(Arc::try_unwrap(counter).unwrap().get(), 1);
    }

    fn get_auth() -> Auth {
        Auth::new("abcdefghklmnopq", "1234567890")
    }
}
