use super::{Backoff, BackoffDuration, BackoffOptions, RetryDecision};
use qiniu_http::Request as HTTPRequest;
use std::time::Duration;

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
    fn time(&self, _request: &mut HTTPRequest, opts: &BackoffOptions) -> BackoffDuration {
        let retried_count = if opts.retry_decision() == RetryDecision::Throttled {
            opts.retried().retried_total()
        } else {
            opts.retried().retried_on_current_endpoint()
        };
        BackoffDuration::from(self.base_delay * 2_u32.pow(retried_count as u32))
    }
}

impl Default for ExponentialBackoff {
    #[inline]
    fn default() -> Self {
        ExponentialBackoff::new(Duration::from_millis(100))
    }
}
