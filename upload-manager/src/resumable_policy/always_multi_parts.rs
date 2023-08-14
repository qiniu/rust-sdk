use super::{GetPolicyOptions, ResumablePolicy, ResumablePolicyProvider};
use std::fmt::Debug;

/// 总是选择分片上传
#[derive(Debug, Copy, Clone, Default)]
pub struct AlwaysMultiParts;

impl ResumablePolicyProvider for AlwaysMultiParts {
    #[inline]
    fn get_policy(&self, _opts: GetPolicyOptions) -> ResumablePolicy {
        ResumablePolicy::MultiPartsUploading
    }
}
