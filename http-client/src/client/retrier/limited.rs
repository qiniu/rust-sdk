use super::{
    super::{Idempotent, ResponseError, RetriedStatsInfo},
    RequestRetrier, RetryResult,
};
use qiniu_http::Request as HTTPRequest;
use std::any::Any;

const DEFAULT_RETIES: usize = 2;

#[derive(Clone, Debug)]
pub struct LimitedRetrier<R: RequestRetrier> {
    retrier: R,
    retries: usize,
}

impl<R: RequestRetrier> LimitedRetrier<R> {
    #[inline]
    pub fn new(retrier: R, retries: usize) -> Self {
        Self { retrier, retries }
    }
}

impl<R: Default + RequestRetrier> Default for LimitedRetrier<R> {
    #[inline]
    fn default() -> Self {
        Self::new(R::default(), DEFAULT_RETIES)
    }
}

impl<R: RequestRetrier> RequestRetrier for LimitedRetrier<R> {
    #[inline]
    fn retry(
        &self,
        request: &mut HTTPRequest,
        idempotent: Idempotent,
        response_error: &ResponseError,
        retried: &RetriedStatsInfo,
    ) -> RetryResult {
        match self
            .retrier
            .retry(request, idempotent, response_error, retried)
        {
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

#[cfg(test)]
mod tests {
    use super::{super::ErrorRetrier, *};
    use qiniu_http::{
        Method as HTTPMethod, ResponseErrorKind as HTTPResponseErrorKind, Uri as HTTPUri,
    };
    use std::{convert::TryFrom, error::Error, result::Result};

    #[test]
    fn test_limited_retrier_retries() -> Result<(), Box<dyn Error>> {
        let uri = HTTPUri::try_from("http://localhost/abc")?;

        let retrier = LimitedRetrier::new(ErrorRetrier, 2);
        let mut retried = RetriedStatsInfo::default();
        retried.increase();
        retried.increase();

        let result = retrier.retry(
            &mut HTTPRequest::builder()
                .url(uri.to_owned())
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
                .url(uri)
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
