mod default;
mod never;

use super::ResponseError;
use qiniu_http::Request as HTTPRequest;
use std::{any::Any, fmt::Debug};

pub trait RequestRetrier: Any + Debug + Sync + Send {
    fn retry(
        &self,
        request: &mut HTTPRequest,
        response_error: &ResponseError,
        retried: usize,
    ) -> RetryResult;

    fn as_any(&self) -> &dyn Any;
    fn as_request_retrier(&self) -> &dyn RequestRetrier;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RetryResult {
    DontRetry,
    TryNextServer,
    RetryRequest,
}

pub use default::{DefaultRetrier, DefaultRetrierBuilder};
pub use never::NeverRetrier;
