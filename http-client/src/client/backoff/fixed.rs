use super::{Backoff, BackoffDuration, BackoffOptions};
use qiniu_http::RequestParts as HttpRequestParts;
use std::time::Duration;

pub const NO_BACKOFF: FixedBackoff = FixedBackoff::new(Duration::from_nanos(0));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedBackoff {
    delay: Duration,
}

impl FixedBackoff {
    #[inline]
    pub const fn new(delay: Duration) -> Self {
        Self { delay }
    }
}

impl Backoff for FixedBackoff {
    #[inline]
    fn time(&self, _request: &mut HttpRequestParts, _opts: &BackoffOptions) -> BackoffDuration {
        self.delay.into()
    }
}

impl Default for FixedBackoff {
    #[inline]
    fn default() -> Self {
        FixedBackoff::new(Duration::from_millis(100))
    }
}
