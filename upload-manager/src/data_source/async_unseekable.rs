use super::{
    super::{AsyncDataSource, AsyncDataSourceReader, PartSize},
    reader::first_part_number,
    unseekable::unsupported_reset_error,
    SourceKey,
};
use futures::{
    future::{self, BoxFuture},
    lock::Mutex,
    AsyncRead, AsyncReadExt,
};
use sha1::{digest::Digest, Sha1};
use std::{
    fmt::{self, Debug},
    io::Result as IoResult,
    num::NonZeroUsize,
    sync::Arc,
};

/// 不可寻址的异步数据源
///
/// 基于一个不可寻址的异步阅读器实现了异步数据源接口
pub struct AsyncUnseekableDataSource<R: AsyncRead + Debug + Unpin + Send + Sync + 'static + ?Sized, A: Digest = Sha1>(
    Arc<Mutex<AsyncUnseekableDataSourceInner<R, A>>>,
);

impl<R: AsyncRead + Debug + Unpin + Send + Sync + 'static, A: Digest> Debug for AsyncUnseekableDataSource<R, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AsyncUnseekableDataSource").field(&self.0).finish()
    }
}

impl<R: AsyncRead + Debug + Unpin + Send + Sync + 'static, A: Digest> Clone for AsyncUnseekableDataSource<R, A> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

struct AsyncUnseekableDataSourceInner<R: AsyncRead + Debug + Unpin + Send + Sync + 'static + ?Sized, A: Digest> {
    current_offset: u64,
    current_part_number: NonZeroUsize,
    source_key: Option<SourceKey<A>>,
    reader: R,
}

impl<R: AsyncRead + Debug + Unpin + Send + Sync + 'static, A: Digest> AsyncUnseekableDataSource<R, A> {
    /// 创建不可寻址的异步数据源
    pub fn new(reader: R) -> Self {
        Self(Arc::new(Mutex::new(AsyncUnseekableDataSourceInner {
            reader,
            current_offset: 0,
            current_part_number: first_part_number(),
            source_key: None,
        })))
    }
}

impl<R: AsyncRead + Debug + Unpin + Send + Sync + 'static, A: Digest> AsyncDataSource<A>
    for AsyncUnseekableDataSource<R, A>
{
    fn slice(&self, size: PartSize) -> BoxFuture<IoResult<Option<AsyncDataSourceReader>>> {
        Box::pin(async move {
            let mut buf = Vec::new();
            let guard = &mut self.0.lock().await;
            let have_read = (&mut guard.reader).take(size.as_u64()).read_to_end(&mut buf).await?;
            if have_read > 0 {
                let source_reader =
                    AsyncDataSourceReader::unseekable(guard.current_part_number, buf, guard.current_offset);
                guard.current_offset += have_read as u64;
                guard.current_part_number =
                    NonZeroUsize::new(guard.current_part_number.get() + 1).expect("Page number is too big");
                Ok(Some(source_reader))
            } else {
                Ok(None)
            }
        })
    }

    #[inline]
    fn reset(&self) -> BoxFuture<IoResult<()>> {
        Box::pin(async move { Err(unsupported_reset_error()) })
    }

    #[inline]
    fn source_key(&self) -> BoxFuture<IoResult<Option<SourceKey<A>>>> {
        Box::pin(async move { Ok(self.0.lock().await.source_key.to_owned()) })
    }

    #[inline]
    fn total_size(&self) -> BoxFuture<IoResult<Option<u64>>> {
        Box::pin(future::ok(None))
    }
}

impl<R: AsyncRead + Debug + Unpin + Send + Sync + 'static, A: Digest> Debug for AsyncUnseekableDataSourceInner<R, A> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncUnseekableDataSourceInner")
            .field("reader", &self.reader)
            .field("current_offset", &self.current_offset)
            .field("current_part_number", &self.current_part_number)
            .field("source_key", &self.source_key)
            .finish()
    }
}
