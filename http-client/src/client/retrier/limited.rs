use super::{RequestRetrier, RequestRetrierOptions, RetryDecision, RetryResult};
use qiniu_http::RequestParts as HttpRequestParts;

const DEFAULT_RETIES: usize = 2;

#[derive(Copy, Clone, Debug)]
enum LimitTarget {
    LimitCurrentEndpoint,
    LimitTotal,
}

impl LimitTarget {
    fn retry(
        self,
        decision: RetryDecision,
        retries: usize,
        opts: &RequestRetrierOptions,
    ) -> RetryDecision {
        match self {
            Self::LimitCurrentEndpoint => match decision {
                RetryDecision::RetryRequest | RetryDecision::Throttled
                    if opts.retried().retried_on_current_endpoint() >= retries =>
                {
                    RetryDecision::TryNextServer
                }
                result => result,
            },
            Self::LimitTotal => match decision {
                RetryDecision::RetryRequest | RetryDecision::Throttled
                    if opts.retried().retried_total() >= retries =>
                {
                    RetryDecision::DontRetry
                }
                result => result,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct LimitedRetrier<R: ?Sized> {
    retries: usize,
    target: LimitTarget,
    retrier: R,
}

impl<R> LimitedRetrier<R> {
    #[inline]
    pub fn new(retrier: R, retries: usize) -> Self {
        Self::limit_current_endpoint(retrier, retries)
    }

    #[inline]
    pub fn limit_current_endpoint(retrier: R, retries: usize) -> Self {
        Self {
            retrier,
            retries,
            target: LimitTarget::LimitCurrentEndpoint,
        }
    }

    #[inline]
    pub fn limit_total(retrier: R, retries: usize) -> Self {
        Self {
            retrier,
            retries,
            target: LimitTarget::LimitTotal,
        }
    }
}

impl<R: Default> Default for LimitedRetrier<R> {
    #[inline]
    fn default() -> Self {
        Self::new(R::default(), DEFAULT_RETIES)
    }
}

impl<R: RequestRetrier> RequestRetrier for LimitedRetrier<R> {
    fn retry(&self, request: &mut HttpRequestParts, opts: &RequestRetrierOptions) -> RetryResult {
        self.target
            .retry(
                self.retrier.retry(request, opts).decision(),
                self.retries,
                opts,
            )
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

        let current_endpoint_retrier = LimitedRetrier::new(ErrorRetrier, 2);
        let total_retrier = LimitedRetrier::limit_total(ErrorRetrier, 2);
        let mut retried = RetriedStatsInfo::default();
        retried.increase();
        retried.increase();

        let (mut parts, _) = HttpRequest::builder()
            .url(uri)
            .method(HttpMethod::GET)
            .body(())
            .build()
            .into_parts();
        let result = current_endpoint_retrier.retry(
            &mut parts,
            &RequestRetrierOptions::new(
                Idempotent::Default,
                &ResponseError::new(HttpResponseErrorKind::ReceiveError.into(), "Test Error"),
                &retried,
            ),
        );
        assert_eq!(result.decision(), RetryDecision::TryNextServer);

        let result = total_retrier.retry(
            &mut parts,
            &RequestRetrierOptions::new(
                Idempotent::Default,
                &ResponseError::new(HttpResponseErrorKind::ReceiveError.into(), "Test Error"),
                &retried,
            ),
        );
        assert_eq!(result.decision(), RetryDecision::DontRetry);

        retried.switch_endpoint();

        let result = current_endpoint_retrier.retry(
            &mut parts,
            &RequestRetrierOptions::new(
                Idempotent::Default,
                &ResponseError::new(HttpResponseErrorKind::ReceiveError.into(), "Test Error"),
                &retried,
            ),
        );
        assert_eq!(result.decision(), RetryDecision::RetryRequest);

        let result = total_retrier.retry(
            &mut parts,
            &RequestRetrierOptions::new(
                Idempotent::Default,
                &ResponseError::new(HttpResponseErrorKind::ReceiveError.into(), "Test Error"),
                &retried,
            ),
        );
        assert_eq!(result.decision(), RetryDecision::DontRetry);

        Ok(())
    }
}
