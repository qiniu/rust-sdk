use super::{RequestRetrier, RequestRetrierOptions, RetryDecision, RetryResult};
use qiniu_http::RequestParts as HttpRequestParts;

const DEFAULT_RETIES: usize = 2;

#[derive(Clone, Debug)]
pub struct LimitedRetrier<R> {
    retrier: R,
    retries: usize,
}

impl<R> LimitedRetrier<R> {
    #[inline]
    pub fn new(retrier: R, retries: usize) -> Self {
        Self { retrier, retries }
    }
}

impl<R: Default> Default for LimitedRetrier<R> {
    #[inline]
    fn default() -> Self {
        Self::new(R::default(), DEFAULT_RETIES)
    }
}

impl<R: RequestRetrier> RequestRetrier for LimitedRetrier<R> {
    #[inline]
    fn retry(&self, request: &mut HttpRequestParts, opts: &RequestRetrierOptions) -> RetryResult {
        match self.retrier.retry(request, opts).decision() {
            RetryDecision::RetryRequest | RetryDecision::Throttled
                if opts.retried().retried_on_current_endpoint() >= self.retries =>
            {
                RetryDecision::TryNextServer
            }
            result => result,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{ErrorRetrier, Idempotent, ResponseError, RetriedStatsInfo},
        *,
    };
    use qiniu_http::{
        Method as HttpMethod, Request as HttpRequest, ResponseErrorKind as HttpResponseErrorKind,
        Uri as HttpUri,
    };
    use std::{convert::TryFrom, error::Error, result::Result};

    #[test]
    fn test_limited_retrier_retries() -> Result<(), Box<dyn Error>> {
        let uri = HttpUri::try_from("http://localhost/abc")?;

        let retrier = LimitedRetrier::new(ErrorRetrier, 2);
        let mut retried = RetriedStatsInfo::default();
        retried.increase();
        retried.increase();

        let (mut parts, _) = HttpRequest::builder()
            .url(uri)
            .method(HttpMethod::GET)
            .body(())
            .build()
            .into_parts();
        let result = retrier.retry(
            &mut parts,
            &RequestRetrierOptions::new(
                Idempotent::Default,
                &ResponseError::new(HttpResponseErrorKind::ReceiveError.into(), "Test Error"),
                &retried,
            ),
        );
        assert_eq!(result.decision(), RetryDecision::TryNextServer);

        retried.switch_endpoint();

        let result = retrier.retry(
            &mut parts,
            &RequestRetrierOptions::new(
                Idempotent::Default,
                &ResponseError::new(HttpResponseErrorKind::ReceiveError.into(), "Test Error"),
                &retried,
            ),
        );
        assert_eq!(result.decision(), RetryDecision::RetryRequest);

        Ok(())
    }
}
