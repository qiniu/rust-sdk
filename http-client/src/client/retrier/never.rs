use super::{RequestRetrier, RequestRetrierOptions, RetryDecision, RetryResult};
use qiniu_http::RequestParts as HTTPRequestParts;

#[derive(Default, Copy, Clone, Debug)]
pub struct NeverRetrier;

impl RequestRetrier for NeverRetrier {
    #[inline]
    fn retry(&self, _request: &mut HTTPRequestParts, _opts: &RequestRetrierOptions) -> RetryResult {
        RetryDecision::DontRetry.into()
    }
}
