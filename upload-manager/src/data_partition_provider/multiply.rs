use super::{DataPartitionProvider, DataPartitionProviderFeedback, PartSize};
use std::num::NonZeroU64;

#[derive(Debug, Clone, Copy)]
pub struct MultiplyDataPartitionProvider<P: ?Sized> {
    multiply: NonZeroU64,
    base: P,
}

impl<P: DataPartitionProvider> MultiplyDataPartitionProvider<P> {
    #[inline]
    pub fn new(base: P, multiply: u64) -> Option<Self> {
        NonZeroU64::new(multiply).map(|multiply| Self { base, multiply })
    }
}

impl<P: DataPartitionProvider + Default> Default for MultiplyDataPartitionProvider<P> {
    #[inline]
    fn default() -> Self {
        Self {
            base: Default::default(),
            multiply: NonZeroU64::new(1 << 20).unwrap(),
        }
    }
}

impl<P> MultiplyDataPartitionProvider<P> {
    #[inline]
    pub fn multiply(&self) -> NonZeroU64 {
        self.multiply
    }
}

impl<P: DataPartitionProvider> DataPartitionProvider for MultiplyDataPartitionProvider<P> {
    #[inline]
    fn part_size(&self) -> PartSize {
        let base_partition = self.base.part_size().as_non_zero_u64();
        let partition =
            base_partition.max(self.multiply).get() / self.multiply.get() * self.multiply.get();
        NonZeroU64::new(partition).unwrap().into()
    }

    #[inline]
    fn feedback(&self, feedback: DataPartitionProviderFeedback<'_>) {
        self.base.feedback(feedback)
    }
}
