use super::Parts;
use error_chain::error_chain;
use qiniu_http::{Method, RequestBuilder, Response};
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
                    qiniu_http::ErrorKind::RetryableError | qiniu_http::ErrorKind::HostUnretryableError
                        if self.is_idempotent(&err) =>
                    {
                        self.parts.config.http_request_call().on_host_failed(host, &err);
                        prev_err = Some(err);
                        continue;
                    }
                    _ => {
                        self.parts.config.http_request_call().on_error(&err);
                        return Err(err);
                    }
                },
            }
        }
        let err = prev_err.unwrap_or_else(|| {
            qiniu_http::Error::new_host_unretryable_error_from_parts(
                Error::from(ErrorKind::NoHostAvailable),
                true,
                Some(self.parts.method.to_owned()),
                None,
            )
        });
        self.parts.config.http_request_call().on_error(&err);
        Err(err)
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
        self.parts.config.http_request_call().on_request_built(&mut request);
        let mut prev_err: Option<qiniu_http::Error> = None;
        let retries = *self.parts.config.http_request_retries();
        for retried in 0..=retries {
            match self
                .parts
                .config
                .http_request_call()
                .call(&request)
                .and_then(|response| self.check_response(response))
            {
                Ok(response) => {
                    self.parts.config.http_request_call().on_response(&request, &response);
                    return Ok(response);
                }
                Err(err) => match err.kind() {
                    qiniu_http::ErrorKind::RetryableError if self.is_idempotent(&err) => {
                        self.parts
                            .config
                            .http_request_call()
                            .on_retry_request(&request, &err, retried, retries);
                        prev_err = Some(err);
                        if self.parts.config.http_request_retry_delay().as_nanos() > 0 {
                            thread::sleep(*self.parts.config.http_request_retry_delay());
                        }
                        continue;
                    }
                    _ => {
                        return Err(err);
                    }
                },
            }
        }
        Err(prev_err.unwrap())
    }

    fn is_idempotent(&self, err: &qiniu_http::Error) -> bool {
        match self.parts.method {
            Method::GET | Method::PUT | Method::HEAD | Method::PATCH | Method::DELETE => true,
            _ => *err.is_retry_safe(),
        }
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
        super::{
            super::{
                super::{
                    config::{Config, ConfigBuilder},
                    utils::auth::Auth,
                },
                token::Token,
            },
            Builder,
        },
        *,
    };
    use qiniu_http::{Error, ErrorKind, HTTPCaller};
    use std::{boxed::Box, cell::Cell, io, sync::Arc, time::Duration};

    struct HTTPRequestCounter {
        call_counter: Arc<Cell<usize>>,
        on_retry_request_counter: Arc<Cell<usize>>,
        on_host_failed_counter: Arc<Cell<usize>>,
        on_request_built_counter: Arc<Cell<usize>>,
        on_response_counter: Arc<Cell<usize>>,
        on_error_counter: Arc<Cell<usize>>,
        is_retry_safe: bool,
        error_kind: ErrorKind,
    }

    impl HTTPCaller for HTTPRequestCounter {
        fn call(&self, request: &qiniu_http::Request) -> qiniu_http::Result<Response> {
            assert!(request.headers().contains_key("Authorization"));
            self.call_counter.set(self.call_counter.get() + 1);
            Err(Error::new_from_parts(
                self.error_kind.clone(),
                io::Error::new(io::ErrorKind::Other, "Test Error"),
                self.is_retry_safe,
                None,
                None,
            ))
        }
        fn on_retry_request(&self, _request: &qiniu_http::Request, _error: &Error, _retried: usize, _retries: usize) {
            self.on_retry_request_counter
                .set(self.on_retry_request_counter.get() + 1);
        }
        fn on_host_failed(&self, _failed_host: &str, _error: &qiniu_http::Error) {
            self.on_host_failed_counter.set(self.on_host_failed_counter.get() + 1);
        }
        fn on_request_built(&self, _request: &mut qiniu_http::Request) {
            self.on_request_built_counter
                .set(self.on_request_built_counter.get() + 1);
        }
        fn on_response(&self, _request: &qiniu_http::Request, _response: &qiniu_http::Response) {
            self.on_response_counter.set(self.on_response_counter.get() + 1);
        }
        fn on_error(&self, _err: &qiniu_http::Error) {
            self.on_error_counter.set(self.on_error_counter.get() + 1);
        }
    }

    const RETRIES: usize = 5;

    #[test]
    fn test_retryable_error_case_1() {
        let call_counter = Arc::new(Cell::new(0));
        let on_retry_request_counter = Arc::new(Cell::new(0));
        let on_host_failed_counter = Arc::new(Cell::new(0));
        let on_request_built_counter = Arc::new(Cell::new(0));
        let on_response_counter = Arc::new(Cell::new(0));
        let on_error_counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestCounter {
                call_counter: call_counter.clone(),
                on_retry_request_counter: on_retry_request_counter.clone(),
                on_host_failed_counter: on_host_failed_counter.clone(),
                on_request_built_counter: on_request_built_counter.clone(),
                on_response_counter: on_response_counter.clone(),
                on_error_counter: on_error_counter.clone(),
                error_kind: ErrorKind::RetryableError,
                is_retry_safe: true,
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
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .unwrap()
        .send()
        .is_err());
        assert_eq!(Arc::try_unwrap(call_counter).unwrap().get(), 2 * (RETRIES + 1));
        assert_eq!(
            Arc::try_unwrap(on_retry_request_counter).unwrap().get(),
            2 * (RETRIES + 1)
        );
        assert_eq!(Arc::try_unwrap(on_host_failed_counter).unwrap().get(), 2);
        assert_eq!(Arc::try_unwrap(on_request_built_counter).unwrap().get(), 2);
        assert_eq!(Arc::try_unwrap(on_response_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_error_counter).unwrap().get(), 1);
    }

    #[test]
    fn test_retryable_error_case_2() {
        let call_counter = Arc::new(Cell::new(0));
        let on_retry_request_counter = Arc::new(Cell::new(0));
        let on_host_failed_counter = Arc::new(Cell::new(0));
        let on_request_built_counter = Arc::new(Cell::new(0));
        let on_response_counter = Arc::new(Cell::new(0));
        let on_error_counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestCounter {
                call_counter: call_counter.clone(),
                on_retry_request_counter: on_retry_request_counter.clone(),
                on_host_failed_counter: on_host_failed_counter.clone(),
                on_request_built_counter: on_request_built_counter.clone(),
                on_response_counter: on_response_counter.clone(),
                on_error_counter: on_error_counter.clone(),
                error_kind: ErrorKind::RetryableError,
                is_retry_safe: true,
            }))
            .build()
            .unwrap();
        assert!(Builder::new(
            Arc::new(get_auth()),
            Arc::new(config),
            Method::POST,
            "/test_call",
            vec!["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .unwrap()
        .send()
        .is_err());
        assert_eq!(Arc::try_unwrap(call_counter).unwrap().get(), 2 * (RETRIES + 1));
        assert_eq!(
            Arc::try_unwrap(on_retry_request_counter).unwrap().get(),
            2 * (RETRIES + 1)
        );
        assert_eq!(Arc::try_unwrap(on_host_failed_counter).unwrap().get(), 2);
        assert_eq!(Arc::try_unwrap(on_request_built_counter).unwrap().get(), 2);
        assert_eq!(Arc::try_unwrap(on_response_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_error_counter).unwrap().get(), 1);
    }

    #[test]
    fn test_retryable_error_case_3() {
        let call_counter = Arc::new(Cell::new(0));
        let on_retry_request_counter = Arc::new(Cell::new(0));
        let on_host_failed_counter = Arc::new(Cell::new(0));
        let on_request_built_counter = Arc::new(Cell::new(0));
        let on_response_counter = Arc::new(Cell::new(0));
        let on_error_counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestCounter {
                call_counter: call_counter.clone(),
                on_retry_request_counter: on_retry_request_counter.clone(),
                on_host_failed_counter: on_host_failed_counter.clone(),
                on_request_built_counter: on_request_built_counter.clone(),
                on_response_counter: on_response_counter.clone(),
                on_error_counter: on_error_counter.clone(),
                error_kind: ErrorKind::RetryableError,
                is_retry_safe: false,
            }))
            .build()
            .unwrap();
        assert!(Builder::new(
            Arc::new(get_auth()),
            Arc::new(config),
            Method::POST,
            "/test_call",
            vec!["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .unwrap()
        .send()
        .is_err());
        assert_eq!(Arc::try_unwrap(call_counter).unwrap().get(), 1);
        assert_eq!(Arc::try_unwrap(on_retry_request_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_host_failed_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_request_built_counter).unwrap().get(), 1);
        assert_eq!(Arc::try_unwrap(on_response_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_error_counter).unwrap().get(), 1);
    }

    #[test]
    fn test_host_unretryable_error() {
        let call_counter = Arc::new(Cell::new(0));
        let on_retry_request_counter = Arc::new(Cell::new(0));
        let on_host_failed_counter = Arc::new(Cell::new(0));
        let on_request_built_counter = Arc::new(Cell::new(0));
        let on_response_counter = Arc::new(Cell::new(0));
        let on_error_counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestCounter {
                call_counter: call_counter.clone(),
                on_retry_request_counter: on_retry_request_counter.clone(),
                on_host_failed_counter: on_host_failed_counter.clone(),
                on_request_built_counter: on_request_built_counter.clone(),
                on_response_counter: on_response_counter.clone(),
                on_error_counter: on_error_counter.clone(),
                error_kind: ErrorKind::HostUnretryableError,
                is_retry_safe: true,
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
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .unwrap()
        .send()
        .is_err());
        assert_eq!(Arc::try_unwrap(call_counter).unwrap().get(), 2);
        assert_eq!(Arc::try_unwrap(on_retry_request_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_host_failed_counter).unwrap().get(), 2);
        assert_eq!(Arc::try_unwrap(on_request_built_counter).unwrap().get(), 2);
        assert_eq!(Arc::try_unwrap(on_response_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_error_counter).unwrap().get(), 1);
    }

    #[test]
    fn test_zone_unretryable_error() {
        let call_counter = Arc::new(Cell::new(0));
        let on_retry_request_counter = Arc::new(Cell::new(0));
        let on_host_failed_counter = Arc::new(Cell::new(0));
        let on_request_built_counter = Arc::new(Cell::new(0));
        let on_response_counter = Arc::new(Cell::new(0));
        let on_error_counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestCounter {
                call_counter: call_counter.clone(),
                on_retry_request_counter: on_retry_request_counter.clone(),
                on_host_failed_counter: on_host_failed_counter.clone(),
                on_request_built_counter: on_request_built_counter.clone(),
                on_response_counter: on_response_counter.clone(),
                on_error_counter: on_error_counter.clone(),
                error_kind: ErrorKind::ZoneUnretryableError,
                is_retry_safe: false,
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
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .unwrap()
        .send()
        .is_err());
        assert_eq!(Arc::try_unwrap(call_counter).unwrap().get(), 1);
        assert_eq!(Arc::try_unwrap(on_retry_request_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_host_failed_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_request_built_counter).unwrap().get(), 1);
        assert_eq!(Arc::try_unwrap(on_response_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_error_counter).unwrap().get(), 1);
    }

    #[test]
    fn test_unretryable_error() {
        let call_counter = Arc::new(Cell::new(0));
        let on_retry_request_counter = Arc::new(Cell::new(0));
        let on_host_failed_counter = Arc::new(Cell::new(0));
        let on_request_built_counter = Arc::new(Cell::new(0));
        let on_response_counter = Arc::new(Cell::new(0));
        let on_error_counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestCounter {
                call_counter: call_counter.clone(),
                on_retry_request_counter: on_retry_request_counter.clone(),
                on_host_failed_counter: on_host_failed_counter.clone(),
                on_request_built_counter: on_request_built_counter.clone(),
                on_response_counter: on_response_counter.clone(),
                on_error_counter: on_error_counter.clone(),
                error_kind: ErrorKind::UnretryableError,
                is_retry_safe: false,
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
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .unwrap()
        .send()
        .is_err());
        assert_eq!(Arc::try_unwrap(call_counter).unwrap().get(), 1);
        assert_eq!(Arc::try_unwrap(on_retry_request_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_host_failed_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_request_built_counter).unwrap().get(), 1);
        assert_eq!(Arc::try_unwrap(on_response_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_error_counter).unwrap().get(), 1);
    }

    fn get_auth() -> Auth {
        Auth::new("abcdefghklmnopq", "1234567890")
    }
}
