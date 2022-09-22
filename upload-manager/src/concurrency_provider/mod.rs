use auto_impl::auto_impl;
use dyn_clonable::clonable;
use qiniu_apis::http_client::ResponseError;
use std::{
    fmt::Debug,
    num::{NonZeroU64, NonZeroUsize},
    ops::{Deref, DerefMut},
    time::Duration,
};

/// 并发数获取接口
///
/// 获取分片上传时的并发数
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait ConcurrencyProvider: Clone + Debug + Sync + Send {
    /// 获取并发数
    fn concurrency(&self) -> Concurrency;

    /// 反馈并发数结果
    fn feedback(&self, feedback: ConcurrencyProviderFeedback<'_>);
}

/// 上传并发数
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Concurrency(NonZeroUsize);

impl Concurrency {
    /// 创建上传并发数
    ///
    /// 如果传入 `0` 将返回 [`None`]。
    #[inline]
    pub fn new(concurrency: usize) -> Option<Self> {
        NonZeroUsize::new(concurrency).map(Self)
    }

    /// 创建上传并发数
    ///
    /// 提供 [`NonZeroUsize`] 作为并发数类型。
    #[inline]
    pub const fn new_with_non_zero_usize(concurrency: NonZeroUsize) -> Self {
        Self(concurrency)
    }

    /// 获取并发数
    #[inline]
    pub fn as_usize(&self) -> usize {
        self.as_non_zero_usize().get()
    }

    /// 获取并发数
    ///
    /// 返回 [`NonZeroUsize`] 作为并发数类型。
    #[inline]
    pub fn as_non_zero_usize(&self) -> NonZeroUsize {
        self.0
    }
}

impl Default for Concurrency {
    #[inline]
    fn default() -> Self {
        Self(NonZeroUsize::new(1).unwrap())
    }
}

impl From<NonZeroUsize> for Concurrency {
    #[inline]
    fn from(size: NonZeroUsize) -> Self {
        Self(size)
    }
}

impl From<Concurrency> for NonZeroUsize {
    #[inline]
    fn from(size: Concurrency) -> Self {
        size.as_non_zero_usize()
    }
}

impl From<Concurrency> for usize {
    #[inline]
    fn from(size: Concurrency) -> Self {
        size.as_usize()
    }
}

impl Deref for Concurrency {
    type Target = NonZeroUsize;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Concurrency {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// 并发数提供者反馈
///
/// 反馈给提供者并发的效果，包含对象大小，花费时间，以及错误信息。
#[derive(Debug, Clone)]
pub struct ConcurrencyProviderFeedback<'f> {
    concurrency: Concurrency,
    object_size: NonZeroU64,
    elapsed: Duration,
    error: Option<&'f ResponseError>,
}

impl<'f> ConcurrencyProviderFeedback<'f> {
    /// 创建并发数提供者反馈构建器
    #[inline]
    pub fn builder(
        concurrency: Concurrency,
        object_size: NonZeroU64,
        elapsed: Duration,
    ) -> ConcurrencyProviderFeedbackBuilder<'f> {
        ConcurrencyProviderFeedbackBuilder::new(concurrency, object_size, elapsed)
    }

    /// 获取并发数
    #[inline]
    pub fn concurrency(&self) -> Concurrency {
        self.concurrency
    }

    /// 获取对象大小
    #[inline]
    pub fn object_size(&self) -> NonZeroU64 {
        self.object_size
    }

    /// 获取花费时间
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// 获取错误信息
    #[inline]
    pub fn error(&self) -> Option<&'f ResponseError> {
        self.error
    }
}

/// 并发数提供者反馈构建器
#[derive(Debug, Clone)]
pub struct ConcurrencyProviderFeedbackBuilder<'f>(ConcurrencyProviderFeedback<'f>);

impl<'f> ConcurrencyProviderFeedbackBuilder<'f> {
    /// 创建并发数提供者反馈构建器
    #[inline]
    pub fn new(concurrency: Concurrency, object_size: NonZeroU64, elapsed: Duration) -> Self {
        Self(ConcurrencyProviderFeedback {
            concurrency,
            object_size,
            elapsed,
            error: None,
        })
    }

    /// 设置错误信息
    #[inline]
    pub fn error(&mut self, error: &'f ResponseError) -> &mut Self {
        self.0.error = Some(error);
        self
    }

    /// 构建并发数提供者反馈
    #[inline]
    pub fn build(&self) -> ConcurrencyProviderFeedback<'f> {
        self.0.to_owned()
    }
}

mod fixed;
pub use fixed::FixedConcurrencyProvider;
