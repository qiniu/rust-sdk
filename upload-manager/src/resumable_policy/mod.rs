use auto_impl::auto_impl;
use dyn_clonable::clonable;
use std::{
    fmt::Debug,
    io::{Read, Result as IoResult},
};

#[cfg(feature = "async")]
use futures::{future::BoxFuture, AsyncRead};

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
}

/// 可恢复策略获取接口
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait ResumablePolicyProvider: Clone + Debug + Sync + Send {
    /// 通过数据源大小获取可恢复策略
    fn get_policy_from_size(&self, source_size: u64, opts: GetPolicyOptions) -> ResumablePolicy;

    /// 通过输入流获取可恢复策略
    ///
    /// 返回选择的可恢复策略，以及经过更新的输入流
    ///
    /// 该方法的异步版本为 [`Self::get_policy_from_async_reader`]。
    fn get_policy_from_reader<'a>(
        &self,
        reader: Box<dyn DynRead + 'a>,
        opts: GetPolicyOptions,
    ) -> IoResult<(ResumablePolicy, Box<dyn DynRead + 'a>)>;

    /// 通过异步输入流获取可恢复策略
    ///
    /// 返回选择的可恢复策略，以及经过更新的异步输入流
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn get_policy_from_async_reader<'a>(
        &self,
        reader: Box<dyn DynAsyncRead + 'a>,
        opts: GetPolicyOptions,
    ) -> BoxFuture<'a, IoResult<(ResumablePolicy, Box<dyn DynAsyncRead + 'a>)>>;
}

/// 获取可恢复策略的选项
#[derive(Debug, Copy, Clone, Default)]
pub struct GetPolicyOptions {}

/// 阻塞输入流
pub trait DynRead: Read + Debug + Send + Sync {}
impl<T: Read + Debug + Send + Sync> DynRead for T {}

/// 异步输入流
#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub trait DynAsyncRead: AsyncRead + Debug + Unpin + Send + Sync {}

#[cfg(feature = "async")]
impl<T: AsyncRead + Debug + Unpin + Send + Sync> DynAsyncRead for T {}

mod always_single_part;
pub use always_single_part::AlwaysSinglePart;

mod always_multi_parts;
pub use always_multi_parts::AlwaysMultiParts;

mod fixed;
pub use fixed::FixedThresholdResumablePolicy;

mod multiple_partitions;
pub use multiple_partitions::MultiplePartitionsResumablePolicyProvider;
