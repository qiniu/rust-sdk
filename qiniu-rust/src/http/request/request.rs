use super::{
    super::{
        error::{Error as QiniuError, ErrorKind as QiniuErrorKind},
        response::Response,
        DomainsManager,
    },
    Parts,
};
use qiniu_http::{
    Error as HTTPError, ErrorKind as HTTPErrorKind, Method, Request as HTTPRequest, RequestBuilder,
    Response as HTTPResponse, Result as HTTPResult, StatusCode,
};
use std::{
    fmt,
    io::{self, Cursor},
    thread,
    time::Duration,
};
use url::Url;

#[derive(Clone)]
pub struct Request<'a> {
    pub(super) parts: Parts<'a>,
    pub(super) domains_manager: DomainsManager,
    pub(super) host_freeze_duration: Duration,
}

impl<'a> Request<'a> {
    pub fn send(&self) -> HTTPResult<Response> {
        let mut prev_err: Option<HTTPError> = None;
        for &host in self.parts.hosts {
            if self.domains_manager.is_frozen(host).map_err(|err| {
                HTTPError::new_host_unretryable_error_from_parts(err, true, Some(self.parts.method.to_owned()), None)
            })? {
                continue;
            }
            match self.try_host(host) {
                Ok(resp) => {
                    return Ok(Response {
                        inner: resp,
                        method: self.parts.method,
                        host: host,
                        path: self.parts.path,
                    });
                }
                Err(err) => match err.kind() {
                    HTTPErrorKind::RetryableError | HTTPErrorKind::HostUnretryableError if self.is_idempotent(&err) => {
                        self.domains_manager
                            .freeze(host.to_string(), self.host_freeze_duration)
                            .unwrap();
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
            HTTPError::new_host_unretryable_error_from_parts(
                error::Error::from(error::ErrorKind::NoHostAvailable),
                true,
                Some(self.parts.method.to_owned()),
                None,
            )
        });
        self.parts.config.http_request_call().on_error(&err);
        Err(err)
    }

    fn try_host(&self, host: &str) -> HTTPResult<HTTPResponse> {
        let mut url = host.to_string() + self.parts.path;
        if let Some(ref query) = self.parts.query {
            url = Url::parse_with_params(url.as_str(), query)
                .map_err(|err| {
                    HTTPError::new_unretryable_error_from_parts(
                        err,
                        Some(self.parts.method),
                        Some((host.to_owned() + &self.parts.path).into()),
                    )
                })?
                .into_string();
        }
        let mut request = {
            let mut builder = RequestBuilder::default().method(self.parts.method).url(url);
            if let Some(headers) = &self.parts.headers {
                builder = builder.headers(headers.to_owned());
            }
            if let Some(body) = &self.parts.body {
                builder = builder.body(body.as_slice());
            }
            builder.build()
        };
        self.parts.token.sign(&mut request, &self.parts.auth);
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
            {
                Ok(response) => {
                    self.parts.config.http_request_call().on_response(&request, &response);
                    return Ok(response);
                }
                Err(err) => match err.kind() {
                    HTTPErrorKind::RetryableError if self.is_idempotent(&err) => {
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

    fn is_idempotent(&self, err: &HTTPError) -> bool {
        match self.parts.method {
            Method::GET | Method::PUT | Method::HEAD | Method::PATCH | Method::DELETE => true,
            _ => err.is_retry_safe(),
        }
    }

    fn check_response(mut response: HTTPResponse, request: &HTTPRequest) -> HTTPResult<HTTPResponse> {
        let status_code = response.status_code();
        if (200..300).contains(&status_code) {
            return Ok(response);
        }
        let mut error_message: Option<Box<str>> = None;
        if let Some(body) = Self::read_body_to_string(&mut response, request)? {
            error_message = serde_json::from_str::<error::ErrorResponse>(&body)
                .map_err(|err| HTTPError::new_retryable_error(err, false, request, Some(&response)))?
                .error
                .map(|e| e.into())
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
                return HTTPError::new_unretryable_error(err, request, Some(response));
            })?);
        }
        if let Some(body_reader) = response.body_mut() {
            let mut buf = String::new();
            let body_len = body_reader
                .read_to_string(&mut buf)
                .map_err(|err| HTTPError::new_retryable_error(err, false, request, Some(response)))?;
            if let Some(content_length) = content_length {
                if content_length != body_len {
                    return Err(HTTPError::new_retryable_error(
                        io::Error::from(io::ErrorKind::UnexpectedEof),
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
            400 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::BadRequestError(status_code, error_message)),
                request,
                response,
            ),
            401 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::UnauthorizedError(status_code, error_message)),
                request,
                response,
            ),
            403 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::ForbiddenError(status_code, error_message)),
                request,
                response,
            ),
            404 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::URLNotFoundError(status_code, error_message)),
                request,
                response,
            ),
            405 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::MethodNotAllowedError(status_code, error_message)),
                request,
                response,
            ),
            406 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::NotAcceptableError(status_code, error_message)),
                request,
                response,
            ),
            409 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::ConflictError(status_code, error_message)),
                request,
                response,
            ),
            419 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::UserDisabledError(status_code, error_message)),
                request,
                response,
            ),
            501 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::NotImplementedError(status_code, error_message)),
                request,
                response,
            ),
            502 => HTTPError::new_host_unretryable_error(
                QiniuError::from(QiniuErrorKind::BadGatewayError(status_code, error_message)),
                true,
                request,
                response,
            ),
            503 => HTTPError::new_host_unretryable_error(
                QiniuError::from(QiniuErrorKind::ServiceUnavailableError(status_code, error_message)),
                true,
                request,
                response,
            ),
            504 => HTTPError::new_retryable_error(
                QiniuError::from(QiniuErrorKind::GatewayTimeoutError(status_code, error_message)),
                false,
                request,
                response,
            ),
            571 => HTTPError::new_retryable_error(
                QiniuError::from(QiniuErrorKind::BusyError(status_code, error_message)),
                true,
                request,
                response,
            ),
            573 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::OutOfLimitError(status_code, error_message)),
                request,
                response,
            ),
            579 => HTTPError::new_retryable_error(
                QiniuError::from(QiniuErrorKind::CallbackError(status_code, error_message)),
                false,
                request,
                response,
            ),
            599 => HTTPError::new_retryable_error(
                QiniuError::from(QiniuErrorKind::InternalServerError(status_code, error_message)),
                false,
                request,
                response,
            ),
            608 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::FileModifiedError(status_code, error_message)),
                request,
                response,
            ),
            612 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::ResourceNotFoundError(status_code, error_message)),
                request,
                response,
            ),
            614 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::ResourceExistsError(status_code, error_message)),
                request,
                response,
            ),
            615 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::RoomIsInactiveError(status_code, error_message)),
                request,
                response,
            ),
            616 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::HubNotMatchError(status_code, error_message)),
                request,
                response,
            ),
            619 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::NoDataError(status_code, error_message)),
                request,
                response,
            ),
            630 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::TooManyBucketsError(status_code, error_message)),
                request,
                response,
            ),
            631 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::BucketNotFoundError(status_code, error_message)),
                request,
                response,
            ),
            640 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::InvalidMarkerError(status_code, error_message)),
                request,
                response,
            ),
            701 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::InvalidContextError(status_code, error_message)),
                request,
                response,
            ),
            400..=499 => HTTPError::new_unretryable_error(
                QiniuError::from(QiniuErrorKind::UnknownClientError(status_code, error_message)),
                request,
                response,
            ),
            _ => HTTPError::new_retryable_error(
                QiniuError::from(QiniuErrorKind::UnknownServerError(status_code, error_message)),
                false,
                request,
                response,
            ),
        }
    }
}

