use super::PartSize;
use auto_impl::auto_impl;
use digest::OutputSizeUser;
use qiniu_apis::http::Reset;
use std::{
    fmt::Debug,
    io::{Cursor, Read, Result as IoResult},
    num::NonZeroUsize,
};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait DataSource<A: OutputSizeUser>: Debug + Sync + Send {
    fn slice(&self, size: PartSize) -> IoResult<Option<DataSourceReader>>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_slice(&self, size: PartSize) -> BoxFuture<IoResult<Option<AsyncDataSourceReader>>>;

    #[inline]
    fn source_key(&self) -> IoResult<Option<SourceKey<A>>> {
        Ok(None)
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_source_key(&self) -> BoxFuture<IoResult<Option<SourceKey<A>>>> {
        Box::pin(async move { self.source_key() })
    }

    fn total_size(&self) -> IoResult<Option<u64>>;

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_total_size(&self) -> BoxFuture<IoResult<Option<u64>>> {
        Box::pin(async move { self.total_size() })
    }
}

#[derive(Debug)]
pub struct DataSourceReader {
    inner: DataSourceReaderInner,
    part_number: NonZeroUsize,
}

#[derive(Debug)]
enum DataSourceReaderInner {
    ReadSeekable(SeekableSource),
    Readable { data: Cursor<Vec<u8>>, offset: u64 },
}

impl DataSourceReader {
    #[inline]
    pub fn seekable(part_number: NonZeroUsize, source: SeekableSource) -> Self {
        Self {
            inner: DataSourceReaderInner::ReadSeekable(source),
            part_number,
        }
    }

    #[inline]
    pub fn unseekable(part_number: NonZeroUsize, data: Vec<u8>, offset: u64) -> Self {
        Self {
            inner: DataSourceReaderInner::Readable {
                data: Cursor::new(data),
                offset,
            },
            part_number,
        }
    }

    pub(super) fn part_number(&self) -> NonZeroUsize {
        self.part_number
    }

    pub(super) fn offset(&self) -> u64 {
        match &self.inner {
            DataSourceReaderInner::ReadSeekable(source) => source.offset(),
            DataSourceReaderInner::Readable { offset, .. } => *offset,
        }
    }

    pub(super) fn len(&self) -> IoResult<u64> {
        match &self.inner {
            DataSourceReaderInner::ReadSeekable(source) => source.len(),
            DataSourceReaderInner::Readable { data, .. } => Ok(data.get_ref().len() as u64),
        }
    }
}

impl Read for DataSourceReader {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        match &mut self.inner {
            DataSourceReaderInner::ReadSeekable(source) => source.read(buf),
            DataSourceReaderInner::Readable { data, .. } => data.read(buf),
        }
    }
}

impl Reset for DataSourceReader {
    #[inline]
    fn reset(&mut self) -> IoResult<()> {
        match &mut self.inner {
            DataSourceReaderInner::ReadSeekable(source) => source.reset(),
            DataSourceReaderInner::Readable { data, .. } => data.reset(),
        }
    }
}

