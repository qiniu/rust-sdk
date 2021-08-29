mod error;
mod limited;
mod never;

use super::{Idempotent, ResponseError, RetriedStatsInfo};
use qiniu_http::Request as HTTPRequest;
use std::{any::Any, fmt::Debug};

pub trait RequestRetrier: Any + Debug + Sync + Send {
    fn retry(
        &self,
        request: &mut HTTPRequest,
        idempotent: Idempotent,
        response_error: &ResponseError,
        retried: &RetriedStatsInfo,
    ) -> RetryResult;

    fn as_any(&self) -> &dyn Any;
    fn as_request_retrier(&self) -> &dyn RequestRetrier;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RetryResult {
    DontRetry,
    TryNextServer,
    TryOldEndpoints,
    RetryRequest,
    Throttled,
}

pub use error::ErrorRetrier;
pub use limited::LimitedRetrier;
pub use never::NeverRetrier;
