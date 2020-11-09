use super::{
    super::{Idempotent, ResponseError, ResponseErrorKind, RetriedStatsInfo},
    RequestRetrier, RetryResult,
};
use qiniu_http::{
    Method as HTTPMethod, Request as HTTPRequest, ResponseErrorKind as HTTPResponseErrorKind,
};
use std::any::Any;

#[derive(Clone, Debug)]
pub struct DefaultRetrier {
    retries: usize,
}

impl Default for DefaultRetrier {
    fn default() -> Self {
        Self { retries: 2 }
    }
}

impl DefaultRetrier {
    #[inline]
    pub fn builder() -> DefaultRetrierBuilder {
        DefaultRetrierBuilder::default()
    }

    fn _retry(
        &self,
        request: &mut HTTPRequest,
        idempotent: Idempotent,
        response_error: &ResponseError,
    ) -> RetryResult {
        match response_error.kind() {
            ResponseErrorKind::HTTPError(http_err_kind) => match http_err_kind {
                HTTPResponseErrorKind::ProtocolError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::InvalidURLError => RetryResult::TryNextServer,
                HTTPResponseErrorKind::ConnectError => RetryResult::TryNextServer,
                HTTPResponseErrorKind::ProxyError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::DNSServerError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::UnknownHostError => RetryResult::TryNextServer,
                HTTPResponseErrorKind::SendError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::ReceiveError | HTTPResponseErrorKind::UnknownError => {
                    if self.is_idempotent(request, idempotent) {
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
            ResponseErrorKind::StatusCodeError(status_code) => match status_code {
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
                if self.is_idempotent(request, idempotent) {
                    RetryResult::RetryRequest
                } else {
                    RetryResult::DontRetry
                }
            }
            ResponseErrorKind::MaliciousResponse => RetryResult::RetryRequest,
        }
    }

    fn is_idempotent(&self, request: &HTTPRequest, idempotent: Idempotent) -> bool {
        match idempotent {
            Idempotent::Always => true,
            Idempotent::Default => request.method() != HTTPMethod::POST,
            Idempotent::Never => false,
        }
    }
}

impl RequestRetrier for DefaultRetrier {
    #[inline]
    fn retry(
        &self,
        request: &mut HTTPRequest,
        idempotent: Idempotent,
        response_error: &ResponseError,
        retried: &RetriedStatsInfo,
    ) -> RetryResult {
        match self._retry(request, idempotent, response_error) {
            RetryResult::RetryRequest | RetryResult::Throttled
                if retried.retried_on_current_endpoint() >= self.retries =>
            {
                RetryResult::TryNextServer
            }
            result => result,
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

#[derive(Default, Clone, Debug)]
pub struct DefaultRetrierBuilder {
    inner: DefaultRetrier,
}

impl DefaultRetrierBuilder {
    #[inline]
    pub fn retries(&mut self, retries: usize) -> &mut Self {
        self.inner.retries = retries;
        self
    }

    #[inline]
    pub fn build(&self) -> DefaultRetrier {
        self.inner.to_owned()
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
