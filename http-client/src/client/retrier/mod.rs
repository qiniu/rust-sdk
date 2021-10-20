mod error;
mod limited;
mod never;

use super::{Idempotent, ResponseError, RetriedStatsInfo};
use qiniu_http::Request as HTTPRequest;
use std::{
    any::Any,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

pub trait RequestRetrier: Any + Debug + Sync + Send {
    fn retry(&self, request: &mut HTTPRequest, opts: &RequestRetrierOptions) -> RetryResult;

    fn as_any(&self) -> &dyn Any;
    fn as_request_retrier(&self) -> &dyn RequestRetrier;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RetryDecision {
    DontRetry,
    TryNextServer,
    TryAlternativeEndpoints,
    RetryRequest,
    Throttled,
}

impl Default for RetryDecision {
    #[inline]
    fn default() -> Self {
        Self::DontRetry
    }
}

#[derive(Debug, Clone)]
pub struct RequestRetrierOptions<'a> {
    idempotent: Idempotent,
    response_error: &'a ResponseError,
    retried: &'a RetriedStatsInfo,
}

impl<'a> RequestRetrierOptions<'a> {
    #[inline]
    pub(super) fn new(
        idempotent: Idempotent,
        response_error: &'a ResponseError,
        retried: &'a RetriedStatsInfo,
    ) -> Self {
        Self {
            idempotent,
            response_error,
            retried,
        }
    }

    #[inline]
    pub fn idempotent(&self) -> Idempotent {
        self.idempotent
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

#[derive(Clone)]
pub struct RetryResult(RetryDecision);

impl RetryResult {
    #[inline]
    pub fn decision(&self) -> RetryDecision {
        self.0
    }

    #[inline]
    pub fn decision_mut(&mut self) -> &mut RetryDecision {
        &mut self.0
    }
}

impl From<RetryDecision> for RetryResult {
    #[inline]
    fn from(decision: RetryDecision) -> Self {
        Self(decision)
    }
}

impl From<RetryResult> for RetryDecision {
    #[inline]
    fn from(result: RetryResult) -> Self {
        result.0
    }
}

impl AsRef<RetryDecision> for RetryResult {
    #[inline]
    fn as_ref(&self) -> &RetryDecision {
        &self.0
    }
}

impl AsMut<RetryDecision> for RetryResult {
    #[inline]
    fn as_mut(&mut self) -> &mut RetryDecision {
        &mut self.0
    }
}

impl Deref for RetryResult {
    type Target = RetryDecision;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RetryResult {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub use error::ErrorRetrier;
pub use limited::LimitedRetrier;
pub use never::NeverRetrier;
