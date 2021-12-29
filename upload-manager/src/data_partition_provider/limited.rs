use super::{DataPartitionProvider, DataPartitionProviderFeedback, PartSize};
use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy)]
pub struct LimitedDataPartitionProvider<P> {
    base: P,
    min: NonZeroU64,
    max: NonZeroU64,
}

impl<P: DataPartitionProvider> LimitedDataPartitionProvider<P> {
    #[inline]
    pub fn new(base: P, min: u64, max: u64) -> Option<Self> {
        match (NonZeroU64::new(min), NonZeroU64::new(max)) {
            (Some(min), Some(max)) => Some(Self { base, min, max }),
            _ => None,
        }
    }
}

impl<P: DataPartitionProvider + Default> Default for LimitedDataPartitionProvider<P> {
    #[inline]
    fn default() -> Self {
        Self {
            base: Default::default(),
            min: NonZeroU64::new(1 << 20).unwrap(),
            max: NonZeroU64::new(1 << 30).unwrap(),
        }
    }
}

impl<P> LimitedDataPartitionProvider<P> {
    #[inline]
    pub fn min_part_size(&self) -> NonZeroU64 {
        self.min
    }

    #[inline]
    pub fn max_part_size(&self) -> NonZeroU64 {
        self.max
    }
}

impl<P: DataPartitionProvider> DataPartitionProvider for LimitedDataPartitionProvider<P> {
    #[inline]
    fn part_size(&self) -> PartSize {
        let base_partition = self.base.part_size().as_non_zero_u64();
        base_partition.min(self.max).max(self.min).into()
    }

    #[inline]
    fn feedback(&self, feedback: DataPartitionProviderFeedback<'_>) {
        self.base.feedback(feedback)
    }
}
