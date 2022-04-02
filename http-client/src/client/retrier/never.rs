use super::{RequestRetrier, RequestRetrierOptions, RetryDecision, RetryResult};
use qiniu_http::RequestParts as HttpRequestParts;

/// 永不重试器
///
/// 总是返回不再重试的重试器
#[derive(Default, Copy, Clone, Debug)]
pub struct NeverRetrier;

impl RequestRetrier for NeverRetrier {
    #[inline]
    fn retry(&self, _request: &mut HttpRequestParts, _opts: RequestRetrierOptions) -> RetryResult {
        RetryDecision::DontRetry.into()
    }
}
