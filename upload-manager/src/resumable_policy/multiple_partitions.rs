use super::{
    super::DataPartitionProvider, GetPolicyOptions, ResumablePolicy, ResumablePolicyProvider,
};
use std::num::NonZeroU64;

#[derive(Debug, Clone)]
pub struct MultiplePartitionsResumablePolicyProvider<P: ?Sized> {
    multiply: NonZeroU64,
    base_partition_provider: P,
}

impl<P> MultiplePartitionsResumablePolicyProvider<P> {
    #[inline]
    pub fn new(base_partition_provider: P, multiply: u64) -> Option<Self> {
        NonZeroU64::new(multiply)
            .map(|multiply| Self::new_with_non_zero_multiply(base_partition_provider, multiply))
    }

    #[inline]
    pub fn new_with_non_zero_multiply(base_partition_provider: P, multiply: NonZeroU64) -> Self {
        Self {
            base_partition_provider,
            multiply,
        }
    }
}

impl<P: DataPartitionProvider> ResumablePolicyProvider
    for MultiplePartitionsResumablePolicyProvider<P>
{
    #[inline]
    fn get_policy(&self, source_size: u64, _opts: &GetPolicyOptions) -> ResumablePolicy {
        if self.threshold() <= source_size {
            ResumablePolicy::SinglePartUploading
        } else {
            ResumablePolicy::MultiPartsUploading
        }
    }
}

impl<P: DataPartitionProvider> MultiplePartitionsResumablePolicyProvider<P> {
    fn threshold(&self) -> u64 {
        self.base_partition_provider.part_size().as_u64() * self.multiply.get()
    }
}
