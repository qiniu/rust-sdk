use auto_impl::auto_impl;
use qiniu_apis::{http::Extensions, http_client::ResponseError};
use std::{
    fmt::Debug,
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    time::Duration,
};

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait ConcurrencyProvider: Debug + Sync + Send {
    fn concurrency(&self) -> Concurrency;
    fn feedback(&self, feedback: ConcurrencyProviderFeedback<'_>);
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Concurrency(NonZeroUsize);

impl Concurrency {
    #[inline]
    pub fn new(concurrency: usize) -> Option<Self> {
        NonZeroUsize::new(concurrency).map(Self::new_with_concurrency)
    }

    #[inline]
    pub fn new_with_concurrency(concurrency: NonZeroUsize) -> Self {
        Self(concurrency)
    }

    #[inline]
    pub fn as_usize(&self) -> usize {
        self.as_non_zero_usize().get()
    }

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

#[derive(Debug)]
pub struct ConcurrencyProviderFeedback<'f> {
    concurrency: NonZeroUsize,
    elapsed: Duration,
    extensions: &'f mut Extensions,
    error: Option<&'f ResponseError>,
}

impl<'f> ConcurrencyProviderFeedback<'f> {
    pub(super) fn new(
        concurrency: NonZeroUsize,
        elapsed: Duration,
        extensions: &'f mut Extensions,
        error: Option<&'f ResponseError>,
    ) -> Self {
        Self {
            concurrency,
            elapsed,
            extensions,
            error,
        }
    }

    #[inline]
    pub fn concurrency(&self) -> NonZeroUsize {
        self.concurrency
    }

    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    #[inline]
    pub fn error(&self) -> Option<&'f ResponseError> {
        self.error
    }

    #[inline]
    pub fn extensions(&self) -> &Extensions {
        self.extensions
    }

    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        self.extensions
    }
}

mod fixed;
pub use fixed::FixedConcurrencyProvider;

mod time_aware;
pub use time_aware::TimeAwareConcurrencyProvider;
