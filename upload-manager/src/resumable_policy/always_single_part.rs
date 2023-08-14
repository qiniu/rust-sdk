use super::{GetPolicyOptions, ResumablePolicy, ResumablePolicyProvider};
use std::fmt::Debug;

/// 总是选择单请求上传
#[derive(Debug, Copy, Clone, Default)]
pub struct AlwaysSinglePart;

impl ResumablePolicyProvider for AlwaysSinglePart {
    #[inline]
    fn get_policy(&self, _opts: GetPolicyOptions) -> ResumablePolicy {
        ResumablePolicy::SinglePartUploading
    }
}
