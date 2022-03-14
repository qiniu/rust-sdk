use super::{DynRead, GetPolicyOptions, ResumablePolicy, ResumablePolicyProvider};
use std::{
    fmt::Debug,
    io::{Read, Result as IoResult},
};

#[cfg(feature = "async")]
use {
    super::DynAsyncRead,
    futures::{future::BoxFuture, AsyncRead},
};

#[derive(Debug, Copy, Clone, Default)]
pub struct AlwaysSinglePart;

impl ResumablePolicyProvider for AlwaysSinglePart {
    #[inline]
    fn get_policy_from_size(&self, _source_size: u64, _opts: &GetPolicyOptions) -> ResumablePolicy {
        ResumablePolicy::SinglePartUploading
    }

    #[inline]
    fn get_policy_from_reader<'a, R: Read + Debug + Send + Sync + 'a>(
        &self,
        reader: R,
        _opts: &GetPolicyOptions,
    ) -> IoResult<(ResumablePolicy, Box<dyn DynRead + 'a>)> {
        Ok((
            ResumablePolicy::SinglePartUploading,
            Box::new(reader) as Box<dyn DynRead>,
        ))
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn get_policy_from_async_reader<'a, R: AsyncRead + Debug + Unpin + Send + Sync + 'a>(
        &self,
        reader: R,
        _opts: &GetPolicyOptions,
    ) -> BoxFuture<'a, IoResult<(ResumablePolicy, Box<dyn DynAsyncRead + 'a>)>> {
        Box::pin(async move {
            Ok((
                ResumablePolicy::SinglePartUploading,
                Box::new(reader) as Box<dyn DynAsyncRead>,
            ))
        })
    }
}
