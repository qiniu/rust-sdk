mod exponential;
mod fixed;
mod randomized;

use super::{ResponseError, RetriedStatsInfo, RetryDecision};
use qiniu_http::Request as HTTPRequest;
use std::{
    any::Any,
    fmt::Debug,
    ops::{Deref, DerefMut},
    time::Duration,
};

pub trait Backoff: Any + Debug + Sync + Send {
    fn time(&self, request: &mut HTTPRequest, opts: &BackoffOptions) -> BackoffDuration;

    fn as_any(&self) -> &dyn Any;
    fn as_backoff(&self) -> &dyn Backoff;
}

#[derive(Debug, Clone)]
pub struct BackoffOptions<'a> {
    retry_decision: RetryDecision,
    response_error: &'a ResponseError,
    retried: &'a RetriedStatsInfo,
}

impl<'a> BackoffOptions<'a> {
    #[inline]
    pub(super) fn new(
        retry_decision: RetryDecision,
        response_error: &'a ResponseError,
        retried: &'a RetriedStatsInfo,
    ) -> Self {
        Self {
            retry_decision,
            response_error,
            retried,
        }
    }

    #[inline]
    pub fn retry_decision(&self) -> RetryDecision {
        self.retry_decision
    }

    #[inline]
    pub fn response_error(&self) -> &ResponseError {
        self.response_error
    }

    #[inline]
    pub fn retried(&self) -> &RetriedStatsInfo {
        self.retried
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BackoffDuration(Duration);

impl BackoffDuration {
    #[inline]
    pub fn duration(&self) -> Duration {
        self.0
    }

    #[inline]
    pub fn duration_mut(&mut self) -> &mut Duration {
        &mut self.0
    }
}

impl From<Duration> for BackoffDuration {
    #[inline]
    fn from(duration: Duration) -> Self {
        Self(duration)
    }
}

impl From<BackoffDuration> for Duration {
    #[inline]
    fn from(backoff_duration: BackoffDuration) -> Self {
        backoff_duration.0
    }
}

impl AsRef<Duration> for BackoffDuration {
    #[inline]
    fn as_ref(&self) -> &Duration {
        &self.0
    }
}

impl AsMut<Duration> for BackoffDuration {
    #[inline]
    fn as_mut(&mut self) -> &mut Duration {
        &mut self.0
    }
}

impl Deref for BackoffDuration {
    type Target = Duration;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BackoffDuration {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub use exponential::ExponentialBackoff;
pub use fixed::{FixedBackoff, NO_BACKOFF};
pub use randomized::{RandomizedBackoff, Ratio};
