mod builder;
mod parts;

pub(crate) use builder::Builder;
pub(crate) use parts::Parts;

use super::{response::Response, Choice, DomainsManager};
use qiniu_http::{
    Error as HTTPError, ErrorKind as HTTPErrorKind, HeaderName, HeaderValue, Headers, Method, Request as HTTPRequest,
    RequestBuilder, Response as HTTPResponse, ResponseBody as HTTPResponseBody, Result as HTTPResult,
    RetryKind as HTTPRetryKind, StatusCode,
};
use rand::{thread_rng, Rng};
use serde::Deserialize;
use std::{
    fmt,
    io::{Error as IOError, ErrorKind as IOErrorKind, Read},
    thread::sleep,
    time::{Duration, Instant},
};
use url::Url;

#[derive(Deserialize)]
pub(super) struct RequestErrorResponse {
    pub(super) error: Option<String>,
}

pub(crate) struct Request<'a> {
    pub(super) parts: Parts<'a>,
    pub(super) domains_manager: DomainsManager,
}

impl<'a> Request<'a> {
    pub(crate) fn send(&self) -> HTTPResult<Response> {
        let mut prev_err: Option<HTTPError> = None;
        let choices = self.domains_manager.choose(self.parts.base_urls).map_err(|err| {
            HTTPError::new_host_unretryable_error_from_parts(
                HTTPErrorKind::UnknownError(Box::new(err)),
                true,
                Some(self.parts.method.to_owned()),
                None,
            )
        })?;
        for choice in choices {
            let base_url = choice.base_url;
            let timer = Instant::now();
            match self.try_choice(choice) {
                Ok(resp) => {
                    return Ok(resp);
                }
                Err(err) => match err.retry_kind() {
                    HTTPRetryKind::RetryableError | HTTPRetryKind::HostUnretryableError if self.is_retry_safe(&err) => {
                        self.domains_manager.freeze_url(base_url).unwrap();
                        if let Some(on_error) = &self.parts.on_error {
                            (on_error)(Some(base_url), &err, timer.elapsed());
                        }
                        prev_err = Some(err);
                        continue;
                    }
                    _ => {
                        if let Some(on_error) = &self.parts.on_error {
                            (on_error)(Some(base_url), &err, timer.elapsed());
                        }
                        return Err(err);
                    }
                },
            }
        }
        Err(prev_err.unwrap())
    }

