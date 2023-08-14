use super::{GetPolicyOptions, ResumablePolicy, ResumablePolicyProvider};
use std::fmt::Debug;

/// 固定阀值的可恢复策略
#[derive(Debug, Copy, Clone)]
pub struct FixedThresholdResumablePolicy {
    threshold: u64,
}

impl FixedThresholdResumablePolicy {
    /// 创建固定阀值的可恢复策略
    #[inline]
    pub fn new(threshold: u64) -> Self {
        Self::from(threshold)
    }
}

impl Default for FixedThresholdResumablePolicy {
    #[inline]
    fn default() -> Self {
        Self::from(1 << 22)
    }
}

impl From<u64> for FixedThresholdResumablePolicy {
    #[inline]
    fn from(threshold: u64) -> Self {
        Self { threshold }
    }
}

impl From<FixedThresholdResumablePolicy> for u64 {
    #[inline]
    fn from(policy: FixedThresholdResumablePolicy) -> Self {
        policy.threshold
    }
}

impl ResumablePolicyProvider for FixedThresholdResumablePolicy {
    #[inline]
    fn get_policy(&self, _opts: GetPolicyOptions) -> ResumablePolicy {
        ResumablePolicy::MultiPartsUploadingThreshold(self.threshold)
    }
}
