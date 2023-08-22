use super::{super::PartSize, async_seekable::AsyncSeekableSource, SourceKey};
use auto_impl::auto_impl;
use digest::{Digest, Output as DigestOutput};
use dyn_clonable::clonable;
use futures::{
    future::BoxFuture,
    io::{copy as async_io_copy, sink as async_sink, Cursor, SeekFrom},
    AsyncRead, AsyncSeek, AsyncSeekExt,
};
use qiniu_apis::http::AsyncReset;
use std::{
    fmt::Debug,
    io::Result as IoResult,
    num::NonZeroUsize,
    pin::Pin,
    task::{ready, Context, Poll},
};

/// 异步数据源接口
///
/// 提供上传所用的数据源
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait AsyncDataSource<A: Digest>: Clone + Debug + Sync + Send {
    /// 异步数据源切片
    fn slice(&self, size: PartSize) -> BoxFuture<IoResult<Option<AsyncDataSourceReader>>>;

    /// 异步重置数据源
    fn reset(&self) -> BoxFuture<IoResult<()>>;

    /// 异步获取数据源 KEY
    ///
    /// 用于区分不同的数据源
    fn source_key(&self) -> BoxFuture<IoResult<Option<SourceKey<A>>>>;

    /// 异步获取数据源大小
    fn total_size(&self) -> BoxFuture<IoResult<Option<u64>>>;
}

pub(crate) trait AsyncDigestible<A: Digest + Unpin + Send>: AsyncRead + AsyncReset + Unpin + Send {
    fn digest(&mut self) -> BoxFuture<IoResult<DigestOutput<A>>> {
        struct ReadWithDigest<A, R> {
            reader: R,
            digest: A,
        }

        impl<A: Digest + Unpin + Send, R: AsyncRead + Unpin> AsyncRead for ReadWithDigest<A, R> {
            fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<IoResult<usize>> {
                let size = ready!(Pin::new(&mut self.reader).poll_read(cx, buf))?;
                self.digest.update(buf);
                Poll::Ready(Ok(size))
            }
        }

        Box::pin(async move {
            let mut hasher = ReadWithDigest {
                reader: self,
                digest: A::new(),
            };
            async_io_copy(Pin::new(&mut hasher), &mut async_sink()).await?;
            hasher.reader.reset().await?;
            Ok(hasher.digest.finalize())
        })
    }
}

impl<T: AsyncRead + AsyncReset + Unpin + Send, A: Digest + Unpin + Send> AsyncDigestible<A> for T {}

/// 异步数据源阅读器
///
/// 提供异步读取接口
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
#[derive(Debug)]
pub struct AsyncDataSourceReader {
    inner: AsyncDataSourceReaderInner,
    part_number: NonZeroUsize,
}

#[derive(Debug)]
enum AsyncDataSourceReaderInner {
    ReadSeekable(AsyncSeekableSource),
    Readable { data: Cursor<Vec<u8>>, offset: u64 },
}

impl AsyncDataSourceReader {
    /// 创建可寻址的异步数据源阅读器
    #[inline]
    pub fn seekable(part_number: NonZeroUsize, source: AsyncSeekableSource) -> Self {
        Self {
            inner: AsyncDataSourceReaderInner::ReadSeekable(source),
            part_number,
        }
    }

    /// 创建不可寻址的异步数据源阅读器
    #[inline]
    pub fn unseekable(part_number: NonZeroUsize, data: Vec<u8>, offset: u64) -> Self {
        Self {
            inner: AsyncDataSourceReaderInner::Readable {
                data: Cursor::new(data),
                offset,
            },
            part_number,
        }
    }

    pub(in super::super) fn part_number(&self) -> NonZeroUsize {
        self.part_number
    }

    pub(in super::super) fn offset(&self) -> u64 {
        match &self.inner {
            AsyncDataSourceReaderInner::ReadSeekable(source) => source.offset(),
            AsyncDataSourceReaderInner::Readable { offset, .. } => *offset,
        }
    }

    pub(in super::super) async fn len(&self) -> IoResult<u64> {
        match &self.inner {
            AsyncDataSourceReaderInner::ReadSeekable(source) => source.len().await,
            AsyncDataSourceReaderInner::Readable { data, .. } => Ok(data.get_ref().len() as u64),
        }
    }
}

impl AsyncRead for AsyncDataSourceReader {
    #[inline]
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<IoResult<usize>> {
        match &mut self.inner {
            AsyncDataSourceReaderInner::ReadSeekable(source) => Pin::new(source).poll_read(cx, buf),
            AsyncDataSourceReaderInner::Readable { data, .. } => Pin::new(data).poll_read(cx, buf),
        }
    }
}

impl AsyncReset for AsyncDataSourceReader {
    #[inline]
    fn reset(&mut self) -> BoxFuture<IoResult<()>> {
        match &mut self.inner {
            AsyncDataSourceReaderInner::ReadSeekable(source) => source.reset(),
            AsyncDataSourceReaderInner::Readable { data, .. } => Box::pin(async move {
                data.seek(SeekFrom::Start(0)).await?;
                Ok(())
            }),
        }
    }
}

trait ReadSeek: AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin {}
impl<T: AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin> ReadSeek for T {}