impl fmt::Debug for Request<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Request")
            .field("parts", &self.parts)
            .field("host_freeze_duration", &self.host_freeze_duration)
            .finish()
    }
}

mod error {
    use error_chain::error_chain;
    use serde::{Deserialize, Serialize};

    error_chain! {
        errors {
            NoHostAvailable {
                description("no host is available"),
                display("no host is available"),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub(crate) struct ErrorResponse {
        pub(crate) error: Option<String>,
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
    use qiniu_test_utils::http_call_mock::{CounterCallMock, ErrorResponseMock};
    use std::{io, time::Duration};

    struct HTTPRequestCounter {
        is_retry_safe: bool,
        error_kind: ErrorKind,
    }

    impl HTTPCaller for HTTPRequestCounter {
        fn call(&self, request: &HTTPRequest) -> HTTPResult<HTTPResponse> {
            assert!(request.headers().contains_key("Authorization"));
            Err(Error::new_from_parts(
                self.error_kind.clone(),
                io::Error::new(io::ErrorKind::Other, "Test Error"),
                self.is_retry_safe,
                None,
                None,
            ))
        }
    }

    const RETRIES: usize = 5;

    #[test]
    fn test_retryable_error_case_1() {
        let mock = CounterCallMock::new(HTTPRequestCounter {
            error_kind: ErrorKind::RetryableError,
            is_retry_safe: true,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        assert!(Builder::new(
            get_auth(),
            config.clone(),
            Method::GET,
            "/test_call",
            &["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(config.domains_manager().is_frozen("host1").unwrap());
        assert!(config.domains_manager().is_frozen("host2").unwrap());

        assert_eq!(mock.call_called(), 2 * (RETRIES + 1));
        assert_eq!(mock.on_retry_request_called(), 2 * (RETRIES + 1));
        assert_eq!(mock.on_host_failed_called(), 2);
        assert_eq!(mock.on_request_built_called(), 2);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
    }

    #[test]
    fn test_retryable_error_case_2() {
        let mock = CounterCallMock::new(HTTPRequestCounter {
            error_kind: ErrorKind::RetryableError,
            is_retry_safe: true,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        assert!(Builder::new(
            get_auth(),
            config.clone(),
            Method::POST,
            "/test_call",
            &["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(config.domains_manager().is_frozen("host1").unwrap());
        assert!(config.domains_manager().is_frozen("host2").unwrap());

        assert_eq!(mock.call_called(), 2 * (RETRIES + 1));
        assert_eq!(mock.on_retry_request_called(), 2 * (RETRIES + 1));
        assert_eq!(mock.on_host_failed_called(), 2);
        assert_eq!(mock.on_request_built_called(), 2);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
    }

    #[test]
    fn test_retryable_error_case_3() {
        let mock = CounterCallMock::new(HTTPRequestCounter {
            error_kind: ErrorKind::RetryableError,
            is_retry_safe: false,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        assert!(Builder::new(
            get_auth(),
            config.clone(),
            Method::POST,
            "/test_call",
            &["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(!config.domains_manager().is_frozen("host1").unwrap());
        assert!(!config.domains_manager().is_frozen("host2").unwrap());

        assert_eq!(mock.call_called(), 1);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 0);
        assert_eq!(mock.on_request_built_called(), 1);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
    }

    #[test]
    fn test_host_unretryable_error() {
        let mock = CounterCallMock::new(HTTPRequestCounter {
            error_kind: ErrorKind::HostUnretryableError,
            is_retry_safe: true,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        assert!(Builder::new(
            get_auth(),
            config.clone(),
            Method::GET,
            "/test_call",
            &["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(config.domains_manager().is_frozen("host1").unwrap());
        assert!(config.domains_manager().is_frozen("host2").unwrap());

        assert_eq!(mock.call_called(), 2);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 2);
        assert_eq!(mock.on_request_built_called(), 2);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
    }

    #[test]
    fn test_zone_unretryable_error() {
        let mock = CounterCallMock::new(HTTPRequestCounter {
            error_kind: ErrorKind::ZoneUnretryableError,
            is_retry_safe: false,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        assert!(Builder::new(
            get_auth(),
            config.clone(),
            Method::GET,
            "/test_call",
            &["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(!config.domains_manager().is_frozen("host1").unwrap());
        assert!(!config.domains_manager().is_frozen("host2").unwrap());

        assert_eq!(mock.call_called(), 1);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 0);
        assert_eq!(mock.on_request_built_called(), 1);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
    }

    #[test]
    fn test_unretryable_error() {
        let mock = CounterCallMock::new(HTTPRequestCounter {
            error_kind: ErrorKind::UnretryableError,
            is_retry_safe: false,
        });
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        assert!(Builder::new(
            get_auth(),
            config.clone(),
            Method::GET,
            "/test_call",
            &["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert!(!config.domains_manager().is_frozen("host1").unwrap());
        assert!(!config.domains_manager().is_frozen("host2").unwrap());

        assert_eq!(mock.call_called(), 1);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 0);
        assert_eq!(mock.on_request_built_called(), 1);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
    }

    #[test]
    fn test_status_code_571_with_get() {
        let mock = CounterCallMock::new(ErrorResponseMock::new(571, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        assert!(Builder::new(
            get_auth(),
            config,
            Method::GET,
            "/test_call",
            &["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());

        assert_eq!(mock.call_called(), 2 * (RETRIES + 1));
        assert_eq!(mock.on_retry_request_called(), 2 * (RETRIES + 1));
        assert_eq!(mock.on_host_failed_called(), 2);
        assert_eq!(mock.on_request_built_called(), 2);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
    }

    #[test]
    fn test_status_code_571_with_post() {
        let mock = CounterCallMock::new(ErrorResponseMock::new(571, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        assert!(Builder::new(
            get_auth(),
            config,
            Method::POST,
            "/test_call",
            &["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert_eq!(mock.call_called(), 2 * (RETRIES + 1));
        assert_eq!(mock.on_retry_request_called(), 2 * (RETRIES + 1));
        assert_eq!(mock.on_host_failed_called(), 2);
        assert_eq!(mock.on_request_built_called(), 2);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
    }

    #[test]
    fn test_status_code_504() {
        let mock = CounterCallMock::new(ErrorResponseMock::new(504, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        assert!(Builder::new(
            get_auth(),
            config,
            Method::POST,
            "/test_call",
            &["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert_eq!(mock.call_called(), 1);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 0);
        assert_eq!(mock.on_request_built_called(), 1);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
    }

    #[test]
    fn test_status_code_503() {
        let mock = CounterCallMock::new(ErrorResponseMock::new(503, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        assert!(Builder::new(
            get_auth(),
            config,
            Method::GET,
            "/test_call",
            &["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert_eq!(mock.call_called(), 2);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 2);
        assert_eq!(mock.on_request_built_called(), 2);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
    }

    #[test]
    fn test_status_code_631() {
        let mock = CounterCallMock::new(ErrorResponseMock::new(631, "Test Error"));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        assert!(Builder::new(
            get_auth(),
            config,
            Method::GET,
            "/test_call",
            &["http://host1:1111", "http://host2:2222"],
        )
        .token(Token::V1)
        .raw_body("application/json", b"{\"test\":123}".as_ref())
        .send()
        .is_err());
        assert_eq!(mock.call_called(), 1);
        assert_eq!(mock.on_retry_request_called(), 0);
        assert_eq!(mock.on_host_failed_called(), 0);
        assert_eq!(mock.on_request_built_called(), 1);
        assert_eq!(mock.on_response_called(), 0);
        assert_eq!(mock.on_error_called(), 1);
    }

    fn get_auth() -> Auth {
        Auth::new("abcdefghklmnopq", "1234567890")
    }
}
