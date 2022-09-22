use super::{Backoff, BackoffOptions, GotBackoffDuration};
use qiniu_http::RequestParts as HttpRequestParts;
use std::time::Duration;

/// 限制范围的退避时长提供者
///
/// 基于一个退避时长提供者并为其增加限制范围
///
/// 默认的限制范围为 `[0, 5]` 秒
#[derive(Debug, Clone)]
pub struct LimitedBackoff<P: ?Sized> {
    max_backoff: Duration,
    min_backoff: Duration,
    base_backoff: P,
}

impl<P> LimitedBackoff<P> {
    /// 创建限制范围的退避时长提供者
    ///
    /// 需要提供限制范围
    #[inline]
    pub const fn new(base_backoff: P, min_backoff: Duration, max_backoff: Duration) -> Self {
        Self {
            base_backoff,
            min_backoff,
            max_backoff,
        }
    }

    /// 获取基础的退避时长提供者
    #[inline]
    pub const fn base_backoff(&self) -> &P {
        &self.base_backoff
    }

    /// 获取最短的退避时长
    #[inline]
    pub const fn max_backoff(&self) -> Duration {
        self.max_backoff
    }

    /// 获取最长的退避时长
    #[inline]
    pub const fn min_backoff(&self) -> Duration {
        self.min_backoff
    }
}

impl<P: Backoff + Clone> Backoff for LimitedBackoff<P> {
    #[inline]
    fn time(&self, request: &mut HttpRequestParts, opts: BackoffOptions) -> GotBackoffDuration {
        self.base_backoff
            .time(request, opts)
            .duration()
            .max(self.min_backoff)
            .min(self.max_backoff)
            .into()
    }
}

impl<P: Default> Default for LimitedBackoff<P> {
    #[inline]
    fn default() -> Self {
        LimitedBackoff::new(P::default(), Duration::from_secs(0), Duration::from_secs(300))
    }
}
