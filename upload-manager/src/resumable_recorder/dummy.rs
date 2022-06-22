use super::{AppendOnlyResumableRecorderMedium, ReadOnlyResumableRecorderMedium, ResumableRecorder, SourceKey};
use digest::Digest;
use sha1::Sha1;
use std::{
    fmt::{self, Debug},
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult, Write},
    marker::PhantomData,
};

#[cfg(feature = "async")]
use {
    super::{AppendOnlyAsyncResumableRecorderMedium, ReadOnlyAsyncResumableRecorderMedium},
    futures::future::BoxFuture,
};

/// 无断点恢复记录器
///
/// 实现了断点恢复记录器接口，但总是返回找不到记录
#[derive(Clone, Copy)]
pub struct DummyResumableRecorder<O = Sha1> {
    _unused: PhantomData<O>,
}

impl<O> DummyResumableRecorder<O> {
    /// 创建无断点恢复记录器
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }
}

impl<O> Default for DummyResumableRecorder<O> {
    #[inline]
    fn default() -> Self {
        Self {
            _unused: Default::default(),
        }
    }
}

impl<O: Clone + Digest + Send + Sync + Unpin> ResumableRecorder for DummyResumableRecorder<O> {
    type HashAlgorithm = O;

    #[inline]
    fn open_for_read(
        &self,
        _source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Box<dyn ReadOnlyResumableRecorderMedium>> {
        Err(make_error())
    }

    #[inline]
    fn open_for_append(
        &self,
        _source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Box<dyn AppendOnlyResumableRecorderMedium>> {
        Err(make_error())
    }

    #[inline]
    fn open_for_create_new(
        &self,
        _source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Box<dyn AppendOnlyResumableRecorderMedium>> {
        Err(make_error())
    }

    #[inline]
    fn delete(&self, _source_key: &SourceKey<Self::HashAlgorithm>) -> IoResult<()> {
        Err(make_error())
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_read<'a>(
        &'a self,
        _source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Box<dyn ReadOnlyAsyncResumableRecorderMedium>>> {
        Box::pin(async move { Err(make_error()) })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_append<'a>(
        &'a self,
        _source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Box<dyn AppendOnlyAsyncResumableRecorderMedium>>> {
        Box::pin(async move { Err(make_error()) })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_create_new<'a>(
        &'a self,
        _source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Box<dyn AppendOnlyAsyncResumableRecorderMedium>>> {
        Box::pin(async move { Err(make_error()) })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_delete<'a>(&'a self, _source_key: &'a SourceKey<Self::HashAlgorithm>) -> BoxFuture<'a, IoResult<()>> {
        Box::pin(async move { Err(make_error()) })
    }
}

impl<O> Debug for DummyResumableRecorder<O> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DummyResumableRecorder").finish()
    }
}

/// 无断点恢复记录介质
///
/// 实现了断点恢复记录介质接口，但总是返回错误
#[derive(Debug, Clone, Copy)]
pub struct DummyResumableRecorderMedium;

impl Read for DummyResumableRecorderMedium {
    #[inline]
    fn read(&mut self, _buf: &mut [u8]) -> IoResult<usize> {
        Err(make_error())
    }
}

impl Write for DummyResumableRecorderMedium {
    #[inline]
    fn write(&mut self, _buf: &[u8]) -> IoResult<usize> {
        Err(make_error())
    }

    #[inline]
    fn flush(&mut self) -> IoResult<()> {
        Err(make_error())
    }
}

#[cfg(feature = "async")]
use {
    futures::{AsyncRead, AsyncWrite},
    std::{
        pin::Pin,
        task::{Context, Poll},
    },
};

#[cfg(feature = "async")]
impl AsyncRead for DummyResumableRecorderMedium {
    #[inline]
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, _buf: &mut [u8]) -> Poll<IoResult<usize>> {
        Poll::Ready(Err(make_error()))
    }
}

#[cfg(feature = "async")]
impl AsyncWrite for DummyResumableRecorderMedium {
    #[inline]
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, _buf: &[u8]) -> Poll<IoResult<usize>> {
        Poll::Ready(Err(make_error()))
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Err(make_error()))
    }

    #[inline]
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Poll::Ready(Err(make_error()))
    }
}

fn make_error() -> IoError {
    IoError::new(IoErrorKind::Unsupported, "Unimplemented")
}
