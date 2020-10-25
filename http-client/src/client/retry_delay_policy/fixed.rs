use super::{ResponseError, RetriedStatsInfo, RetryDelayPolicy, RetryResult};
use qiniu_http::Request as HTTPRequest;
use std::{any::Any, time::Duration};

pub const NO_DELAY_POLICY: FixedRetryDelayPolicy =
    FixedRetryDelayPolicy::new(Duration::from_nanos(0));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedRetryDelayPolicy {
    delay: Duration,
}

impl FixedRetryDelayPolicy {
    #[inline]
    pub const fn new(delay: Duration) -> Self {
        Self { delay }
    }
}

impl RetryDelayPolicy for FixedRetryDelayPolicy {
    #[inline]
    fn delay_before_next_retry(
        &self,
        _request: &mut HTTPRequest,
        _retry_result: RetryResult,
        _response_error: &ResponseError,
        _retried: &RetriedStatsInfo,
    ) -> Duration {
        self.delay
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_retry_delay_policy(&self) -> &dyn RetryDelayPolicy {
        self
    }
}

impl Default for FixedRetryDelayPolicy {
    #[inline]
    fn default() -> Self {
        FixedRetryDelayPolicy::new(Duration::from_millis(100))
    }
}
