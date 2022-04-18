use super::{DownloadRetrier, DownloadRetrierOptions, RetryDecision, RetryResult};
use qiniu_apis::http_client::CallbackContext;

/// 永不重试器
///
/// 总是返回不再重试的重试器
#[derive(Copy, Clone, Debug, Default)]
pub struct NeverRetrier;

impl DownloadRetrier for NeverRetrier {
    #[inline]
    fn retry(&self, _request: &mut dyn CallbackContext, _opts: DownloadRetrierOptions<'_>) -> RetryResult {
        RetryDecision::DontRetry.into()
    }
}
