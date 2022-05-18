mod exponential;
mod fixed;
mod limited;
mod randomized;

use super::{ResponseError, RetriedStatsInfo, RetryDecision};
use auto_impl::auto_impl;
use dyn_clonable::clonable;
use qiniu_http::RequestParts as HttpRequestParts;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    time::Duration,
};

/// 退避时长获取接口
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait Backoff: Clone + Debug + Sync + Send {
    /// 获取退避时长
    fn time(&self, request: &mut HttpRequestParts, opts: BackoffOptions) -> GotBackoffDuration;
}

/// 获取退避时长的选项
#[derive(Copy, Debug, Clone)]
pub struct BackoffOptions<'a> {
    retry_decision: RetryDecision,
    response_error: &'a ResponseError,
    retried: &'a RetriedStatsInfo,
}

impl<'a> BackoffOptions<'a> {
    /// 创建退避时长的选项构建器
    #[inline]
    pub fn builder(response_error: &'a ResponseError, retried: &'a RetriedStatsInfo) -> BackoffOptionsBuilder<'a> {
        BackoffOptionsBuilder::new(response_error, retried)
    }

    /// 获取重试决定
    #[inline]
    pub fn retry_decision(&self) -> RetryDecision {
        self.retry_decision
    }

    /// 获取响应错误
    #[inline]
    pub fn response_error(&self) -> &ResponseError {
        self.response_error
    }

    /// 获取重试信息
    #[inline]
    pub fn retried(&self) -> &RetriedStatsInfo {
        self.retried
    }
}

/// 退避时长的选项构建器
#[derive(Copy, Debug, Clone)]
pub struct BackoffOptionsBuilder<'a>(BackoffOptions<'a>);

impl<'a> BackoffOptionsBuilder<'a> {
    /// 创建退避时长的选项构建器
    #[inline]
    pub fn new(response_error: &'a ResponseError, retried: &'a RetriedStatsInfo) -> Self {
        Self(BackoffOptions {
            response_error,
            retried,
            retry_decision: Default::default(),
        })
    }

    /// 设置重试决定
    #[inline]
    pub fn retry_decision(&mut self, decision: RetryDecision) -> &mut Self {
        self.0.retry_decision = decision;
        self
    }

    /// 构建退避时长的选项
    pub fn build(&self) -> BackoffOptions<'a> {
        self.0
    }
}

/// 获取的退避时长
#[derive(Debug)]
pub struct GotBackoffDuration(Duration);

impl GotBackoffDuration {
    /// 获取退避时长
    #[inline]
    pub fn duration(&self) -> Duration {
        self.0
    }

    /// 获取退避时长的可变引用
    #[inline]
    pub fn duration_mut(&mut self) -> &mut Duration {
        &mut self.0
    }
}

impl From<Duration> for GotBackoffDuration {
    #[inline]
    fn from(duration: Duration) -> Self {
        Self(duration)
    }
}

impl From<GotBackoffDuration> for Duration {
    #[inline]
    fn from(backoff_duration: GotBackoffDuration) -> Self {
        backoff_duration.0
    }
}

impl AsRef<Duration> for GotBackoffDuration {
    #[inline]
    fn as_ref(&self) -> &Duration {
        &self.0
    }
}

impl AsMut<Duration> for GotBackoffDuration {
    #[inline]
    fn as_mut(&mut self) -> &mut Duration {
        &mut self.0
    }
}

impl Deref for GotBackoffDuration {
    type Target = Duration;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GotBackoffDuration {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub use exponential::ExponentialBackoff;
pub use fixed::{FixedBackoff, NO_BACKOFF};
pub use limited::LimitedBackoff;
pub use randomized::{RandomizedBackoff, Ratio};
