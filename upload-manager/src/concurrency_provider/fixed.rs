use std::num::NonZeroUsize;

use super::{Concurrency, ConcurrencyProvider, ConcurrencyProviderFeedback};

#[derive(Debug, Clone, Copy)]
pub struct FixedConcurrencyProvider(NonZeroUsize);

impl FixedConcurrencyProvider {
    #[inline]
    pub fn new(concurrency: usize) -> Option<Self> {
        NonZeroUsize::new(concurrency).map(Self)
    }

    #[inline]
    pub fn fixed_concurrency(&self) -> NonZeroUsize {
        self.0
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
