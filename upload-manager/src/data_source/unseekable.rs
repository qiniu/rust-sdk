use super::{DataSource, DataSourceReader, SourceKey};
use std::{
    fmt::Debug,
    io::{Read, Result as IoResult},
    sync::Mutex,
};

#[cfg(feature = "async")]
use {
    super::AsyncDataSourceReader,
    futures::{future::BoxFuture, AsyncRead},
};

#[derive(Debug)]
pub(crate) struct UnseekableDataSource<R>(Mutex<UnseekableDataSourceInner<R>>);

#[derive(Debug)]
struct UnseekableDataSourceInner<R> {
    reader: R,
    current_offset: u64,
    source_key: Option<SourceKey>,
}

impl<R> UnseekableDataSource<R> {
    pub(crate) fn new(reader: R) -> Self {
        Self(Mutex::new(UnseekableDataSourceInner {
            reader,
            current_offset: 0,
            source_key: None,
        }))
    }

    pub(crate) fn new_with_source_key(reader: R, source_key: SourceKey) -> Self {
        Self(Mutex::new(UnseekableDataSourceInner {
            reader,
            source_key: Some(source_key),
            current_offset: 0,
        }))
    }
}

impl<R: Read + Debug + Send + Sync + 'static> DataSource for UnseekableDataSource<R> {
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
    fn source_key(&self) -> IoResult<Option<SourceKey>> {
        Ok(self.0.lock().unwrap().source_key.to_owned())
    }

    #[inline]
    fn into_read(self) -> IoResult<Box<dyn Read + Send + Sync>> {
        Ok(Box::new(self.0.into_inner().unwrap().reader) as Box<dyn Read + Send + Sync>)
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn into_async_read(
        self,
    ) -> BoxFuture<'static, IoResult<Box<dyn AsyncRead + Unpin + Send + Sync>>> {
        unimplemented!()
    }
}

#[cfg(feature = "async")]
mod async_unseekable {
    use super::*;
    use futures::{lock::Mutex, AsyncRead, AsyncReadExt};

    #[derive(Debug)]
    pub(crate) struct AsyncUnseekableDataSource<R>(Mutex<AsyncUnseekableDataSourceInner<R>>);

    #[derive(Debug)]
    struct AsyncUnseekableDataSourceInner<R> {
        reader: R,
        current_offset: u64,
        source_key: Option<SourceKey>,
    }

    impl<R> AsyncUnseekableDataSource<R> {
        pub(crate) fn new(reader: R) -> Self {
            Self(Mutex::new(AsyncUnseekableDataSourceInner {
                reader,
                current_offset: 0,
                source_key: None,
            }))
        }

        pub(crate) fn new_with_source_key(reader: R, source_key: SourceKey) -> Self {
            Self(Mutex::new(AsyncUnseekableDataSourceInner {
                reader,
                source_key: Some(source_key),
                current_offset: 0,
            }))
        }
    }

    impl<R: AsyncRead + Debug + Unpin + Send + Sync + 'static> DataSource
        for AsyncUnseekableDataSource<R>
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
        fn source_key(&self) -> IoResult<Option<SourceKey>> {
            unimplemented!()
        }

        #[inline]
        fn async_source_key(&self) -> BoxFuture<IoResult<Option<SourceKey>>> {
            Box::pin(async move { Ok(self.0.lock().await.source_key.to_owned()) })
        }

        fn into_read(self) -> IoResult<Box<dyn Read + Send + Sync>> {
            unimplemented!()
        }

        fn into_async_read(
            self,
        ) -> BoxFuture<'static, IoResult<Box<dyn AsyncRead + Unpin + Send + Sync>>> {
            Box::pin(async move {
                Ok(Box::new(self.0.into_inner().reader)
                    as Box<dyn AsyncRead + Unpin + Send + Sync>)
            })
        }
    }
}

#[cfg(feature = "async")]
pub(crate) use async_unseekable::*;
