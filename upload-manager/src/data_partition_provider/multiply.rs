use super::{DataPartitionProvider, DataPartitionProviderFeedback, PartSize};
use std::num::NonZeroU64;

/// 整数倍分片大小提供者
///
/// 基于一个分片大小提供者实例，如果提供的分片大小不是指定倍数的整数倍，则下调到它的整数倍
#[derive(Debug, Clone, Copy)]
pub struct MultiplyDataPartitionProvider<P: ?Sized> {
    multiply: NonZeroU64,
    base: P,
}

impl<P: DataPartitionProvider> MultiplyDataPartitionProvider<P> {
    /// 创建整数倍分片大小提供者
    ///
    /// 如果传入 `0` 将返回 [`None`]。
    #[inline]
    pub fn new(base: P, multiply: u64) -> Option<Self> {
        NonZeroU64::new(multiply).map(|multiply| Self::new_with_non_zero_multiply(base, multiply))
    }

    /// 创建整数倍分片大小提供者
    ///
    /// 提供 [`NonZeroU64`] 作为分片大小类型。
    #[inline]
    pub fn new_with_non_zero_multiply(base: P, multiply: NonZeroU64) -> Self {
        Self { base, multiply }
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
    /// 获取倍数
    #[inline]
    pub const fn multiply(&self) -> NonZeroU64 {
        self.multiply
    }
}

impl<P: DataPartitionProvider + Clone> DataPartitionProvider for MultiplyDataPartitionProvider<P> {
    #[inline]
    fn part_size(&self) -> PartSize {
        let base_partition = self.base.part_size().as_non_zero_u64();
        let multiply = self.multiply.get();
        let partition = base_partition.max(self.multiply).get() / multiply * multiply;
        NonZeroU64::new(partition).unwrap().into()
    }

    #[inline]
    fn feedback(&self, feedback: DataPartitionProviderFeedback<'_>) {
        self.base.feedback(feedback)
    }
}
