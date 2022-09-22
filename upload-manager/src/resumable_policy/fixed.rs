use super::{DynRead, GetPolicyOptions, ResumablePolicy, ResumablePolicyProvider};
use std::{
    fmt::Debug,
    io::{Cursor, Read, Result as IoResult},
};

#[cfg(feature = "async")]
use {
    super::DynAsyncRead,
    futures::{future::BoxFuture, io::Cursor as AsyncCursor, AsyncReadExt},
};

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
    fn get_policy_from_size(&self, source_size: u64, _opts: GetPolicyOptions) -> ResumablePolicy {
        get_policy_from_size(self.threshold, source_size)
    }

    fn get_policy_from_reader<'a>(
        &self,
        mut reader: Box<dyn DynRead + 'a>,
        opts: GetPolicyOptions,
    ) -> IoResult<(ResumablePolicy, Box<dyn DynRead + 'a>)> {
        let mut first_chunk = Vec::new();
        (&mut reader).take(self.threshold + 1).read_to_end(&mut first_chunk)?;
        let policy = self.get_policy_from_size(first_chunk.len() as u64, opts);
        Ok((policy, Box::new(Cursor::new(first_chunk).chain(reader))))
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn get_policy_from_async_reader<'a>(
        &self,
        mut reader: Box<dyn DynAsyncRead + 'a>,
        _opts: GetPolicyOptions,
    ) -> BoxFuture<'a, IoResult<(ResumablePolicy, Box<dyn DynAsyncRead + 'a>)>> {
        let threshold = self.threshold;
        Box::pin(async move {
            let mut first_chunk = Vec::new();
            (&mut reader).take(threshold + 1).read_to_end(&mut first_chunk).await?;
            let policy = get_policy_from_size(threshold, first_chunk.len() as u64);
            Ok((
                policy,
                Box::new(AsyncCursor::new(first_chunk).chain(reader)) as Box<dyn DynAsyncRead>,
            ))
        })
    }
}

fn get_policy_from_size(threshold: u64, source_size: u64) -> ResumablePolicy {
    if threshold < source_size {
        ResumablePolicy::MultiPartsUploading
    } else {
        ResumablePolicy::SinglePartUploading
    }
}
