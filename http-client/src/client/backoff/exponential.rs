use super::{Backoff, ResponseError, RetriedStatsInfo, RetryResult};
use qiniu_http::Request as HTTPRequest;
use std::{any::Any, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExponentialBackoff {
    base_delay: Duration,
}

impl ExponentialBackoff {
    #[inline]
    pub const fn new(base_delay: Duration) -> Self {
        Self { base_delay }
    }
}

impl Backoff for ExponentialBackoff {
    #[inline]
    fn time(
        &self,
        _request: &mut HTTPRequest,
        retry_result: RetryResult,
        _response_error: &ResponseError,
        retried: &RetriedStatsInfo,
    ) -> Duration {
        let retried_count = if retry_result == RetryResult::Throttled {
            retried.retried_total()
        } else {
            retried.retried_on_current_endpoint()
        };
        self.base_delay * 2_u32.pow(retried_count as u32)
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_backoff(&self) -> &dyn Backoff {
        self
    }
}

impl Default for ExponentialBackoff {
    #[inline]
    fn default() -> Self {
        ExponentialBackoff::new(Duration::from_millis(100))
    }
}
