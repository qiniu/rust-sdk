use super::SourceKey;
use auto_impl::auto_impl;
use digest::Digest;
use dyn_clonable::dyn_clone::{clone_trait_object, DynClone};
use std::{
    fmt::Debug,
    io::{Read, Result as IoResult, Write},
};

#[cfg(feature = "async")]
use futures::{
    future::BoxFuture,
    io::{AsyncRead, AsyncWrite},
};

/// 断点恢复记录器
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait ResumableRecorder: DynClone + Debug + Sync + Send {
    /// 数据源 KEY 的哈希算法
    type HashAlgorithm: Digest;

    /// 根据数据源 KEY 打开只读记录介质
    fn open_for_read(
        &self,
        source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Box<dyn ReadOnlyResumableRecorderMedium>>;

    /// 根据数据源 KEY 打开追加记录介质
    fn open_for_append(
        &self,
        source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Box<dyn AppendOnlyResumableRecorderMedium>>;

    /// 根据数据源 KEY 创建追加记录介质
    fn open_for_create_new(
        &self,
        source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Box<dyn AppendOnlyResumableRecorderMedium>>;

    /// 根据数据源 KEY 删除记录介质
    fn delete(&self, source_key: &SourceKey<Self::HashAlgorithm>) -> IoResult<()>;

    /// 根据数据源 KEY 打开异步只读记录介质
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_read<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Box<dyn ReadOnlyAsyncResumableRecorderMedium>>>;

    /// 根据数据源 KEY 打开异步追加记录介质
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_append<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Box<dyn AppendOnlyAsyncResumableRecorderMedium>>>;

    /// 根据数据源 KEY 创建异步追加记录介质
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_create_new<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Box<dyn AppendOnlyAsyncResumableRecorderMedium>>>;

    /// 根据数据源 KEY 异步删除记录介质
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_delete<'a>(&'a self, source_key: &'a SourceKey<Self::HashAlgorithm>) -> BoxFuture<'a, IoResult<()>>;
}

clone_trait_object!(<H> ResumableRecorder<HashAlgorithm=H> where H: Digest);

/// 只读介质接口
pub trait ReadOnlyResumableRecorderMedium: Read + Debug + Sync + Send {}
impl<T: Read + Debug + Sync + Send> ReadOnlyResumableRecorderMedium for T {}

/// 追加介质接口
pub trait AppendOnlyResumableRecorderMedium: Write + Debug + Sync + Send {}
impl<T: Write + Debug + Sync + Send> AppendOnlyResumableRecorderMedium for T {}

/// 异步只读介质接口
#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub trait ReadOnlyAsyncResumableRecorderMedium: AsyncRead + Unpin + Debug + Sync + Send {}

#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
impl<T: AsyncRead + Unpin + Debug + Sync + Send> ReadOnlyAsyncResumableRecorderMedium for T {}

/// 异步追加介质接口
#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub trait AppendOnlyAsyncResumableRecorderMedium: AsyncWrite + Unpin + Debug + Sync + Send {}

#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
impl<T: AsyncWrite + Unpin + Debug + Sync + Send> AppendOnlyAsyncResumableRecorderMedium for T {}

mod dummy;
mod file;
pub use dummy::{DummyResumableRecorder, DummyResumableRecorderMedium};
pub use file::FileSystemResumableRecorder;
