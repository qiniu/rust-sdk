use super::{Backoff, BackoffOptions, GotBackoffDuration, RetryDecision};
use qiniu_http::RequestParts as HttpRequestParts;
use std::time::Duration;

/// 指数级增长的退避时长提供者
///
/// 默认底数为 2，基础时长为 100 毫秒
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExponentialBackoff {
    base_number: u32,
    base_delay: Duration,
}

impl ExponentialBackoff {
    /// 创建指数级增长的退避时长提供者
    ///
    /// 需要提供底数和基础时长，返回的退避时长为 `base_delay * (base_number ^ retry_count)`
    #[inline]
    pub const fn new(base_number: u32, base_delay: Duration) -> Self {
        Self {
            base_number,
            base_delay,
        }
    }

    /// 获取底数
    #[inline]
    pub const fn base_number(&self) -> u32 {
        self.base_number
    }

    /// 获取基础时长
    #[inline]
    pub const fn base_delay(&self) -> Duration {
        self.base_delay
    }
}

impl Backoff for ExponentialBackoff {
    fn time(&self, _request: &mut HttpRequestParts, opts: BackoffOptions) -> GotBackoffDuration {
        let retried_count = if opts.retry_decision() == RetryDecision::Throttled {
            opts.retried().retried_total()
        } else {
            opts.retried().retried_on_current_endpoint()
        };
        GotBackoffDuration::from(self.base_delay * self.base_number.pow(retried_count as u32))
    }
}

impl Default for ExponentialBackoff {
    #[inline]
    fn default() -> Self {
        ExponentialBackoff::new(2, Duration::from_millis(100))
    }
}
