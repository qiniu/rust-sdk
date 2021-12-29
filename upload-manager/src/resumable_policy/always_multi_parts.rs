use super::{GetPolicyOptions, ResumablePolicy, ResumablePolicyProvider};

#[derive(Debug, Copy, Clone)]
pub struct AlwaysMultiParts;

impl ResumablePolicyProvider for AlwaysMultiParts {
    #[inline]
    fn get_policy(&self, _source_size: u64, _opts: &GetPolicyOptions) -> ResumablePolicy {
        ResumablePolicy::MultiPartsUploading
    }
}
