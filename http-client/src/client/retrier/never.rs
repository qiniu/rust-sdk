use super::{RequestRetrier, RequestRetrierOptions, RetryDecision, RetryResult};
use qiniu_http::RequestParts as HttpRequestParts;

#[derive(Default, Copy, Clone, Debug)]
pub struct NeverRetrier;

impl RequestRetrier for NeverRetrier {
    #[inline]
    fn retry(&self, _request: &mut HttpRequestParts, _opts: &RequestRetrierOptions) -> RetryResult {
        RetryDecision::DontRetry.into()
    }
}
