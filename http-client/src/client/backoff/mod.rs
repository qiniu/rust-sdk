mod exponential;
mod fixed;
mod randomized;

use super::{ResponseError, RetriedStatsInfo, RetryResult};
use qiniu_http::Request as HTTPRequest;
use std::{any::Any, fmt::Debug, time::Duration};

pub trait Backoff: Any + Debug + Sync + Send {
    fn time(
        &self,
        request: &mut HTTPRequest,
        retry_result: RetryResult,
        response_error: &ResponseError,
        retried: &RetriedStatsInfo,
    ) -> Duration;

    fn as_any(&self) -> &dyn Any;
    fn as_backoff(&self) -> &dyn Backoff;
}

pub use exponential::ExponentialBackoff;
pub use fixed::{FixedBackoff, NO_BACKOFF};
pub use randomized::{RandomizedBackoff, Ratio};
