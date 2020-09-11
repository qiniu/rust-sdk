use qiniu_http::{Request, SyncResponseResult};
use std::{any::Any, fmt::Debug};

#[cfg(feature = "async")]
use {futures::future::BoxFuture, qiniu_http::AsyncResponseResult};

pub trait RequestRetrier: Any + Debug + Sync + Send {
    fn retry(
        &self,
        request: &mut Request,
        response_result: SyncResponseResult,
        retried: usize,
    ) -> RetryResult;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_retry(
        &self,
        request: &mut Request,
        response_result: AsyncResponseResult,
        retried: usize,
    ) -> BoxFuture<RetryResult>;

    fn as_any(&self) -> &dyn Any;
    fn as_request_retrier(&self) -> &dyn RequestRetrier;
}

#[derive(Copy, Clone, Debug)]
pub enum RetryResult {
    DontRetry,
    TryNextServer,
    RetryRequest,
}

#[derive(Copy, Clone, Debug)]
pub struct NeverRetry;

impl RequestRetrier for NeverRetry {
    fn retry(
        &self,
        _request: &mut Request,
        _response_result: SyncResponseResult,
        _retried: usize,
    ) -> RetryResult {
        RetryResult::DontRetry
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_retry(
        &self,
        _request: &mut Request,
        _response_result: AsyncResponseResult,
        _retried: usize,
    ) -> BoxFuture<RetryResult> {
        Box::pin(async { RetryResult::DontRetry })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_request_retrier(&self) -> &dyn RequestRetrier {
        self
    }
}

// TODO: Default RequestRetier
