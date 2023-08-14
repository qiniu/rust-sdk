use auto_impl::auto_impl;
use dyn_clonable::clonable;
use std::fmt::Debug;

/// 可恢复策略
///
/// 选择使用单请求上传或分片上传
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum ResumablePolicy {
    /// 分片上传
    MultiPartsUploading,

    /// 单请求上传
    SinglePartUploading,

    /// 分片上传阈值
    MultiPartsUploadingThreshold(u64),
}

/// 可恢复策略获取接口
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait ResumablePolicyProvider: Clone + Debug + Sync + Send {
    /// 获取可恢复策略
    fn get_policy(&self, opts: GetPolicyOptions) -> ResumablePolicy;
}

/// 获取可恢复策略的选项
#[derive(Debug, Copy, Clone, Default)]
pub struct GetPolicyOptions {}

mod always_single_part;
pub use always_single_part::AlwaysSinglePart;

mod always_multi_parts;
pub use always_multi_parts::AlwaysMultiParts;

mod fixed;
pub use fixed::FixedThresholdResumablePolicy;

mod multiple_partitions;
pub use multiple_partitions::MultiplePartitionsResumablePolicyProvider;
