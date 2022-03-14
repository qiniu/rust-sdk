use std::num::NonZeroUsize;

use super::{Concurrency, ConcurrencyProvider, ConcurrencyProviderFeedback};

#[derive(Debug, Clone, Copy)]
pub struct FixedConcurrencyProvider(NonZeroUsize);

impl FixedConcurrencyProvider {
    #[inline]
    pub fn new(concurrency: usize) -> Option<Self> {
        NonZeroUsize::new(concurrency).map(Self::new_with_non_zero_concurrency)
    }

    #[inline]
    pub fn new_with_non_zero_concurrency(concurrency: NonZeroUsize) -> Self {
        Self(concurrency)
    }

    #[inline]
    pub fn fixed_concurrency(&self) -> NonZeroUsize {
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
