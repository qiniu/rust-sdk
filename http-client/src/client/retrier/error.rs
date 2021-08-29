use super::{
    super::{Idempotent, ResponseError, ResponseErrorKind, RetriedStatsInfo},
    RequestRetrier, RetryResult,
};
use qiniu_http::{
    Method as HTTPMethod, Request as HTTPRequest, ResponseErrorKind as HTTPResponseErrorKind,
};
use std::any::Any;

#[derive(Copy, Clone, Debug, Default)]
pub struct ErrorRetrier;

impl RequestRetrier for ErrorRetrier {
    #[inline]
    fn retry(
        &self,
        request: &mut HTTPRequest,
        idempotent: Idempotent,
        response_error: &ResponseError,
        _retried: &RetriedStatsInfo,
    ) -> RetryResult {
        return match response_error.kind() {
            ResponseErrorKind::HTTPError(http_err_kind) => match http_err_kind {
                HTTPResponseErrorKind::ProtocolError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::InvalidURL => RetryResult::TryNextServer,
                HTTPResponseErrorKind::ConnectError => RetryResult::TryNextServer,
                HTTPResponseErrorKind::ProxyError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::DNSServerError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::UnknownHostError => RetryResult::TryNextServer,
                HTTPResponseErrorKind::SendError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::ReceiveError | HTTPResponseErrorKind::UnknownError => {
                    if is_idempotent(request, idempotent) {
                        RetryResult::RetryRequest
                    } else {
                        RetryResult::DontRetry
                    }
                }
                HTTPResponseErrorKind::LocalIOError => RetryResult::DontRetry,
                HTTPResponseErrorKind::TimeoutError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::SSLError => RetryResult::TryOldEndpoints,
                HTTPResponseErrorKind::TooManyRedirect => RetryResult::DontRetry,
                HTTPResponseErrorKind::UserCanceled => RetryResult::DontRetry,
                _ => RetryResult::RetryRequest,
            },
            ResponseErrorKind::UnexpectedStatusCode(_) => RetryResult::DontRetry,
            ResponseErrorKind::StatusCodeError(status_code) => match status_code.as_u16() {
                0..=399 => panic!("Should not arrive here"),
                400..=501
                | 579
                | 599
                | 608
                | 612
                | 614
                | 616
                | 618
                | 630
                | 631
                | 632
                | 640
                | 701 => RetryResult::DontRetry,
                509 | 573 => RetryResult::Throttled,
                _ => RetryResult::TryNextServer,
            },
            ResponseErrorKind::ParseResponseError | ResponseErrorKind::UnexpectedEof => {
                if is_idempotent(request, idempotent) {
                    RetryResult::RetryRequest
                } else {
                    RetryResult::DontRetry
                }
            }
            ResponseErrorKind::MaliciousResponse => RetryResult::RetryRequest,
        };

        #[inline]
        fn is_idempotent(request: &HTTPRequest, idempotent: Idempotent) -> bool {
            match idempotent {
                Idempotent::Always => true,
                Idempotent::Default => request.method() != HTTPMethod::POST,
                Idempotent::Never => false,
            }
        }
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_request_retrier(&self) -> &dyn RequestRetrier {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{error::Error, result::Result};

    #[test]
    fn test_default_retrier_idempotent() -> Result<(), Box<dyn Error>> {
        let retrier = DefaultRetrierBuilder::default().retries(2).build();
        let result = retrier.retry(
            &mut HTTPRequest::builder()
                .url("http://localhost/abc")
                .method(HTTPMethod::GET)
                .build(),
            Idempotent::Default,
            &ResponseError::new(HTTPResponseErrorKind::ReceiveError.into(), "Test Error"),
            &RetriedStatsInfo::default(),
        );
        assert_eq!(result, RetryResult::RetryRequest);

        let result = retrier.retry(
            &mut HTTPRequest::builder()
                .url("http://localhost/abc")
                .method(HTTPMethod::GET)
                .build(),
            Idempotent::Never,
            &ResponseError::new(HTTPResponseErrorKind::ReceiveError.into(), "Test Error"),
            &RetriedStatsInfo::default(),
        );
        assert_eq!(result, RetryResult::DontRetry);

        let result = retrier.retry(
            &mut HTTPRequest::builder()
                .url("http://localhost/abc")
                .method(HTTPMethod::POST)
                .build(),
            Idempotent::Default,
            &ResponseError::new(HTTPResponseErrorKind::ReceiveError.into(), "Test Error"),
            &RetriedStatsInfo::default(),
        );
        assert_eq!(result, RetryResult::DontRetry);

        let result = retrier.retry(
            &mut HTTPRequest::builder()
                .url("http://localhost/abc")
                .method(HTTPMethod::POST)
                .build(),
            Idempotent::Always,
            &ResponseError::new(HTTPResponseErrorKind::ReceiveError.into(), "Test Error"),
            &RetriedStatsInfo::default(),
        );
        assert_eq!(result, RetryResult::RetryRequest);

        let result = retrier.retry(
            &mut HTTPRequest::builder()
                .url("http://localhost/abc")
                .method(HTTPMethod::POST)
                .build(),
            Idempotent::Always,
            &ResponseError::new(HTTPResponseErrorKind::InvalidURLError.into(), "Test Error"),
            &RetriedStatsInfo::default(),
        );
        assert_eq!(result, RetryResult::TryNextServer);

        Ok(())
    }

    #[test]
    fn test_default_retrier_retries() -> Result<(), Box<dyn Error>> {
        let retrier = DefaultRetrierBuilder::default().retries(2).build();
        let mut retried = RetriedStatsInfo::default();
        retried.increase();
        retried.increase();

        let result = retrier.retry(
            &mut HTTPRequest::builder()
                .url("http://localhost/abc")
                .method(HTTPMethod::GET)
                .build(),
            Idempotent::Default,
            &ResponseError::new(HTTPResponseErrorKind::ReceiveError.into(), "Test Error"),
            &retried,
        );
        assert_eq!(result, RetryResult::TryNextServer);

        retried.switch_endpoint();

        let result = retrier.retry(
            &mut HTTPRequest::builder()
                .url("http://localhost/abc")
                .method(HTTPMethod::GET)
                .build(),
            Idempotent::Default,
            &ResponseError::new(HTTPResponseErrorKind::ReceiveError.into(), "Test Error"),
            &retried,
        );
        assert_eq!(result, RetryResult::RetryRequest);

        Ok(())
    }
}
