use super::{GetPolicyOptions, ResumablePolicy, ResumablePolicyProvider};

#[derive(Debug, Copy, Clone)]
pub struct AlwaysSinglePart;

impl ResumablePolicyProvider for AlwaysSinglePart {
    #[inline]
    fn get_policy(&self, _source_size: u64, _opts: &GetPolicyOptions) -> ResumablePolicy {
        ResumablePolicy::SinglePartUploading
    }
}
