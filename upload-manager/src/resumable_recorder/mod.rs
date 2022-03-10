use super::SourceKey;
use auto_impl::auto_impl;
use digest::Digest;
use std::{
    fmt::Debug,
    io::{Read, Result as IoResult, Write},
};

#[cfg(feature = "async")]
use futures::{
    future::BoxFuture,
    io::{AsyncRead, AsyncWrite},
};

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait ResumableRecorder: Debug + Sync + Send {
    type HashAlgorithm: Digest;
    type ReadOnlyMedium: ReadOnlyResumableRecorderMedium;
    type AppendOnlyMedium: AppendOnlyResumableRecorderMedium;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    type AsyncReadOnlyMedium: ReadOnlyAsyncResumableRecorderMedium;
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    type AsyncAppendOnlyMedium: AppendOnlyAsyncResumableRecorderMedium;

    fn open_for_read(
        &self,
        source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Self::ReadOnlyMedium>;
    fn open_for_append(
        &self,
        source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Self::AppendOnlyMedium>;
    fn open_for_create_new(
        &self,
        source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Self::AppendOnlyMedium>;
    fn delete(&self, source_key: &SourceKey<Self::HashAlgorithm>) -> IoResult<()>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_read<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Self::AsyncReadOnlyMedium>>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_append<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Self::AsyncAppendOnlyMedium>>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_create_new<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Self::AsyncAppendOnlyMedium>>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_delete<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<()>>;
}

pub trait ReadOnlyResumableRecorderMedium: Read + Debug + Sync + Send {}
impl<T: Read + Debug + Sync + Send> ReadOnlyResumableRecorderMedium for T {}

pub trait AppendOnlyResumableRecorderMedium: Write + Debug + Sync + Send {}
impl<T: Write + Debug + Sync + Send> AppendOnlyResumableRecorderMedium for T {}

#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub trait ReadOnlyAsyncResumableRecorderMedium: AsyncRead + Unpin + Debug + Sync + Send {}

#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
impl<T: AsyncRead + Unpin + Debug + Sync + Send> ReadOnlyAsyncResumableRecorderMedium for T {}

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
