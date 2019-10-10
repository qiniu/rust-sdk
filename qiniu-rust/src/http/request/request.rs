use super::{
    super::{response::Response, Choice, DomainsManager},
    ErrorResponse as RequestErrorResponse, Parts,
};
use qiniu_http::{
    Error as HTTPError, ErrorKind as HTTPErrorKind, Method, Request as HTTPRequest, RequestBuilder,
    Response as HTTPResponse, Result as HTTPResult, RetryKind as HTTPRetryKind, StatusCode,
};
use std::{
    borrow::Cow,
    fmt,
    io::{self, Cursor},
    thread,
};
use url::Url;

pub(crate) struct Request<'a> {
    pub(super) parts: Parts<'a>,
    pub(super) domains_manager: DomainsManager,
}

impl<'a> Request<'a> {
    pub(crate) fn send(&self) -> HTTPResult<Response> {
        let mut prev_err: Option<HTTPError> = None;
        let choices = self.domains_manager.choose(self.parts.hosts).map_err(|err| {
            HTTPError::new_host_unretryable_error_from_parts(
                HTTPErrorKind::UnknownError(Box::new(err)),
                true,
                Some(self.parts.method.to_owned()),
                None,
            )
        })?;
        for choice in choices {
            let url = choice.url;
            match self.try_choice(choice) {
                Ok(resp) => {
                    return Ok(resp);
                }
                Err(err) => match err.retry_kind() {
                    HTTPRetryKind::RetryableError | HTTPRetryKind::HostUnretryableError if self.is_retry_safe(&err) => {
                        self.domains_manager.freeze_url(url).unwrap();
                        self.parts.config.http_request_call().on_host_failed(url, &err);
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
        let err = prev_err.unwrap();
        self.parts.config.http_request_call().on_error(&err);
        Err(err)
    }

    fn try_choice(&self, choice: Choice<'a>) -> HTTPResult<Response<'a>> {
        let mut url = choice.url.to_owned() + self.parts.path;
        if let Some(query) = &self.parts.query {
            url = Url::parse_with_params(url.as_str(), query)
                .map_err(|err| {
                    HTTPError::new_unretryable_error_from_parts(
                        HTTPErrorKind::UnknownError(Box::new(err)),
                        Some(self.parts.method),
                        Some((choice.url.to_owned() + &self.parts.path).into()),
                    )
                })?
                .into_string();
        }
        let mut request = {
            let mut builder = RequestBuilder::default()
                .method(self.parts.method)
                .url(url)
                .follow_redirection(self.parts.follow_redirection);
            if !choice.socket_addrs.is_empty() {
                builder = builder.resolved_socket_addrs(&choice.socket_addrs);
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
        self.parts.token.sign(&mut request);
        self.parts.config.http_request_call().on_request_built(&mut request);
        let mut prev_err: Option<HTTPError> = None;
        let retries = self.parts.config.http_request_retries();
        for retried in 0..=retries {
            match self
                .parts
                .config
                .http_request_call()
                .call(&request)
                .and_then(|response| Self::check_response(response, &request))
                .and_then(|response| self.fulfill_body_if_needed(response, &request))
                .map(|response| Response {
                    inner: response,
                    method: self.parts.method,
                    host: choice.url,
                    path: self.parts.path,
                }) {
                Ok(mut response) => {
                    if let Some(callback) = self.parts.response_callback {
                        callback.on_response_callback(&mut response, &request)?;
                    }
                    self.parts
                        .config
                        .http_request_call()
                        .on_response(&request, &response.inner);
                    return Ok(response);
                }
                Err(err) => match err.retry_kind() {
                    HTTPRetryKind::RetryableError if self.is_retry_safe(&err) => {
                        self.parts
                            .config
                            .http_request_call()
                            .on_retry_request(&request, &err, retried, retries);
                        prev_err = Some(err);
                        if self.parts.config.http_request_retry_delay().as_nanos() > 0 {
                            // TODO: Think about async sleep
                            thread::sleep(self.parts.config.http_request_retry_delay());
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

    fn is_retry_safe(&self, err: &HTTPError) -> bool {
        match self.parts.method {
            Method::GET | Method::PUT | Method::HEAD | Method::PATCH | Method::DELETE => true,
            _ => self.parts.idempotent || err.is_retry_safe(),
        }
    }

    fn check_response(mut response: HTTPResponse, request: &HTTPRequest) -> HTTPResult<HTTPResponse> {
        let status_code = response.status_code();
        if (200..300).contains(&status_code) {
            return Ok(response);
        }
        let mut error_message: Option<Box<str>> = None;
        if let Some(body) = Self::read_body_to_string(&mut response, request)? {
            if response.header("Content-Type") == Some(&Cow::Borrowed("application/json")) {
                error_message = serde_json::from_str::<RequestErrorResponse>(&body)
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
        if let Some(body) = Self::read_body_to_string(&mut response, request)? {
            *response.body_mut() = Some(Box::new(Cursor::new(body)));
        }
        Ok(response)
    }

    fn read_body_to_string(response: &mut HTTPResponse, request: &HTTPRequest) -> HTTPResult<Option<String>> {
        let mut content_length = None::<usize>;
        if let Some(content_length_str) = response.header("Content-Length") {
            content_length = Some(content_length_str.parse().map_err(|err| {
                return HTTPError::new_unretryable_error(
                    HTTPErrorKind::UnknownError(Box::new(err)),
                    request,
                    Some(response),
                );
            })?);
        }
        if let Some(body_reader) = response.body_mut() {
            let mut buf = String::new();
            let body_len = body_reader.read_to_string(&mut buf).map_err(|err| {
                HTTPError::new_retryable_error(HTTPErrorKind::IOError(err), false, request, Some(response))
            })?;
            if let Some(content_length) = content_length {
                if content_length != body_len {
                    return Err(HTTPError::new_retryable_error(
                        HTTPErrorKind::IOError(io::Error::from(io::ErrorKind::UnexpectedEof)),
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
                super::{
                    config::{Config, ConfigBuilder},
                    credential::Credential,
                },
                token::Token,
                DomainsManagerBuilder,
            },
            Builder,
        },
        *,
    };
    use qiniu_http::HTTPCaller;
    use qiniu_test_utils::http_call_mock::{CounterCallMock, ErrorResponseMock};
    use std::{boxed::Box, error::Error as StdError, io, result::Result as StdResult, time::Duration};

    struct HTTPRequestCounter {
        is_retry_safe: bool,
        retry_kind: HTTPRetryKind,
    }

    impl HTTPCaller for HTTPRequestCounter {
        fn call(&self, request: &HTTPRequest) -> HTTPResult<HTTPResponse> {
            assert!(request.headers().contains_key("Authorization"));
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
        let mock = CounterCallMock::new(HTTPRequestCounter {
            retry_kind: HTTPRetryKind::RetryableError,
            is_retry_safe: true,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        assert!(Builder::new(
            config.clone(),
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(Token::V1(get_credential()))
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(config.domains_manager().is_frozen_url("http://z1h1.com:1111")?);
        assert!(config.domains_manager().is_frozen_url("http://z1h2.com:2222")?);

        assert_eq!(mock.call_called(), 2 * (RETRIES + 1));
        assert_eq!(mock.on_retry_request_called(), 2 * (RETRIES + 1));
        assert_eq!(mock.on_host_failed_called(), 2);
        assert_eq!(mock.on_request_built_called(), 2);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
        Ok(())
    }

    #[test]
    fn test_retryable_error_case_2() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRequestCounter {
            retry_kind: HTTPRetryKind::RetryableError,
            is_retry_safe: true,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        assert!(Builder::new(
            config.clone(),
            Method::POST,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(Token::V1(get_credential()))
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(config.domains_manager().is_frozen_url("http://z1h1.com:1111")?);
        assert!(config.domains_manager().is_frozen_url("http://z1h2.com:2222")?);

        assert_eq!(mock.call_called(), 2 * (RETRIES + 1));
        assert_eq!(mock.on_retry_request_called(), 2 * (RETRIES + 1));
        assert_eq!(mock.on_host_failed_called(), 2);
        assert_eq!(mock.on_request_built_called(), 2);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
        Ok(())
    }

    #[test]
    fn test_retryable_error_case_3() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRequestCounter {
            retry_kind: HTTPRetryKind::RetryableError,
            is_retry_safe: false,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        assert!(Builder::new(
            config.clone(),
            Method::POST,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(Token::V1(get_credential()))
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(!config.domains_manager().is_frozen_url("http://z1h1.com:1111")?);
        assert!(!config.domains_manager().is_frozen_url("http://z1h2.com:2222")?);

        assert_eq!(mock.call_called(), 1);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 0);
        assert_eq!(mock.on_request_built_called(), 1);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
        Ok(())
    }

    #[test]
    fn test_host_unretryable_error() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRequestCounter {
            retry_kind: HTTPRetryKind::HostUnretryableError,
            is_retry_safe: true,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        assert!(Builder::new(
            config.clone(),
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(Token::V1(get_credential()))
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(config.domains_manager().is_frozen_url("http://z1h1.com:1111")?);
        assert!(config.domains_manager().is_frozen_url("http://z1h2.com:2222")?);

        assert_eq!(mock.call_called(), 2);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 2);
        assert_eq!(mock.on_request_built_called(), 2);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
        Ok(())
    }

    #[test]
    fn test_zone_unretryable_error() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRequestCounter {
            retry_kind: HTTPRetryKind::ZoneUnretryableError,
            is_retry_safe: false,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        assert!(Builder::new(
            config.clone(),
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(Token::V1(get_credential()))
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(!config.domains_manager().is_frozen_url("http://z1h1.com:1111")?);
        assert!(!config.domains_manager().is_frozen_url("http://z1h2.com:2222")?);

        assert_eq!(mock.call_called(), 1);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 0);
        assert_eq!(mock.on_request_built_called(), 1);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
        Ok(())
    }

    #[test]
    fn test_unretryable_error() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(HTTPRequestCounter {
            retry_kind: HTTPRetryKind::UnretryableError,
            is_retry_safe: false,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        assert!(Builder::new(
            config.clone(),
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(Token::V1(get_credential()))
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(!config.domains_manager().is_frozen_url("http://z1h1.com:1111")?);
        assert!(!config.domains_manager().is_frozen_url("http://z1h2.com:2222")?);

        assert_eq!(mock.call_called(), 1);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 0);
        assert_eq!(mock.on_request_built_called(), 1);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
        Ok(())
    }

    #[test]
    fn test_status_code_571_with_get() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(ErrorResponseMock::new(571, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        assert!(Builder::new(
            config,
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(Token::V1(get_credential()))
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());

        assert_eq!(mock.call_called(), 2);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 2);
        assert_eq!(mock.on_request_built_called(), 2);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
        Ok(())
    }

    #[test]
    fn test_status_code_571_with_post() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(ErrorResponseMock::new(571, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        assert!(Builder::new(
            config,
            Method::POST,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(Token::V1(get_credential()))
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert_eq!(mock.call_called(), 2);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 2);
        assert_eq!(mock.on_request_built_called(), 2);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
        Ok(())
    }

    #[test]
    fn test_status_code_504() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(ErrorResponseMock::new(504, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        assert!(Builder::new(
            config,
            Method::POST,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(Token::V1(get_credential()))
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert_eq!(mock.call_called(), 1);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 0);
        assert_eq!(mock.on_request_built_called(), 1);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
        Ok(())
    }

    #[test]
    fn test_status_code_503() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(ErrorResponseMock::new(503, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        assert!(Builder::new(
            config,
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(Token::V1(get_credential()))
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert_eq!(mock.call_called(), 2);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 2);
        assert_eq!(mock.on_request_built_called(), 2);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
        Ok(())
    }

    #[test]
    fn test_status_code_631() -> StdResult<(), Box<dyn StdError>> {
        let mock = CounterCallMock::new(ErrorResponseMock::new(631, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .build()?;
        assert!(Builder::new(
            config,
            Method::GET,
            "/test_call",
            &["http://z1h1.com:1111", "http://z1h2.com:2222"],
        )
        .token(Token::V1(get_credential()))
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert_eq!(mock.call_called(), 1);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 0);
        assert_eq!(mock.on_request_built_called(), 1);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
