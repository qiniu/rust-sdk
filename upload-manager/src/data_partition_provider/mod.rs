use auto_impl::auto_impl;
use qiniu_apis::http_client::ResponseError;
use std::{
    fmt::Debug,
    num::NonZeroU64,
    ops::{Deref, DerefMut},
    time::Duration,
};

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait DataPartitionProvider: Debug + Sync + Send {
    fn part_size(&self) -> PartSize;
    fn feedback(&self, feedback: DataPartitionProviderFeedback<'_>);
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartSize(NonZeroU64);

impl PartSize {
    pub fn new(part_size: u64) -> Option<Self> {
        NonZeroU64::new(part_size).map(Self)
    }

    #[inline]
    pub fn as_non_zero_u64(&self) -> NonZeroU64 {
        self.0
    }

    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.as_non_zero_u64().get()
    }
}

impl Default for PartSize {
    #[inline]
    fn default() -> Self {
        Self(NonZeroU64::new(1 << 22).unwrap())
    }
}

impl From<NonZeroU64> for PartSize {
    #[inline]
    fn from(size: NonZeroU64) -> Self {
        Self(size)
    }
}

impl From<PartSize> for NonZeroU64 {
    #[inline]
    fn from(size: PartSize) -> Self {
        size.as_non_zero_u64()
    }
}

impl From<PartSize> for u64 {
    #[inline]
    fn from(size: PartSize) -> Self {
        size.as_u64()
    }
}

impl Deref for PartSize {
    type Target = NonZeroU64;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PartSize {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct DataPartitionProviderFeedback<'f> {
    part_size: NonZeroU64,
    elapsed: Duration,
    error: Option<&'f ResponseError>,
}

impl<'f> DataPartitionProviderFeedback<'f> {
    pub(super) fn new(
        part_size: NonZeroU64,
        elapsed: Duration,
        error: Option<&'f ResponseError>,
    ) -> Self {
        Self {
            part_size,
            elapsed,
            error,
        }
    }

    #[inline]
    pub fn part_size(&self) -> NonZeroU64 {
        self.part_size
    }

    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    #[inline]
    pub fn error(&self) -> Option<&'f ResponseError> {
        self.error
    }
}

mod fixed;
pub use fixed::FixedDataPartitionProvider;

mod time_aware;
pub use time_aware::TimeAwareDataPartitionProvider;

mod limited;
pub use limited::LimitedDataPartitionProvider;

mod multiply;
pub use multiply::MultiplyDataPartitionProvider;
