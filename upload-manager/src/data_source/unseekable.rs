use super::{DataSource, DataSourceReader, SourceKey};
use std::{
    fmt::{self, Debug},
    io::{Read, Result as IoResult},
    sync::Mutex,
};

#[cfg(feature = "async")]
use {super::AsyncDataSourceReader, futures::future::BoxFuture};

pub struct UnseekableDataSource<R: Read + Debug + Send + Sync + 'static + ?Sized, A: OutputSizeUser>(
    Mutex<UnseekableDataSourceInner<R, A>>,
);

impl<R: Read + Debug + Send + Sync + 'static, A: OutputSizeUser> Debug
    for UnseekableDataSource<R, A>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UnseekableDataSource")
            .field(&self.0)
            .finish()
    }
}

struct UnseekableDataSourceInner<
    R: Read + Debug + Send + Sync + 'static + ?Sized,
    A: OutputSizeUser,
> {
    current_offset: u64,
    source_key: Option<SourceKey<A>>,
    reader: R,
}

impl<R: Read + Debug + Send + Sync + 'static, A: OutputSizeUser> UnseekableDataSource<R, A> {
    pub fn new(reader: R) -> Self {
        Self(Mutex::new(UnseekableDataSourceInner {
            reader,
            current_offset: 0,
            source_key: None,
        }))
    }

    pub fn new_with_source_key(reader: R, source_key: SourceKey<A>) -> Self {
        Self(Mutex::new(UnseekableDataSourceInner {
            reader,
            source_key: Some(source_key),
            current_offset: 0,
        }))
    }
}

impl<R: Read + Debug + Send + Sync + 'static, A: OutputSizeUser> DataSource<A>
    for UnseekableDataSource<R, A>
{
    fn slice(&self, size: u64) -> IoResult<Option<DataSourceReader>> {
        let mut buf = Vec::new();
        let guard = &mut *self.0.lock().unwrap();
        let have_read = (&mut guard.reader).take(size).read_to_end(&mut buf)?;
        if have_read > 0 {
            let source_reader = DataSourceReader::unseekable(buf, guard.current_offset);
            guard.current_offset += have_read as u64;
            Ok(Some(source_reader))
        } else {
            Ok(None)
        }
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_slice(&self, _size: u64) -> BoxFuture<IoResult<Option<AsyncDataSourceReader>>> {
        unimplemented!()
    }

    #[inline]
    fn source_key(&self) -> IoResult<Option<SourceKey<A>>> {
        Ok(self.0.lock().unwrap().source_key.to_owned())
    }
}

impl<R: Read + Debug + Send + Sync + 'static, A: OutputSizeUser> Debug
    for UnseekableDataSourceInner<R, A>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnseekableDataSourceInner")
            .field("reader", &self.reader)
            .field("current_offset", &self.current_offset)
            .field("source_key", &self.source_key)
            .finish()
    }
}

#[cfg(feature = "async")]
mod async_unseekable {
    use super::*;
    use futures::{lock::Mutex, AsyncRead, AsyncReadExt};

    pub struct AsyncUnseekableDataSource<
        R: AsyncRead + Debug + Unpin + Send + Sync + 'static + ?Sized,
        A: OutputSizeUser,
    >(Mutex<AsyncUnseekableDataSourceInner<R, A>>);

    impl<R: AsyncRead + Debug + Unpin + Send + Sync + 'static, A: OutputSizeUser> Debug
        for AsyncUnseekableDataSource<R, A>
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_tuple("AsyncUnseekableDataSource")
                .field(&self.0)
                .finish()
        }
    }

    struct AsyncUnseekableDataSourceInner<
        R: AsyncRead + Debug + Unpin + Send + Sync + 'static + ?Sized,
        A: OutputSizeUser,
    > {
        current_offset: u64,
        source_key: Option<SourceKey<A>>,
        reader: R,
    }

    impl<R: AsyncRead + Debug + Unpin + Send + Sync + 'static, A: OutputSizeUser>
        AsyncUnseekableDataSource<R, A>
    {
        pub fn new(reader: R) -> Self {
            Self(Mutex::new(AsyncUnseekableDataSourceInner {
                reader,
                current_offset: 0,
                source_key: None,
            }))
        }

        pub fn new_with_source_key(reader: R, source_key: SourceKey<A>) -> Self {
            Self(Mutex::new(AsyncUnseekableDataSourceInner {
                reader,
                source_key: Some(source_key),
                current_offset: 0,
            }))
        }
    }

    impl<R: AsyncRead + Debug + Unpin + Send + Sync + 'static, A: OutputSizeUser> DataSource<A>
        for AsyncUnseekableDataSource<R, A>
    {
        fn slice(&self, _size: u64) -> IoResult<Option<DataSourceReader>> {
            unimplemented!()
        }

        fn async_slice(&self, size: u64) -> BoxFuture<IoResult<Option<AsyncDataSourceReader>>> {
            Box::pin(async move {
                let mut buf = Vec::new();
                let guard = &mut *self.0.lock().await;
                let have_read = (&mut guard.reader).take(size).read_to_end(&mut buf).await?;
                if have_read > 0 {
                    let source_reader =
                        AsyncDataSourceReader::unseekable(buf, guard.current_offset);
                    guard.current_offset += have_read as u64;
                    Ok(Some(source_reader))
                } else {
                    Ok(None)
                }
            })
        }

        #[inline]
        fn source_key(&self) -> IoResult<Option<SourceKey<A>>> {
            unimplemented!()
        }

        #[inline]
        fn async_source_key(&self) -> BoxFuture<IoResult<Option<SourceKey<A>>>> {
            Box::pin(async move { Ok(self.0.lock().await.source_key.to_owned()) })
        }
    }

    impl<R: AsyncRead + Debug + Unpin + Send + Sync + 'static, A: OutputSizeUser> Debug
        for AsyncUnseekableDataSourceInner<R, A>
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("AsyncUnseekableDataSourceInner")
                .field("reader", &self.reader)
                .field("current_offset", &self.current_offset)
                .field("source_key", &self.source_key)
                .finish()
        }
    }
}

#[cfg(feature = "async")]
pub use async_unseekable::*;
use sha1::digest::OutputSizeUser;
