use super::SourceKey;
use auto_impl::auto_impl;
use digest::OutputSizeUser;
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
    type HashAlgorithm: OutputSizeUser;
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

pub trait ReadOnlyResumableRecorderMedium: Read + Debug + Sync + Send {
    type AppendOnlyMedium: AppendOnlyResumableRecorderMedium;
    fn into_medium_for_append(self) -> IoResult<Self::AppendOnlyMedium>;
    fn into_medium_for_create_new(self) -> IoResult<Self::AppendOnlyMedium>;
}

pub trait AppendOnlyResumableRecorderMedium: Write + Debug + Sync + Send {
    type ReadOnlyMedium: ReadOnlyResumableRecorderMedium;
    fn into_medium_for_read(self) -> IoResult<Self::ReadOnlyMedium>;
}

#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub trait ReadOnlyAsyncResumableRecorderMedium: AsyncRead + Unpin + Debug + Sync + Send {
    type AppendOnlyMedium: AppendOnlyAsyncResumableRecorderMedium;
    fn into_medium_for_append(self) -> BoxFuture<'static, IoResult<Self::AppendOnlyMedium>>;
    fn into_medium_for_create_new(self) -> BoxFuture<'static, IoResult<Self::AppendOnlyMedium>>;
}

#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub trait AppendOnlyAsyncResumableRecorderMedium: AsyncWrite + Unpin + Debug + Sync + Send {
    type ReadOnlyMedium: ReadOnlyAsyncResumableRecorderMedium;
    fn into_medium_for_read(self) -> BoxFuture<'static, IoResult<Self::ReadOnlyMedium>>;
}

mod dummy;
mod file;
pub use dummy::{DummyResumableRecorder, DummyResumableRecorderMedium};
pub use file::{
    FileSystemAppendOnlyResumableRecorderMedium, FileSystemReadOnlyResumableRecorderMedium,
    FileSystemResumableRecorder,
};

#[cfg(feature = "async")]
pub use file::{
    FileSystemAppendOnlyAsyncResumableRecorderMedium,
    FileSystemReadOnlyAsyncResumableRecorderMedium,
};
