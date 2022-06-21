use super::{super::DataPartitionProvider, DynRead, GetPolicyOptions, ResumablePolicy, ResumablePolicyProvider};
use std::{
    fmt::Debug,
    io::{Cursor, Read, Result as IoResult},
    num::NonZeroU64,
};

#[cfg(feature = "async")]
use {
    super::DynAsyncRead,
    futures::{future::BoxFuture, io::Cursor as AsyncCursor, AsyncReadExt},
};

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
    fn get_policy_from_size(&self, source_size: u64, _opts: GetPolicyOptions) -> ResumablePolicy {
        get_policy_from_size(self.threshold(), source_size)
    }

    fn get_policy_from_reader<'a>(
        &self,
        mut reader: Box<dyn DynRead + 'a>,
        opts: GetPolicyOptions,
    ) -> IoResult<(ResumablePolicy, Box<dyn DynRead + 'a>)> {
        let mut first_chunk = Vec::new();
        (&mut reader).take(self.threshold() + 1).read_to_end(&mut first_chunk)?;
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
        let threshold = self.threshold();
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

impl<P: DataPartitionProvider> MultiplePartitionsResumablePolicyProvider<P> {
    fn threshold(&self) -> u64 {
        self.base_partition_provider.part_size().as_u64() * self.multiply.get()
    }
}

fn get_policy_from_size(threshold: u64, source_size: u64) -> ResumablePolicy {
    if threshold < source_size {
        ResumablePolicy::MultiPartsUploading
    } else {
        ResumablePolicy::SinglePartUploading
    }
}