    fn try_choice(&self, choice: Choice<'a>) -> HTTPResult<Response<'a>> {
        let mut request = {
            let mut builder = RequestBuilder::default()
                .method(self.parts.method)
                .url(self.make_url(choice.base_url)?)
                .user_agent(self.parts.config.user_agent())
                .connect_timeout(self.parts.config.http_connect_timeout())
                .request_timeout(self.parts.config.http_request_timeout())
                .tcp_keepalive_idle_timeout(self.parts.config.tcp_keepalive_idle_timeout())
                .tcp_keepalive_probe_interval(self.parts.config.tcp_keepalive_probe_interval())
                .low_transfer_speed(self.parts.config.http_low_transfer_speed())
                .low_transfer_speed_timeout(self.parts.config.http_low_transfer_speed_timeout())
                .follow_redirection(self.parts.follow_redirection);
            if !choice.socket_addrs.is_empty() {
                builder = builder.resolved_socket_addrs(choice.socket_addrs.as_ref());
            }
            if let Some(on_uploading_progress) = self.parts.on_uploading_progress {
                builder = builder.on_uploading_progress(on_uploading_progress);
            }
            if let Some(on_downloading_progress) = self.parts.on_downloading_progress {
                builder = builder.on_downloading_progress(on_downloading_progress);
            }
            if let Some(headers) = &self.parts.headers {
                builder = builder.headers(headers.to_owned());
            }
            if let Some(body) = &self.parts.body {
                builder = builder.body(body.as_slice());
            }
            builder.build()
        };
        if let Some(token) = &self.parts.token {
            token.sign(&mut request);
        }

        let mut prev_err: Option<HTTPError> = None;
        let retries = self.parts.config.http_request_retries();
        assert!(retries > 0);
        for _ in 0..=retries {
            let timer = Instant::now();
            match self
                .do_request(&mut request)
                .and_then(|response| Self::check_response(response, &request))
                .and_then(|response| self.fulfill_body_if_needed(response, &request))
                .map(|response| Response {
                    inner: response,
                    method: self.parts.method,
                    base_url: choice.base_url,
                    path: self.parts.path,
                })
                .and_then(|mut response| {
                    if let Some(on_response) = &self.parts.on_response {
                        (on_response)(&mut response, timer.elapsed())?;
                    }
                    Ok(response)
                }) {
                Ok(response) => {
                    return Ok(response);
                }
                Err(err) => match err.retry_kind() {
                    HTTPRetryKind::RetryableError if self.is_retry_safe(&err) => {
                        if let Some(on_error) = &self.parts.on_error {
                            (on_error)(Some(&choice.base_url), &err, timer.elapsed());
                        }
                        prev_err = Some(err);
                        let delay_nanos = self.parts.config.http_request_retry_delay().as_nanos() as u64;
                        if delay_nanos > 0 {
                            sleep(Duration::from_nanos(
                                thread_rng().gen_range(delay_nanos / 2, delay_nanos),
                            ));
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

    fn do_request(&self, request: &mut HTTPRequest) -> HTTPResult<HTTPResponse> {
        for handler in self.parts.config.http_request_before_action_handlers().iter() {
            handler.before_call(request)?;
        }
        let mut response = self.parts.config.http_request_handler().call(&request)?;
        for handler in self.parts.config.http_request_after_action_handlers().iter() {
            handler.after_call(request, &mut response)?;
        }
        Ok(response)
    }

    fn make_url(&self, base_url: &str) -> HTTPResult<String> {
        let mut url = base_url.to_owned() + self.parts.path;
        if let Some(query) = &self.parts.query {
            url = Url::parse_with_params(url.as_str(), query)
                .map_err(|err| {
                    HTTPError::new_unretryable_error_from_parts(
                        HTTPErrorKind::UnknownError(Box::new(err)),
                        Some(self.parts.method),
                        Some(url.into()),
                    )
                })?
                .into_string();
        }
        Ok(url)
    }

    fn is_retry_safe(&self, err: &HTTPError) -> bool {
        match self.parts.method {
            Method::GET | Method::PUT | Method::HEAD => true,
            _ => self.parts.idempotent || err.is_retry_safe(),
        }
    }

    fn check_response(mut response: HTTPResponse, request: &HTTPRequest) -> HTTPResult<HTTPResponse> {
        let status_code = response.status_code();
        if (200..300).contains(&status_code) {
            return Ok(response);
        }
        let mut error_message: Option<Box<str>> = None;
        if let Some(body) = Self::read_body_to_bytes(&mut response, request)? {
            if response.header("Content-Type") == Some(&"application/json".into()) {
                error_message = serde_json::from_slice::<RequestErrorResponse>(&body)
                    .map_err(|err| {
                        HTTPError::new_retryable_error(HTTPErrorKind::JSONError(err), false, request, Some(&response))
                    })?
                    .error
                    .map(|e| e.into())
            }
        }
        Err(Self::response_error(
            response.status_code(),
            error_message.unwrap_or_else(|| "(None)".into()),
            request,
            Some(&response),
        ))
    }

    fn fulfill_body_if_needed(&self, response: HTTPResponse, request: &HTTPRequest) -> HTTPResult<HTTPResponse> {
        if self.parts.read_body {
            Self::fulfill_body(response, request)
        } else {
            Ok(response)
        }
    }

    fn fulfill_body(mut response: HTTPResponse, request: &HTTPRequest) -> HTTPResult<HTTPResponse> {
        if let Some(body) = Self::read_body_to_bytes(&mut response, request)? {
            *response.body_mut() = Some(HTTPResponseBody::Bytes(body));
        }
        Ok(response)
    }

    fn read_body_to_bytes(response: &mut HTTPResponse, request: &HTTPRequest) -> HTTPResult<Option<Vec<u8>>> {
        let mut content_length = None::<usize>;
        if let Some(content_length_str) = response.header("Content-Length") {
            content_length = Some(content_length_str.parse().map_err(|err| {
                HTTPError::new_unretryable_error(HTTPErrorKind::UnknownError(Box::new(err)), request, Some(response))
            })?);
        }
        if let Some(body) = response.take_body() {
            let mut buf = Vec::<u8>::new();
            let body_len = match body {
                HTTPResponseBody::Reader(mut reader) => reader.read_to_end(&mut buf).map_err(|err| {
                    HTTPError::new_retryable_error(HTTPErrorKind::IOError(err), false, request, Some(response))
                })?,
                HTTPResponseBody::File(mut file) => file.read_to_end(&mut buf).map_err(|err| {
                    HTTPError::new_retryable_error(HTTPErrorKind::IOError(err), false, request, Some(response))
                })?,
                HTTPResponseBody::Bytes(body) => {
                    buf = body;
                    buf.len()
                }
            };
            if let Some(content_length) = content_length {
                if content_length != body_len {
                    return Err(HTTPError::new_retryable_error(
                        HTTPErrorKind::IOError(IOError::from(IOErrorKind::UnexpectedEof)),
                        false,
                        request,
                        Some(response),
                    ));
                }
            }
            Ok(Some(buf))
        } else {
            Ok(None)
        }
    }

    fn response_error(
        status_code: StatusCode,
        error_message: Box<str>,
        request: &HTTPRequest,
        response: Option<&HTTPResponse>,
    ) -> HTTPError {
        match status_code {
            300..=399 => HTTPError::new_unretryable_error(HTTPErrorKind::UnexpectedRedirect, request, response),
            400 if error_message.contains("incorrect region") => HTTPError::new_zone_unretryable_error(
                HTTPErrorKind::ResponseStatusCodeError(status_code, error_message),
                false,
                request,
                response,
            ),
            400..=499 | 501 | 573 | 608 | 612 | 614 | 615 | 616 | 619 | 630 | 631 | 640 | 701 => {
                HTTPError::new_unretryable_error(
                    HTTPErrorKind::ResponseStatusCodeError(status_code, error_message),
                    request,
                    response,
                )
            }
            502 | 503 | 571 => HTTPError::new_host_unretryable_error(
                HTTPErrorKind::ResponseStatusCodeError(status_code, error_message),
                true,
                request,
                response,
            ),
            _ => HTTPError::new_retryable_error(
                HTTPErrorKind::ResponseStatusCodeError(status_code, error_message),
                false,
                request,
                response,
            ),
        }
    }
}

impl fmt::Debug for Request<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Request").field("parts", &self.parts).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{
            super::{
                config::{Config, ConfigBuilder},
                credential::Credential,
            },
            DomainsManagerBuilder, HTTPAfterAction, HTTPBeforeAction, HTTPCaller, TokenVersion,
        },
        Builder, *,
    };
    use qiniu_test_utils::http_call_mock::{CounterCallMock, ErrorResponseMock};
    use std::{
        boxed::Box,
        error::Error as StdError,
        io,
        result::Result as StdResult,
        sync::{
            atomic::{AtomicUsize, Ordering::Relaxed},
            Arc,
        },
        time::Duration,
    };

    #[derive(Debug, Clone)]
    struct HTTPRetryer {
        is_retry_safe: bool,
        retry_kind: HTTPRetryKind,
    }

    impl HTTPCaller for HTTPRetryer {
        fn call(&self, request: &HTTPRequest) -> HTTPResult<HTTPResponse> {
            assert!(request.headers().contains_key(&"authorization".into()));
            Err(HTTPError::new_from_parts(
                self.retry_kind,
                HTTPErrorKind::IOError(io::Error::new(io::ErrorKind::Other, "Test Error")),
                self.is_retry_safe,
                None,
                None,
            ))
        }
    }

    const RETRIES: usize = 5;

    #[test]
    fn test_retryable_error_case_1() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRetryer {
            retry_kind: HTTPRetryKind::RetryableError,
            is_retry_safe: true,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let (on_response_called, on_error_called) = (AtomicUsize::new(0), AtomicUsize::new(0));
        assert!(Builder::new(
            config.clone(),
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .on_response(&|_, _| {
            on_response_called.fetch_add(1, Relaxed);
            Ok(())
        })
        .on_error(&|_, _, _| {
            on_error_called.fetch_add(1, Relaxed);
        })
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(config.domains_manager().is_frozen_url("http://z1h1.com:1111")?);
        assert!(config.domains_manager().is_frozen_url("http://z1h2.com:2222")?);

        assert_eq!(mock.call_called(), 2 * (RETRIES + 1));
        assert_eq!(on_response_called.load(Relaxed), 0);
        assert_eq!(on_error_called.load(Relaxed), 2 * (RETRIES + 1) + 2);
        Ok(())
    }

    #[test]
    fn test_retryable_error_case_2() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRetryer {
            retry_kind: HTTPRetryKind::RetryableError,
            is_retry_safe: true,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let (on_response_called, on_error_called) = (AtomicUsize::new(0), AtomicUsize::new(0));
        assert!(Builder::new(
            config.clone(),
            Method::POST,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .on_response(&|_, _| {
            on_response_called.fetch_add(1, Relaxed);
            Ok(())
        })
        .on_error(&|_, _, _| {
            on_error_called.fetch_add(1, Relaxed);
        })
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(config.domains_manager().is_frozen_url("http://z1h1.com:1111")?);
        assert!(config.domains_manager().is_frozen_url("http://z1h2.com:2222")?);

        assert_eq!(mock.call_called(), 2 * (RETRIES + 1));
        assert_eq!(on_response_called.load(Relaxed), 0);
        assert_eq!(on_error_called.load(Relaxed), 2 * (RETRIES + 1) + 2);
        Ok(())
    }

    #[test]
    fn test_retryable_error_case_3() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRetryer {
            retry_kind: HTTPRetryKind::RetryableError,
            is_retry_safe: false,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let (on_response_called, on_error_called) = (AtomicUsize::new(0), AtomicUsize::new(0));
        assert!(Builder::new(
            config.clone(),
            Method::POST,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .on_response(&|_, _| {
            on_response_called.fetch_add(1, Relaxed);
            Ok(())
        })
        .on_error(&|_, _, _| {
            on_error_called.fetch_add(1, Relaxed);
        })
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(!config.domains_manager().is_frozen_url("http://z1h1.com:1111")?);
        assert!(!config.domains_manager().is_frozen_url("http://z1h2.com:2222")?);

        assert_eq!(mock.call_called(), 1);
        assert_eq!(on_response_called.load(Relaxed), 0);
        assert_eq!(on_error_called.load(Relaxed), 1);
        Ok(())
    }

    #[test]
    fn test_host_unretryable_error() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRetryer {
            retry_kind: HTTPRetryKind::HostUnretryableError,
            is_retry_safe: true,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let (on_response_called, on_error_called) = (AtomicUsize::new(0), AtomicUsize::new(0));
        assert!(Builder::new(
            config.clone(),
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .on_response(&|_, _| {
            on_response_called.fetch_add(1, Relaxed);
            Ok(())
        })
        .on_error(&|_, _, _| {
            on_error_called.fetch_add(1, Relaxed);
        })
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(config.domains_manager().is_frozen_url("http://z1h1.com:1111")?);
        assert!(config.domains_manager().is_frozen_url("http://z1h2.com:2222")?);

        assert_eq!(mock.call_called(), 2);
        assert_eq!(on_response_called.load(Relaxed), 0);
        assert_eq!(on_error_called.load(Relaxed), 2);
        Ok(())
    }

    #[test]
    fn test_zone_unretryable_error() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRetryer {
            retry_kind: HTTPRetryKind::ZoneUnretryableError,
            is_retry_safe: false,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let (on_response_called, on_error_called) = (AtomicUsize::new(0), AtomicUsize::new(0));
        assert!(Builder::new(
            config.clone(),
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .on_response(&|_, _| {
            on_response_called.fetch_add(1, Relaxed);
            Ok(())
        })
        .on_error(&|_, _, _| {
            on_error_called.fetch_add(1, Relaxed);
        })
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(!config.domains_manager().is_frozen_url("http://z1h1.com:1111")?);
        assert!(!config.domains_manager().is_frozen_url("http://z1h2.com:2222")?);

        assert_eq!(mock.call_called(), 1);
        assert_eq!(on_response_called.load(Relaxed), 0);
        assert_eq!(on_error_called.load(Relaxed), 1);
        Ok(())
    }

    #[test]
    fn test_unretryable_error() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRetryer {
            retry_kind: HTTPRetryKind::UnretryableError,
            is_retry_safe: false,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        let (on_response_called, on_error_called) = (AtomicUsize::new(0), AtomicUsize::new(0));
        assert!(Builder::new(
            config.clone(),
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .on_response(&|_, _| {
            on_response_called.fetch_add(1, Relaxed);
            Ok(())
        })
        .on_error(&|_, _, _| {
            on_error_called.fetch_add(1, Relaxed);
        })
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(!config.domains_manager().is_frozen_url("http://z1h1.com:1111")?);
        assert!(!config.domains_manager().is_frozen_url("http://z1h2.com:2222")?);

        assert_eq!(mock.call_called(), 1);
        assert_eq!(on_response_called.load(Relaxed), 0);
        assert_eq!(on_error_called.load(Relaxed), 1);
        Ok(())
    }

    #[test]
    fn test_status_code_571_with_get() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(ErrorResponseMock::new(571, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        assert!(Builder::new(
            config,
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());

        assert_eq!(mock.call_called(), 2);
        Ok(())
    }

    #[test]
    fn test_status_code_571_with_post() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(ErrorResponseMock::new(571, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        assert!(Builder::new(
            config,
            Method::POST,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert_eq!(mock.call_called(), 2);
        Ok(())
    }

    #[test]
    fn test_status_code_504() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(ErrorResponseMock::new(504, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        assert!(Builder::new(
            config,
            Method::POST,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert_eq!(mock.call_called(), 1);
        Ok(())
    }

    #[test]
    fn test_status_code_503() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(ErrorResponseMock::new(503, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        assert!(Builder::new(
            config,
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert_eq!(mock.call_called(), 2);
        Ok(())
    }

    #[test]
    fn test_status_code_631() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(ErrorResponseMock::new(631, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        assert!(Builder::new(
            config,
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert_eq!(mock.call_called(), 1);
        Ok(())
    }

    #[derive(Debug, Clone, Default)]
    struct HTTPActionCounter {
        before: Arc<AtomicUsize>,
        after: Arc<AtomicUsize>,
        destroyer: Option<HTTPRetryer>,
    }

    impl HTTPBeforeAction for HTTPActionCounter {
        fn before_call(&self, request: &mut HTTPRequest) -> HTTPResult<()> {
            self.before.fetch_add(1, Relaxed);
            if let Some(destroyer) = self.destroyer.as_ref() {
                let _ = destroyer.call(request)?;
            }
            Ok(())
        }
    }

    impl HTTPAfterAction for HTTPActionCounter {
        fn after_call(&self, _request: &mut HTTPRequest, _response: &mut HTTPResponse) -> HTTPResult<()> {
            self.after.fetch_add(1, Relaxed);
            Ok(())
        }
    }

    #[test]
    fn test_retryable_error_case_4() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRetryer {
            retry_kind: HTTPRetryKind::RetryableError,
            is_retry_safe: true,
        });
        let counter = HTTPActionCounter::default();
        let config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .append_http_request_before_action_handler(counter.clone())
            .append_http_request_before_action_handler(counter.clone())
            .append_http_request_before_action_handler(counter.clone())
            .append_http_request_after_action_handler(counter.clone())
            .append_http_request_after_action_handler(counter.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        assert!(Builder::new(
            config,
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());

        assert_eq!(mock.call_called(), 2 * (RETRIES + 1));
        assert_eq!(counter.before.load(Relaxed), 3 * 2 * (RETRIES + 1));
        assert_eq!(counter.after.load(Relaxed), 0);
        Ok(())
    }

    #[test]
    fn test_retryable_error_case_5() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRetryer {
            retry_kind: HTTPRetryKind::RetryableError,
            is_retry_safe: true,
        });
        let counter = HTTPActionCounter {
            destroyer: Some(HTTPRetryer {
                retry_kind: HTTPRetryKind::UnretryableError,
                is_retry_safe: false,
            }),
            ..Default::default()
        };
        let config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_handler(mock.clone())
            .append_http_request_before_action_handler(counter.clone())
            .append_http_request_before_action_handler(counter.clone())
            .append_http_request_before_action_handler(counter.clone())
            .append_http_request_after_action_handler(counter.clone())
            .append_http_request_after_action_handler(counter.clone())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build();
        assert!(Builder::new(
            config,
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(TokenVersion::V2, get_credential().into())
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());

        assert_eq!(mock.call_called(), 0);
        assert_eq!(counter.before.load(Relaxed), 1);
        assert_eq!(counter.after.load(Relaxed), 0);
        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
