use super::{
    super::{ResponseError, ResponseErrorKind},
    RequestRetrier, RetryResult,
};
use qiniu_http::{Request as HTTPRequest, ResponseErrorKind as HTTPResponseErrorKind};
use std::any::Any;

#[derive(Copy, Clone, Debug)]
pub struct DefaultRetrier {
    retries: usize,
}

impl Default for DefaultRetrier {
    fn default() -> Self {
        Self { retries: 5 }
    }
}

impl DefaultRetrier {
    fn _retry(&self, response_error: &ResponseError) -> RetryResult {
        match response_error.kind() {
            ResponseErrorKind::HTTPError(http_err_kind) => match http_err_kind {
                HTTPResponseErrorKind::ProtocolError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::InvalidURLError => RetryResult::DontRetry,
                HTTPResponseErrorKind::ConnectError => RetryResult::TryNextServer,
                HTTPResponseErrorKind::ProxyError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::UnknownHostError => RetryResult::TryNextServer,
                HTTPResponseErrorKind::SendError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::ReceiveError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::LocalIOError => RetryResult::DontRetry,
                HTTPResponseErrorKind::TimeoutError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::SSLError => RetryResult::TryNextServer,
                HTTPResponseErrorKind::TooManyRedirect => RetryResult::DontRetry,
                HTTPResponseErrorKind::UnknownError => RetryResult::RetryRequest,
                HTTPResponseErrorKind::UserCanceled => RetryResult::DontRetry,
            },
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
                _ => RetryResult::TryNextServer,
            },
            ResponseErrorKind::ParseResponseError => RetryResult::TryNextServer,
            ResponseErrorKind::MaliciousResponse => RetryResult::RetryRequest,
        }
    }
}

impl RequestRetrier for DefaultRetrier {
    #[inline]
    fn retry(
        &self,
        _request: &mut HTTPRequest,
        response_error: &ResponseError,
        retried: usize,
    ) -> RetryResult {
        let mut result = self._retry(response_error);
        if result == RetryResult::RetryRequest && retried >= self.retries {
            result = RetryResult::TryNextServer
        }
        result
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

#[derive(Default, Copy, Clone, Debug)]
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
