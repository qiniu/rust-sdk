mod never;

use qiniu_http::{Request, ResponseError};
use std::{any::Any, fmt::Debug};

pub trait RequestRetrier: Any + Debug + Sync + Send {
    fn retry(
        &self,
        request: &mut Request,
        response_error: ResponseError,
        retried: usize,
    ) -> RetryResult;

    fn as_any(&self) -> &dyn Any;
    fn as_request_retrier(&self) -> &dyn RequestRetrier;
}

#[derive(Copy, Clone, Debug)]
pub enum RetryResult {
    DontRetry,
    TryNextServer,
    RetryRequest,
}

pub use never::NeverRetry;
// TODO: 提供一个 Default RequestRetrier
