mod error;
mod limited;
mod never;

use super::{Idempotent, ResponseError, RetriedStatsInfo};
use auto_impl::auto_impl;
use dyn_clonable::clonable;
use qiniu_http::RequestParts as HttpRequestParts;
use smart_default::SmartDefault;
use std::{
    fmt::{self, Debug},
    ops::{Deref, DerefMut},
};

/// 请求重试器
///
/// 根据 HTTP 客户端返回的错误，决定是否重试请求，重试决定由 [`RetryDecision`] 定义。
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait RequestRetrier: Clone + Debug + Sync + Send {
    /// 作出重试决定
    fn retry(&self, request: &mut HttpRequestParts, opts: RequestRetrierOptions<'_>) -> RetryResult;
}

/// 重试决定
#[derive(Copy, Clone, Debug, Eq, PartialEq, SmartDefault)]
#[non_exhaustive]
pub enum RetryDecision {
    /// 不再重试
    #[default]
    DontRetry,

    /// 切换到下一个服务器
    TryNextServer,

    /// 切换到备选终端地址
    TryAlternativeEndpoints,

    /// 重试当前请求
    RetryRequest,

    /// 节流
    Throttled,
}

/// 重试器选项
#[derive(Copy, Debug, Clone)]
pub struct RequestRetrierOptions<'a> {
    idempotent: Idempotent,
    response_error: &'a ResponseError,
    retried: &'a RetriedStatsInfo,
}

impl<'a> RequestRetrierOptions<'a> {
    /// 创建重试器选项构建器
    #[inline]
    pub fn builder(
        response_error: &'a ResponseError,
        retried: &'a RetriedStatsInfo,
    ) -> RequestRetrierOptionsBuilder<'a> {
        RequestRetrierOptionsBuilder::new(response_error, retried)
    }

    /// 是否是幂等请求
    #[inline]
    pub fn idempotent(&self) -> Idempotent {
        self.idempotent
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

/// 重试器选项构建器
#[derive(Copy, Debug, Clone)]
pub struct RequestRetrierOptionsBuilder<'a>(RequestRetrierOptions<'a>);

impl<'a> RequestRetrierOptionsBuilder<'a> {
    /// 创建重试器选项构建器
    #[inline]
    pub fn new(response_error: &'a ResponseError, retried: &'a RetriedStatsInfo) -> Self {
        Self(RequestRetrierOptions {
            response_error,
            retried,
            idempotent: Default::default(),
        })
    }

    /// 设置幂等请求
    #[inline]
    pub fn idempotent(&mut self, idempotent: Idempotent) -> &mut Self {
        self.0.idempotent = idempotent;
        self
    }

    /// 构建重试器选项
    #[inline]
    pub fn build(&self) -> RequestRetrierOptions<'a> {
        self.0
    }
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

pub use error::ErrorRetrier;
pub use limited::LimitedRetrier;
pub use never::NeverRetrier;
