use super::{
    super::{
        error::{Error as QiniuError, ErrorKind as QiniuErrorKind},
        DomainsManager,
    },
    Parts,
};
use qiniu_http::{
    Error as HTTPError, ErrorKind as HTTPErrorKind, Method, Request as HTTPRequest, RequestBuilder,
    Response as HTTPResponse, Result as HTTPResult, StatusCode,
};
use std::{fmt, thread, time::Duration};
use url::Url;

#[derive(Clone)]
pub struct Request {
    pub(super) parts: Parts,
    pub(super) domains_manager: DomainsManager,
    pub(super) host_freeze_duration: Duration,
}

impl Request {
    pub fn send(&self) -> HTTPResult<HTTPResponse> {
        let mut prev_err: Option<HTTPError> = None;
        for host in self.parts.hosts.iter() {
            match self.try_host(host) {
                Ok(response) => {
                    return Ok(response);
                }
                Err(err) => match err.kind() {
                    HTTPErrorKind::RetryableError | HTTPErrorKind::HostUnretryableError if self.is_idempotent(&err) => {
                        self.domains_manager.freeze(host, self.host_freeze_duration).unwrap();
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

    fn try_host(&self, host: &String) -> HTTPResult<HTTPResponse> {
        let url = Url::parse_with_params(&(host.to_owned() + &self.parts.path), &self.parts.query).map_err(|err| {
            HTTPError::new_unretryable_error_from_parts(
                err,
                Some(self.parts.method),
                Some(host.to_owned() + &self.parts.path),
            )
        })?;
        let mut request = RequestBuilder::default()
            .method(self.parts.method)
            .url(url.into_string())
            .headers(self.parts.headers.to_owned())
            .body(&self.parts.body)
            .build();
        self.parts.token.sign(&mut request, &self.parts.auth);
        self.parts.config.http_request_call().on_request_built(&mut request);
        let mut prev_err: Option<HTTPError> = None;
        let retries = *self.parts.config.http_request_retries();
        for retried in 0..=retries {
            match self
                .parts
                .config
                .http_request_call()
                .call(&request)
                .and_then(|response| Self::check_response(response, &request))
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

    fn is_idempotent(&self, err: &HTTPError) -> bool {
        match self.parts.method {
            Method::GET | Method::PUT | Method::HEAD | Method::PATCH | Method::DELETE => true,
            _ => *err.is_retry_safe(),
        }
    }

    fn check_response(mut response: HTTPResponse, request: &HTTPRequest) -> HTTPResult<HTTPResponse> {
        let status_code = response.status_code();
        if (200..300).contains(&status_code) {
            return Ok(response);
        }
        let mut error_message: Option<String> = None;
        if let Some(body_reader) = response.body_mut() {
            let mut body = String::new();
            if let Err(err) = body_reader.read_to_string(&mut body) {
                return Err(HTTPError::new_retryable_error(err, false, request, Some(&response)));
            } else {
                match serde_json::from_str::<error::ErrorResponse>(&body) {
                    Ok(response) => {
                        if response.error.is_some() {
                            error_message = response.error;
                        }
                    }
                    Err(err) => {
                        return Err(HTTPError::new_retryable_error(err, false, request, Some(&response)));
                    }
                }
            }
        }
        Err(Self::response_error(
            response.status_code(),
            error_message.unwrap_or_else(|| "(None)".to_string()),
            request,
            Some(&response),
        ))
    }

    fn response_error(
        status_code: StatusCode,
        error_message: String,
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

impl fmt::Debug for Request {
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
    use qiniu_http::{Error, ErrorKind, HTTPCaller, Headers as HTTPHeaders};
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
        fn call(&self, request: &HTTPRequest) -> HTTPResult<HTTPResponse> {
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
        fn on_retry_request(&self, _request: &HTTPRequest, _error: &Error, _retried: usize, _retries: usize) {
            self.on_retry_request_counter
                .set(self.on_retry_request_counter.get() + 1);
        }
        fn on_host_failed(&self, _failed_host: &str, _error: &HTTPError) {
            self.on_host_failed_counter.set(self.on_host_failed_counter.get() + 1);
        }
        fn on_request_built(&self, _request: &mut HTTPRequest) {
            self.on_request_built_counter
                .set(self.on_request_built_counter.get() + 1);
        }
        fn on_response(&self, _request: &HTTPRequest, _response: &HTTPResponse) {
            self.on_response_counter.set(self.on_response_counter.get() + 1);
        }
        fn on_error(&self, _err: &HTTPError) {
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
        .send()
        .is_err());
        assert_eq!(Arc::try_unwrap(call_counter).unwrap().get(), 1);
        assert_eq!(Arc::try_unwrap(on_retry_request_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_host_failed_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_request_built_counter).unwrap().get(), 1);
        assert_eq!(Arc::try_unwrap(on_response_counter).unwrap().get(), 0);
        assert_eq!(Arc::try_unwrap(on_error_counter).unwrap().get(), 1);
    }

    struct HTTPRequestWithStatusCodeCounter {
        call_counter: Arc<Cell<usize>>,
        on_retry_request_counter: Arc<Cell<usize>>,
        on_host_failed_counter: Arc<Cell<usize>>,
        on_request_built_counter: Arc<Cell<usize>>,
        on_response_counter: Arc<Cell<usize>>,
        on_error_counter: Arc<Cell<usize>>,
        status_code: StatusCode,
        error_message: String,
    }

    impl HTTPCaller for HTTPRequestWithStatusCodeCounter {
        fn call(&self, request: &HTTPRequest) -> HTTPResult<HTTPResponse> {
            assert!(request.headers().contains_key("Authorization"));
            self.call_counter.set(self.call_counter.get() + 1);

            let body = serde_json::to_string(&error::ErrorResponse {
                error: Some(self.error_message.to_owned()),
            })
            .unwrap();
            Ok(HTTPResponse::new(
                self.status_code,
                HTTPHeaders::new(),
                Some(Box::new(io::Cursor::new(body))),
            ))
        }
        fn on_retry_request(&self, _request: &HTTPRequest, _error: &Error, _retried: usize, _retries: usize) {
            self.on_retry_request_counter
                .set(self.on_retry_request_counter.get() + 1);
        }
        fn on_host_failed(&self, _failed_host: &str, _error: &HTTPError) {
            self.on_host_failed_counter.set(self.on_host_failed_counter.get() + 1);
        }
        fn on_request_built(&self, _request: &mut HTTPRequest) {
            self.on_request_built_counter
                .set(self.on_request_built_counter.get() + 1);
        }
        fn on_response(&self, _request: &HTTPRequest, _response: &HTTPResponse) {
            self.on_response_counter.set(self.on_response_counter.get() + 1);
        }
        fn on_error(&self, _err: &HTTPError) {
            self.on_error_counter.set(self.on_error_counter.get() + 1);
        }
    }

    #[test]
    fn test_status_code_571_with_get() {
        let call_counter = Arc::new(Cell::new(0));
        let on_retry_request_counter = Arc::new(Cell::new(0));
        let on_host_failed_counter = Arc::new(Cell::new(0));
        let on_request_built_counter = Arc::new(Cell::new(0));
        let on_response_counter = Arc::new(Cell::new(0));
        let on_error_counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestWithStatusCodeCounter {
                call_counter: call_counter.clone(),
                on_retry_request_counter: on_retry_request_counter.clone(),
                on_host_failed_counter: on_host_failed_counter.clone(),
                on_request_built_counter: on_request_built_counter.clone(),
                on_response_counter: on_response_counter.clone(),
                on_error_counter: on_error_counter.clone(),
                status_code: 571,
                error_message: "Test Error".to_string(),
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
    fn test_status_code_571_with_post() {
        let call_counter = Arc::new(Cell::new(0));
        let on_retry_request_counter = Arc::new(Cell::new(0));
        let on_host_failed_counter = Arc::new(Cell::new(0));
        let on_request_built_counter = Arc::new(Cell::new(0));
        let on_response_counter = Arc::new(Cell::new(0));
        let on_error_counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestWithStatusCodeCounter {
                call_counter: call_counter.clone(),
                on_retry_request_counter: on_retry_request_counter.clone(),
                on_host_failed_counter: on_host_failed_counter.clone(),
                on_request_built_counter: on_request_built_counter.clone(),
                on_response_counter: on_response_counter.clone(),
                on_error_counter: on_error_counter.clone(),
                status_code: 571,
                error_message: "Test Error".to_string(),
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
    fn test_status_code_504() {
        let call_counter = Arc::new(Cell::new(0));
        let on_retry_request_counter = Arc::new(Cell::new(0));
        let on_host_failed_counter = Arc::new(Cell::new(0));
        let on_request_built_counter = Arc::new(Cell::new(0));
        let on_response_counter = Arc::new(Cell::new(0));
        let on_error_counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestWithStatusCodeCounter {
                call_counter: call_counter.clone(),
                on_retry_request_counter: on_retry_request_counter.clone(),
                on_host_failed_counter: on_host_failed_counter.clone(),
                on_request_built_counter: on_request_built_counter.clone(),
                on_response_counter: on_response_counter.clone(),
                on_error_counter: on_error_counter.clone(),
                status_code: 504,
                error_message: "Test Error".to_string(),
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
    fn test_status_code_503() {
        let call_counter = Arc::new(Cell::new(0));
        let on_retry_request_counter = Arc::new(Cell::new(0));
        let on_host_failed_counter = Arc::new(Cell::new(0));
        let on_request_built_counter = Arc::new(Cell::new(0));
        let on_response_counter = Arc::new(Cell::new(0));
        let on_error_counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestWithStatusCodeCounter {
                call_counter: call_counter.clone(),
                on_retry_request_counter: on_retry_request_counter.clone(),
                on_host_failed_counter: on_host_failed_counter.clone(),
                on_request_built_counter: on_request_built_counter.clone(),
                on_response_counter: on_response_counter.clone(),
                on_error_counter: on_error_counter.clone(),
                status_code: 503,
                error_message: "Test Error".to_string(),
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
    fn test_status_code_631() {
        let call_counter = Arc::new(Cell::new(0));
        let on_retry_request_counter = Arc::new(Cell::new(0));
        let on_host_failed_counter = Arc::new(Cell::new(0));
        let on_request_built_counter = Arc::new(Cell::new(0));
        let on_response_counter = Arc::new(Cell::new(0));
        let on_error_counter = Arc::new(Cell::new(0));
        let config: Config = ConfigBuilder::default()
            .http_request_retries(RETRIES)
            .http_request_retry_delay(Duration::from_millis(1))
            .http_request_call(Box::new(HTTPRequestWithStatusCodeCounter {
                call_counter: call_counter.clone(),
                on_retry_request_counter: on_retry_request_counter.clone(),
                on_host_failed_counter: on_host_failed_counter.clone(),
                on_request_built_counter: on_request_built_counter.clone(),
                on_response_counter: on_response_counter.clone(),
                on_error_counter: on_error_counter.clone(),
                status_code: 631,
                error_message: "Test Error".to_string(),
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
