mod exponential;
mod fixed;
mod randomized;

use super::{ResponseError, RetriedStatsInfo, RetryResult};
use qiniu_http::Request as HTTPRequest;
use std::{any::Any, fmt::Debug, time::Duration};

pub trait RetryDelayPolicy: Any + Debug + Sync + Send {
    fn delay_before_next_retry(
        &self,
        request: &mut HTTPRequest,
        retry_result: RetryResult,
        response_error: &ResponseError,
        retried: &RetriedStatsInfo,
    ) -> Duration;

    fn as_any(&self) -> &dyn Any;
    fn as_retry_delay_policy(&self) -> &dyn RetryDelayPolicy;
}

pub use exponential::ExponentialRetryDelayPolicy;
pub use fixed::{FixedRetryDelayPolicy, NO_DELAY_POLICY};
pub use randomized::{RandomizedRetryDelayPolicy, Ratio};
