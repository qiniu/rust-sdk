use std::num::NonZeroUsize;

use super::{Concurrency, ConcurrencyProvider, ConcurrencyProviderFeedback};

/// 固定并发数提供者
#[derive(Debug, Clone, Copy)]
pub struct FixedConcurrencyProvider(NonZeroUsize);

impl FixedConcurrencyProvider {
    /// 创建固定并发数提供者
    ///
    /// 如果传入 `0` 将返回 [`None`]。
    #[inline]
    pub fn new(concurrency: usize) -> Option<Self> {
        NonZeroUsize::new(concurrency).map(Self::new_with_non_zero_concurrency)
    }

    /// 创建固定并发数提供者
    ///
    /// 提供 [`NonZeroUsize`] 作为并发数类型。
    #[inline]
    pub const fn new_with_non_zero_concurrency(concurrency: NonZeroUsize) -> Self {
        Self(concurrency)
    }

    /// 获取固定并发数
    #[inline]
    pub const fn fixed_concurrency(&self) -> NonZeroUsize {
        self.0
    }
}

impl Default for FixedConcurrencyProvider {
    #[inline]
    fn default() -> Self {
        Self::new_with_non_zero_concurrency(
            #[allow(unsafe_code)]
            unsafe {
                NonZeroUsize::new_unchecked(4)
            },
        )
    }
}

impl ConcurrencyProvider for FixedConcurrencyProvider {
    #[inline]
    fn concurrency(&self) -> Concurrency {
        self.fixed_concurrency().into()
    }

    #[inline]
    fn feedback(&self, _feedback: ConcurrencyProviderFeedback<'_>) {}
}

impl From<NonZeroUsize> for FixedConcurrencyProvider {
    #[inline]
    fn from(concurrency: NonZeroUsize) -> Self {
        Self(concurrency)
    }
}
