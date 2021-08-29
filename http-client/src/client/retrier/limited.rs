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
