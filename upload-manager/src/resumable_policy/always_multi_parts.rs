use super::{DynRead, GetPolicyOptions, ResumablePolicy, ResumablePolicyProvider};
use std::{fmt::Debug, io::Result as IoResult};

#[cfg(feature = "async")]
use {super::DynAsyncRead, futures::future::BoxFuture};

/// 总是选择分片上传
#[derive(Debug, Copy, Clone, Default)]
pub struct AlwaysMultiParts;

impl ResumablePolicyProvider for AlwaysMultiParts {
    #[inline]
    fn get_policy_from_size(&self, _source_size: u64, _opts: GetPolicyOptions) -> ResumablePolicy {
        ResumablePolicy::MultiPartsUploading
    }

    #[inline]
    fn get_policy_from_reader<'a>(
        &self,
        reader: Box<dyn DynRead + 'a>,
        _opts: GetPolicyOptions,
    ) -> IoResult<(ResumablePolicy, Box<dyn DynRead + 'a>)> {
        Ok((ResumablePolicy::MultiPartsUploading, reader))
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn get_policy_from_async_reader<'a>(
        &self,
        reader: Box<dyn DynAsyncRead + 'a>,
        _opts: GetPolicyOptions,
    ) -> BoxFuture<'a, IoResult<(ResumablePolicy, Box<dyn DynAsyncRead + 'a>)>> {
        Box::pin(async move { Ok((ResumablePolicy::MultiPartsUploading, reader)) })
    }
}
