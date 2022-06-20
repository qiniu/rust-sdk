use super::{DataPartitionProvider, DataPartitionProviderFeedback, PartSize};
use std::num::NonZeroU64;

/// 受限的分片大小提供者
///
/// 基于一个分片大小提供者实例，如果提供的分片大小在限制范围外，则调整到限制范围内。
#[derive(Debug, Clone, Copy)]
pub struct LimitedDataPartitionProvider<P: ?Sized> {
    min: NonZeroU64,
    max: NonZeroU64,
    base: P,
}

impl<P: DataPartitionProvider> LimitedDataPartitionProvider<P> {
    /// 创建受限的分片大小提供者
    ///
    /// 如果传入 `0` 作为 `min` 或 `max` 将返回 [`None`]。
    #[inline]
    pub fn new(base: P, min: u64, max: u64) -> Option<Self> {
        match (NonZeroU64::new(min), NonZeroU64::new(max)) {
            (Some(min), Some(max)) => Some(Self::new_with_non_zero_threshold(base, min, max)),
            _ => None,
        }
    }

    /// 创建受限的分片大小提供者
    ///
    /// 提供 [`NonZeroU64`] 作为分片大小类型。
    #[inline]
    pub fn new_with_non_zero_threshold(base: P, min: NonZeroU64, max: NonZeroU64) -> Self {
        Self { base, min, max }
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
    /// 获得分片大小下限
    #[inline]
    pub const fn min_part_size(&self) -> NonZeroU64 {
        self.min
    }

    /// 获得分片大小上限
    #[inline]
    pub const fn max_part_size(&self) -> NonZeroU64 {
        self.max
    }
}

impl<P: DataPartitionProvider + Clone> DataPartitionProvider for LimitedDataPartitionProvider<P> {
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
