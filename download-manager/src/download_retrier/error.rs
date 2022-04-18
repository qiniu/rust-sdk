use super::{DownloadRetrier, DownloadRetrierOptions, RetryDecision, RetryResult};
use qiniu_apis::http_client::{CallbackContext, ResponseErrorKind};

/// 根据七牛 API 返回的状态码作出重试决定
#[derive(Copy, Clone, Debug, Default)]
pub struct ErrorRetrier;

impl DownloadRetrier for ErrorRetrier {
    fn retry(&self, _request: &mut dyn CallbackContext, opts: DownloadRetrierOptions<'_>) -> RetryResult {
        if let ResponseErrorKind::StatusCodeError(status_code) = opts.response_error().kind() {
            if (400..500).contains(&status_code.as_u16()) {
                return RetryDecision::DontRetry.into();
            }
        }
        RetryDecision::RetryRequest.into()
    }
}
