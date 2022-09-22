use auto_impl::auto_impl;
use dyn_clonable::clonable;
use qiniu_apis::{http::Extensions, http_client::ResponseError};
use std::{
    fmt::Debug,
    num::NonZeroU64,
    ops::{Deref, DerefMut},
    time::Duration,
};

/// 分片大小获取接口
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait DataPartitionProvider: Clone + Debug + Sync + Send {
    /// 获取分片大小
    fn part_size(&self) -> PartSize;
    /// 反馈分片大小结果
    fn feedback(&self, feedback: DataPartitionProviderFeedback<'_>);
}

/// 分片大小
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartSize(NonZeroU64);

impl PartSize {
    /// 创建分片大小
    ///
    /// 如果传入 `0` 将返回 [`None`]。
    pub fn new(part_size: u64) -> Option<Self> {
        NonZeroU64::new(part_size).map(Self)
    }

    /// 创建分片大小
    ///
    /// 提供 [`NonZeroU64`] 作为并发数类型。
    #[inline]
    pub const fn new_with_non_zero_u64(concurrency: NonZeroU64) -> Self {
        Self(concurrency)
    }

    /// 获取并发数
    ///
    /// 返回 [`NonZeroU64`] 作为并发数类型。
    #[inline]
    pub fn as_non_zero_u64(&self) -> NonZeroU64 {
        self.0
    }

    /// 获取并发数
    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.as_non_zero_u64().get()
    }
}

impl Default for PartSize {
    #[inline]
    fn default() -> Self {
        Self(
            #[allow(unsafe_code)]
            unsafe {
                NonZeroU64::new_unchecked(1 << 22)
            },
        )
    }
}

impl From<NonZeroU64> for PartSize {
    #[inline]
    fn from(size: NonZeroU64) -> Self {
        Self(size)
    }
}

impl From<PartSize> for NonZeroU64 {
    #[inline]
    fn from(size: PartSize) -> Self {
        size.as_non_zero_u64()
    }
}

impl From<PartSize> for u64 {
    #[inline]
    fn from(size: PartSize) -> Self {
        size.as_u64()
    }
}

impl Deref for PartSize {
    type Target = NonZeroU64;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PartSize {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// 分片大小提供者反馈
///
/// 反馈给提供者分片的效果，包含对象大小，花费时间，以及错误信息。
#[derive(Debug, Clone)]
pub struct DataPartitionProviderFeedback<'f> {
    part_size: PartSize,
    elapsed: Duration,
    extensions: &'f Extensions,
    error: Option<&'f ResponseError>,
}

impl<'f> DataPartitionProviderFeedback<'f> {
    /// 创建分片大小提供者反馈构建器
    #[inline]
    pub fn builder(
        part_size: PartSize,
        elapsed: Duration,
        extensions: &'f Extensions,
    ) -> DataPartitionProviderFeedbackBuilder<'f> {
        DataPartitionProviderFeedbackBuilder::new(part_size, elapsed, extensions)
    }

    /// 获取分片大小
    #[inline]
    pub fn part_size(&self) -> PartSize {
        self.part_size
    }

    /// 获取花费时间
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// 获取扩展信息
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        self.extensions
    }

    /// 获取错误信息
    #[inline]
    pub fn error(&self) -> Option<&'f ResponseError> {
        self.error
    }
}

/// 分片大小提供者反馈构建器
#[derive(Debug, Clone)]
pub struct DataPartitionProviderFeedbackBuilder<'f>(DataPartitionProviderFeedback<'f>);

impl<'f> DataPartitionProviderFeedbackBuilder<'f> {
    /// 创建分片大小提供者反馈构建器
    #[inline]
    pub fn new(part_size: PartSize, elapsed: Duration, extensions: &'f Extensions) -> Self {
        Self(DataPartitionProviderFeedback {
            part_size,
            elapsed,
            extensions,
            error: None,
        })
    }

    /// 设置错误信息
    #[inline]
    pub fn error(&mut self, err: &'f ResponseError) -> &mut Self {
        self.0.error = Some(err);
        self
    }

    /// 构建分片大小提供者反馈
    #[inline]
    pub fn build(&self) -> DataPartitionProviderFeedback<'f> {
        self.0.to_owned()
    }
}

mod fixed;
pub use fixed::FixedDataPartitionProvider;

mod limited;
pub use limited::LimitedDataPartitionProvider;

mod multiply;
pub use multiply::MultiplyDataPartitionProvider;