#[cfg(feature = "async")]
mod async_reader {
    use super::*;
    use futures::{
        io::{Cursor, SeekFrom},
        AsyncRead, AsyncSeek, AsyncSeekExt,
    };
    use qiniu_apis::http::AsyncReset;
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };

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
        #[inline]
        pub fn seekable(part_number: NonZeroUsize, source: AsyncSeekableSource) -> Self {
            Self {
                inner: AsyncDataSourceReaderInner::ReadSeekable(source),
                part_number,
            }
        }

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
                AsyncDataSourceReaderInner::Readable { data, .. } => {
                    Ok(data.get_ref().len() as u64)
                }
            }
        }
    }

    impl AsyncRead for AsyncDataSourceReader {
        #[inline]
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<IoResult<usize>> {
            match &mut self.inner {
                AsyncDataSourceReaderInner::ReadSeekable(source) => {
                    Pin::new(source).poll_read(cx, buf)
                }
                AsyncDataSourceReaderInner::Readable { data, .. } => {
                    Pin::new(data).poll_read(cx, buf)
                }
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
}

#[cfg(feature = "async")]
pub use async_reader::*;

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rand::{thread_rng, RngCore};
    use std::{
        fs::OpenOptions,
        io::{copy as io_copy, Read, Seek, SeekFrom},
        sync::{Arc, Mutex},
        thread::spawn as thread_spawn,
    };
    use tempfile::{Builder as TempfileBuilder, NamedTempFile};

    const FILE_SIZE: u64 = 1 << 26;

    #[test]
    fn test_sync_data_source_reader() -> Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let temp_file = new_temp_file()?;
        {
            let r = OpenOptions::new().read(true).open(temp_file.path())?;
            let mut w = OpenOptions::new().write(true).open(temp_file.path())?;
            w.seek(SeekFrom::End(0))?;

            io_copy(&mut r.take(FILE_SIZE), &mut w)?;
        }

        let s1 = SeekableSource::new(temp_file, 0, FILE_SIZE);
        let s2 = s1.clone_with_new_offset_and_length(FILE_SIZE, FILE_SIZE);
        let mut r1 = DataSourceReader::seekable(NonZeroUsize::new(1).unwrap(), s1);
        let r1_buf = Vec::<u8>::with_capacity(FILE_SIZE as usize);
        let r1_buf = Arc::new(Mutex::new(Cursor::new(r1_buf)));
        let mut r2 = DataSourceReader::seekable(NonZeroUsize::new(2).unwrap(), s2);
        let r2_buf = Vec::<u8>::with_capacity(FILE_SIZE as usize);
        let r2_buf = Arc::new(Mutex::new(Cursor::new(r2_buf)));

        let t1 = thread_spawn({
            let r1_buf = r1_buf.to_owned();
            move || {
                let mut r1_buf = r1_buf.lock().unwrap();
                io_copy(&mut r1, &mut *r1_buf).unwrap()
            }
        });
        let t2 = thread_spawn({
            let r2_buf = r2_buf.to_owned();
            move || {
                let mut r2_buf = r2_buf.lock().unwrap();
                io_copy(&mut r2, &mut *r2_buf).unwrap()
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let r1_buf = Arc::try_unwrap(r1_buf).unwrap().into_inner()?.into_inner();
        let r2_buf = Arc::try_unwrap(r2_buf).unwrap().into_inner()?.into_inner();
        assert_eq!(r1_buf.len(), r2_buf.len());
        assert!(r1_buf == r2_buf);

        Ok(())
    }

    #[cfg(feature = "async")]
    #[async_std::test]
    async fn test_async_data_source_reader() -> Result<()> {
        use async_std::fs::OpenOptions;
        use futures::{
            future::join,
            io::{copy as io_copy, AsyncReadExt, AsyncSeekExt, Cursor},
            lock::Mutex,
        };

        env_logger::builder().is_test(true).try_init().ok();

        let temp_path = new_temp_file()?.into_temp_path();
        let temp_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&*temp_path)
            .await?;
        {
            let r = OpenOptions::new().read(true).open(&*temp_path).await?;
            let mut w = OpenOptions::new().write(true).open(&*temp_path).await?;
            w.seek(SeekFrom::End(0)).await?;

            io_copy(&mut r.take(FILE_SIZE), &mut w).await?;
        }
        let s1 = AsyncSeekableSource::new(temp_file, 0, FILE_SIZE);
        let s2 = s1.clone_with_new_offset_and_length(FILE_SIZE, FILE_SIZE);
        let mut r1 = AsyncDataSourceReader::seekable(NonZeroUsize::new(1).unwrap(), s1);
        let r1_buf = Vec::<u8>::with_capacity(FILE_SIZE as usize);
        let r1_buf = Arc::new(Mutex::new(Cursor::new(r1_buf)));
        let mut r2 = AsyncDataSourceReader::seekable(NonZeroUsize::new(2).unwrap(), s2);
        let r2_buf = Vec::<u8>::with_capacity(FILE_SIZE as usize);
        let r2_buf = Arc::new(Mutex::new(Cursor::new(r2_buf)));

        let f1 = {
            let r1_buf = r1_buf.to_owned();
            async move {
                let mut r1_buf = r1_buf.lock().await;
                io_copy(&mut r1, &mut *r1_buf).await.unwrap()
            }
        };
        let f2 = {
            let r2_buf = r2_buf.to_owned();
            async move {
                let mut r2_buf = r2_buf.lock().await;
                io_copy(&mut r2, &mut *r2_buf).await.unwrap()
            }
        };
        join(f1, f2).await;

        let r1_buf = Arc::try_unwrap(r1_buf).unwrap().into_inner().into_inner();
        let r2_buf = Arc::try_unwrap(r2_buf).unwrap().into_inner().into_inner();
        assert_eq!(r1_buf.len(), r2_buf.len());
        assert!(r1_buf == r2_buf);

        Ok(())
    }

    fn new_temp_file() -> Result<NamedTempFile> {
        let mut temp_file = TempfileBuilder::new().tempfile()?;
        let rng = &mut thread_rng() as &mut dyn RngCore;
        io_copy(&mut rng.take(FILE_SIZE), &mut temp_file)?;
        temp_file.seek(SeekFrom::Start(0))?;
        Ok(temp_file)
    }
}

mod source_key;
pub use source_key::SourceKey;

mod file;
pub use file::FileDataSource;

mod seekable;
pub use seekable::{SeekableDataSource, SeekableSource};

mod unseekable;
pub use unseekable::UnseekableDataSource;

#[cfg(feature = "async")]
pub use {seekable::AsyncSeekableSource, unseekable::AsyncUnseekableDataSource};
