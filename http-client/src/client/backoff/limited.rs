use super::{Backoff, BackoffDuration, BackoffOptions};
use qiniu_http::RequestParts as HttpRequestParts;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct LimitedBackoff<P: ?Sized> {
    max_backoff: Duration,
    min_backoff: Duration,
    base_backoff: P,
}

impl<P> LimitedBackoff<P> {
    #[inline]
    pub const fn new(base_backoff: P, min_backoff: Duration, max_backoff: Duration) -> Self {
        Self {
            base_backoff,
            min_backoff,
            max_backoff,
        }
    }

    #[inline]
    pub const fn base_backoff(&self) -> &P {
        &self.base_backoff
    }

    #[inline]
    pub const fn max_backoff(&self) -> Duration {
        self.max_backoff
    }

    #[inline]
    pub const fn min_backoff(&self) -> Duration {
        self.min_backoff
    }
}

impl<P: Backoff> Backoff for LimitedBackoff<P> {
    #[inline]
    fn time(&self, request: &mut HttpRequestParts, opts: &BackoffOptions) -> BackoffDuration {
        self.base_backoff
            .time(request, opts)
            .duration()
            .max(self.min_backoff)
            .min(self.max_backoff)
            .into()
    }
}

impl<P: Default> Default for LimitedBackoff<P> {
    #[inline]
    fn default() -> Self {
        LimitedBackoff::new(
            P::default(),
            Duration::from_secs(0),
            Duration::from_secs(300),
        )
    }
}
