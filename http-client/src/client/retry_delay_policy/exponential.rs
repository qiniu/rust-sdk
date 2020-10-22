use super::{ResponseError, RetryDelayPolicy};
use qiniu_http::Request as HTTPRequest;
use std::{any::Any, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExponentialRetryDelayPolicy {
    base_delay: Duration,
}

impl ExponentialRetryDelayPolicy {
    #[inline]
    pub const fn new(base_delay: Duration) -> Self {
        Self { base_delay }
    }
}

impl RetryDelayPolicy for ExponentialRetryDelayPolicy {
    #[inline]
    fn delay_before_next_retry(
        &self,
        _request: &mut HTTPRequest,
        _response_error: &ResponseError,
        retried: usize,
    ) -> Duration {
        self.base_delay * 2_u32.pow(retried as u32)
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

impl Default for ExponentialRetryDelayPolicy {
    #[inline]
    fn default() -> Self {
        ExponentialRetryDelayPolicy::new(Duration::from_millis(100))
    }
}