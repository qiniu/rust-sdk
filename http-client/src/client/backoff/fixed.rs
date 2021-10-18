use super::{Backoff, BackoffDuration, BackoffOptions};
use qiniu_http::Request as HTTPRequest;
use std::{any::Any, time::Duration};

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
    fn time(&self, _request: &mut HTTPRequest, _opts: &BackoffOptions) -> BackoffDuration {
        self.delay.into()
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

impl Default for FixedBackoff {
    #[inline]
    fn default() -> Self {
        FixedBackoff::new(Duration::from_millis(100))
    }
}
