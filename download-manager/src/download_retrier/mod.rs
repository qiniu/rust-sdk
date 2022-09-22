use auto_impl::auto_impl;
use dyn_clonable::clonable;
use qiniu_apis::http_client::{CallbackContext, ResponseError};
use std::{
    fmt::{self, Debug},
    ops::{Deref, DerefMut},
};

/// 下载重试器
///
/// 根据 HTTP 客户端返回的错误，决定是否重试请求，重试决定由 [`RetryDecision`] 定义。
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait DownloadRetrier: Clone + Debug + Sync + Send {
    /// 作出重试决定
    fn retry(&self, request: &mut dyn CallbackContext, opts: DownloadRetrierOptions<'_>) -> RetryResult;
}

/// 重试决定
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RetryDecision {
    /// 不再重试
    DontRetry,

    /// 切换到下一个服务器
    TryNextServer,

    /// 重试当前请求
    RetryRequest,
}

/// 重试器结果
#[derive(Clone)]
pub struct RetryResult(RetryDecision);

impl RetryResult {
    /// 获取重试决定
    #[inline]
    pub fn decision(&self) -> RetryDecision {
        self.0
    }

    /// 获取重试决定的可变引用
    #[inline]
    pub fn decision_mut(&mut self) -> &mut RetryDecision {
        &mut self.0
    }
}

impl From<RetryDecision> for RetryResult {
    #[inline]
    fn from(decision: RetryDecision) -> Self {
        Self(decision)
    }
}

impl From<RetryResult> for RetryDecision {
    #[inline]
    fn from(result: RetryResult) -> Self {
        result.0
    }
}

impl AsRef<RetryDecision> for RetryResult {
    #[inline]
    fn as_ref(&self) -> &RetryDecision {
        &self.0
    }
}

impl AsMut<RetryDecision> for RetryResult {
    #[inline]
    fn as_mut(&mut self) -> &mut RetryDecision {
        &mut self.0
    }
}

impl Deref for RetryResult {
    type Target = RetryDecision;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RetryResult {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Debug for RetryResult {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// 下载重试器选项
#[derive(Copy, Debug, Clone)]
pub struct DownloadRetrierOptions<'a> {
    response_error: &'a ResponseError,
    retried: &'a RetriedStatsInfo,
}

impl<'a> DownloadRetrierOptions<'a> {
    /// 创建下载重试器选项
    #[inline]
    pub fn new(response_error: &'a ResponseError, retried: &'a RetriedStatsInfo) -> Self {
        Self {
            response_error,
            retried,
        }
    }

    /// 获取响应错误
    #[inline]
    pub fn response_error(&self) -> &ResponseError {
        self.response_error
    }

    /// 获取重试统计信息
    #[inline]
    pub fn retried(&self) -> &RetriedStatsInfo {
        self.retried
    }
}

/// 重试统计信息
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RetriedStatsInfo {
    retried_total: usize,
    retried_on_current_endpoint: usize,
    abandoned_endpoints: usize,
}

impl RetriedStatsInfo {
    /// 提升当前终端地址的重试次数
    #[inline]
    pub fn increase(&mut self) {
        self.retried_total += 1;
        self.retried_on_current_endpoint += 1;
    }

    /// 切换终端地址
    #[inline]
    pub fn switch_endpoint(&mut self) {
        self.retried_on_current_endpoint = 0;
        self.abandoned_endpoints += 1;
    }

    /// 获取总共重试的次数
    #[inline]
    pub fn retried_total(&self) -> usize {
        self.retried_total
    }

    /// 获取当前终端地址的重试次数
    #[inline]
    pub fn retried_on_current_endpoint(&self) -> usize {
        self.retried_on_current_endpoint
    }

    /// 获取放弃的终端地址的数量
    #[inline]
    pub fn abandoned_endpoints(&self) -> usize {
        self.abandoned_endpoints
    }
}

mod error;
pub use error::ErrorRetrier;

mod never;
pub use never::NeverRetrier;
