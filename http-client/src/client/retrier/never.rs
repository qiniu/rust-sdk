use qiniu_http::ResponseError;

#[derive(Copy, Clone, Debug)]
pub struct NeverRetry;

impl RequestRetrier for NeverRetry {
    #[inline]
    fn retry(
        &self,
        _request: &mut Request,
        _response_error: ResponseError,
        _retried: usize,
    ) -> RetryResult {
        RetryResult::DontRetry
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_request_retrier(&self) -> &dyn RequestRetrier {
        self
    }
}
