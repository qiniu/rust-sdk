use super::{super::DataPartitionProvider, GetPolicyOptions, ResumablePolicy, ResumablePolicyProvider};
use std::{fmt::Debug, num::NonZeroU64};

/// 整数倍分片大小的可恢复策略
///
/// 在数据源大小超过分片大小提供者返回的分片大小的整数倍时，将使用分片上传。
#[derive(Debug, Clone)]
pub struct MultiplePartitionsResumablePolicyProvider<P: ?Sized> {
    multiply: NonZeroU64,
    base_partition_provider: P,
}

impl<P> MultiplePartitionsResumablePolicyProvider<P> {
    /// 创建整数倍分片大小的可恢复策略
    ///
    /// 如果传入 `0` 则返回 [`None`]。
    #[inline]
    pub fn new(base_partition_provider: P, multiply: u64) -> Option<Self> {
        NonZeroU64::new(multiply).map(|multiply| Self::new_with_non_zero_multiply(base_partition_provider, multiply))
    }

    /// 创建整数倍分片大小的可恢复策略
    ///
    /// 提供 [`NonZeroU64`] 作为分片大小类型。
    #[inline]
    pub fn new_with_non_zero_multiply(base_partition_provider: P, multiply: NonZeroU64) -> Self {
        Self {
            base_partition_provider,
            multiply,
        }
    }
}

impl<P: DataPartitionProvider + Clone> ResumablePolicyProvider for MultiplePartitionsResumablePolicyProvider<P> {
    #[inline]
    fn get_policy(&self, _opts: GetPolicyOptions) -> ResumablePolicy {
        ResumablePolicy::MultiPartsUploadingThreshold(self.threshold())
    }
}

impl<P: DataPartitionProvider> MultiplePartitionsResumablePolicyProvider<P> {
    fn threshold(&self) -> u64 {
        self.base_partition_provider.part_size().as_u64() * self.multiply.get()
    }
}
