use super::{RequestRetrier, RequestRetrierOptions, RetryDecision, RetryResult};
use qiniu_http::Request as HTTPRequest;

#[derive(Default, Copy, Clone, Debug)]
pub struct NeverRetrier;

impl RequestRetrier for NeverRetrier {
    #[inline]
    fn retry(&self, _request: &mut HTTPRequest, _opts: &RequestRetrierOptions) -> RetryResult {
        RetryDecision::DontRetry.into()
    }
}
