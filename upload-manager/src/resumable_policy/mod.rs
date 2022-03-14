use auto_impl::auto_impl;
use std::{
    fmt::Debug,
    io::{Read, Result as IoResult},
};

#[cfg(feature = "async")]
use futures::{future::BoxFuture, AsyncRead};

#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum ResumablePolicy {
    MultiPartsUploading,
    SinglePartUploading,
}

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait ResumablePolicyProvider: Debug + Sync + Send {
    fn get_policy_from_size(&self, source_size: u64, opts: &GetPolicyOptions) -> ResumablePolicy;
    fn get_policy_from_reader<'a, R: Read + Debug + Send + Sync + 'a>(
        &self,
        reader: R,
        opts: &GetPolicyOptions,
    ) -> IoResult<(ResumablePolicy, Box<dyn DynRead + 'a>)>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn get_policy_from_async_reader<'a, R: AsyncRead + Debug + Unpin + Send + Sync + 'a>(
        &self,
        reader: R,
        opts: &GetPolicyOptions,
    ) -> BoxFuture<'a, IoResult<(ResumablePolicy, Box<dyn DynAsyncRead + 'a>)>>;
}

#[derive(Debug, Clone, Default)]
pub struct GetPolicyOptions {}

pub trait DynRead: Read + Debug + Send + Sync {}
impl<T: Read + Debug + Send + Sync> DynRead for T {}

#[cfg(feature = "async")]
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
