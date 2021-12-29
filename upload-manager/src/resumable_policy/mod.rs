use auto_impl::auto_impl;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum ResumablePolicy {
    MultiPartsUploading,
    SinglePartUploading,
}

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait ResumablePolicyProvider: Debug + Sync + Send {
    fn get_policy(&self, source_size: u64, opts: &GetPolicyOptions) -> ResumablePolicy;
}

#[derive(Debug, Clone, Default)]
pub struct GetPolicyOptions {}

mod always_single_part;
pub use always_single_part::AlwaysSinglePart;

mod always_multi_parts;
pub use always_multi_parts::AlwaysMultiParts;

mod fixed;
pub use fixed::FixedThresholdResumablePolicy;

mod multiple_partitions;
pub use multiple_partitions::MultiplePartitionsResumablePolicyProvider;
