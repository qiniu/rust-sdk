use super::{Backoff, BackoffOptions, GotBackoffDuration};
use qiniu_http::RequestParts as HttpRequestParts;
use std::time::Duration;

/// 无退避的退避时长提供者
pub const NO_BACKOFF: FixedBackoff = FixedBackoff::new(Duration::from_nanos(0));

/// 固定时长的退避时长提供者
///
/// 默认时长为 100 毫秒
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedBackoff {
    delay: Duration,
}

impl FixedBackoff {
    /// 创建固定时长的退避时长提供者
    #[inline]
    pub const fn new(delay: Duration) -> Self {
        Self { delay }
    }

    /// 获取固定时长
    #[inline]
    pub const fn delay(&self) -> Duration {
        self.delay
    }
}

impl Backoff for FixedBackoff {
    #[inline]
    fn time(&self, _request: &mut HttpRequestParts, _opts: BackoffOptions) -> GotBackoffDuration {
        self.delay.into()
    }
}

impl Default for FixedBackoff {
    #[inline]
    fn default() -> Self {
        FixedBackoff::new(Duration::from_millis(100))
    }
}
