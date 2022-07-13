use super::{DataSource, DataSourceReader, PartSize, SourceKey};
use sha1::{digest::Digest, Sha1};
use std::{
    fmt::{self, Debug},
    io::{Read, Result as IoResult},
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

/// 不可寻址的数据源
///
/// 基于一个不可寻址的阅读器实现了数据源接口
pub struct UnseekableDataSource<R: Read + Debug + Send + Sync + 'static + ?Sized, A: Digest = Sha1>(
    Arc<Mutex<UnseekableDataSourceInner<R, A>>>,
);

impl<R: Read + Debug + Send + Sync + 'static, A: Digest> Debug for UnseekableDataSource<R, A> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UnseekableDataSource").field(&self.0).finish()
    }
}

impl<R: Read + Debug + Send + Sync + 'static, A: Digest> Clone for UnseekableDataSource<R, A> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

struct UnseekableDataSourceInner<R: Read + Debug + Send + Sync + 'static + ?Sized, A: Digest> {
    current_offset: u64,
    current_part_number: NonZeroUsize,
    source_key: Option<SourceKey<A>>,
    reader: R,
}

impl<R: Read + Debug + Send + Sync + 'static, A: Digest> UnseekableDataSource<R, A> {
    /// 创建不可寻址的数据源
    #[inline]
    pub fn new(reader: R) -> Self {
        Self(Arc::new(Mutex::new(UnseekableDataSourceInner {
            reader,
            current_offset: 0,
            #[allow(unsafe_code)]
            current_part_number: unsafe { NonZeroUsize::new_unchecked(1) },
            source_key: None,
        })))
    }
}

impl<R: Read + Debug + Send + Sync + 'static, A: Digest> DataSource<A> for UnseekableDataSource<R, A> {
    fn slice(&self, size: PartSize) -> IoResult<Option<DataSourceReader>> {
        let mut buf = Vec::new();
        let guard = &mut self.0.lock().unwrap();
        let have_read = (&mut guard.reader).take(size.as_u64()).read_to_end(&mut buf)?;
        if have_read > 0 {
            let source_reader = DataSourceReader::unseekable(guard.current_part_number, buf, guard.current_offset);
            guard.current_offset += have_read as u64;
            guard.current_part_number =
                NonZeroUsize::new(guard.current_part_number.get() + 1).expect("Page number is too big");
            Ok(Some(source_reader))
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn source_key(&self) -> IoResult<Option<SourceKey<A>>> {
        Ok(self.0.lock().unwrap().source_key.to_owned())
    }

    #[inline]
    fn total_size(&self) -> IoResult<Option<u64>> {
        Ok(None)
    }
}

impl<R: Read + Debug + Send + Sync + 'static, A: Digest> Debug for UnseekableDataSourceInner<R, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnseekableDataSourceInner")
            .field("reader", &self.reader)
            .field("current_offset", &self.current_offset)
            .field("current_part_number", &self.current_part_number)
            .field("source_key", &self.source_key)
            .finish()
    }
}

#[cfg(feature = "async")]
mod async_unseekable {
    use super::{
        super::{AsyncDataSource, AsyncDataSourceReader},
        *,
    };
    use futures::{
        future::{self, BoxFuture},
        lock::Mutex,
        AsyncRead, AsyncReadExt,
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
                #[allow(unsafe_code)]
                current_part_number: unsafe { NonZeroUsize::new_unchecked(1) },
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
}

#[cfg(feature = "async")]
pub use async_unseekable::*;
