use sha1::{
    digest::{generic_array::GenericArray, OutputSizeUser},
    Sha1,
};
use std::{
    fmt::Debug,
    io::{Cursor, Read, Result as IoResult, Seek, SeekFrom},
    sync::{Arc, Mutex},
};

#[cfg(feature = "async")]
use futures::{future::BoxFuture, AsyncRead};

pub(super) trait DataSource: Debug + Sync + Send {
    fn slice(&self, size: u64) -> IoResult<Option<DataSourceReader>>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_slice(&self, size: u64) -> BoxFuture<IoResult<Option<AsyncDataSourceReader>>>;

    fn into_read(self) -> IoResult<Box<dyn Read + Send + Sync>>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn into_async_read(
        self,
    ) -> BoxFuture<'static, IoResult<Box<dyn AsyncRead + Unpin + Send + Sync>>>;

    #[inline]
    fn source_key(&self) -> IoResult<Option<SourceKey>> {
        Ok(None)
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_source_key(&self) -> BoxFuture<IoResult<Option<SourceKey>>> {
        Box::pin(async move { self.source_key() })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SourceKey(GenericArray<u8, <Sha1 as OutputSizeUser>::OutputSize>);

impl SourceKey {
    pub fn new(array: impl Into<GenericArray<u8, <Sha1 as OutputSizeUser>::OutputSize>>) -> Self {
        Self::from(array.into())
    }
}

impl From<GenericArray<u8, <Sha1 as OutputSizeUser>::OutputSize>> for SourceKey {
    #[inline]
    fn from(array: GenericArray<u8, <Sha1 as OutputSizeUser>::OutputSize>) -> Self {
        Self(array)
    }
}

#[derive(Debug)]
pub(super) struct DataSourceReader(DataSourceReaderInner);

#[derive(Debug)]
enum DataSourceReaderInner {
    ReadSeekable(SeekableSource),
    Readable { data: Cursor<Vec<u8>>, offset: u64 },
}

impl DataSourceReader {
    pub(super) fn seekable(source: SeekableSource) -> Self {
        Self(DataSourceReaderInner::ReadSeekable(source))
    }

    pub(super) fn unseekable(data: Vec<u8>, offset: u64) -> Self {
        Self(DataSourceReaderInner::Readable {
            data: Cursor::new(data),
            offset,
        })
    }

    pub(super) fn offset(&self) -> u64 {
        match &self.0 {
            DataSourceReaderInner::ReadSeekable(source) => source.offset(),
            DataSourceReaderInner::Readable { offset, .. } => *offset,
        }
    }

    pub(super) fn len(&self) -> u64 {
        match &self.0 {
            DataSourceReaderInner::ReadSeekable(source) => source.len(),
            DataSourceReaderInner::Readable { data, .. } => data.get_ref().len() as u64,
        }
    }
}

impl Read for DataSourceReader {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        match &mut self.0 {
            DataSourceReaderInner::ReadSeekable(source) => source.read(buf),
            DataSourceReaderInner::Readable { data, .. } => data.read(buf),
        }
    }
}

#[derive(Debug)]
struct SeekableSourceInner<T: Read + Seek + Send + Sync + Debug + ?Sized> {
    pos: Option<u64>,
    source: T,
}

impl<T: Read + Seek + Send + Sync + Debug> SeekableSourceInner<T> {
    fn new(source: T) -> Self {
        Self { source, pos: None }
    }
}

#[derive(Debug, Clone)]
pub(super) struct SeekableSource {
    source: Arc<Mutex<SeekableSourceInner<dyn ReadSeek>>>,
    source_offset: u64,
    offset: u64,
    len: u64,
}

impl SeekableSource {
    pub(super) fn new(
        source: impl Read + Seek + Debug + Send + Sync + 'static,
        offset: u64,
        len: u64,
    ) -> Self {
        Self {
            source: Arc::new(Mutex::new(SeekableSourceInner::new(source))),
            source_offset: 0,
            offset,
            len,
        }
    }

    pub(super) fn clone_with_new_offset_and_length(&self, offset: u64, len: u64) -> Self {
        let mut cloned = self.to_owned();
        cloned.source_offset = 0;
        cloned.offset = offset;
        cloned.len = len;
        cloned
    }

    fn offset(&self) -> u64 {
        self.offset
    }

    fn len(&self) -> u64 {
        self.len
    }
}

impl Read for SeekableSource {
    fn read(&mut self, mut buf: &mut [u8]) -> IoResult<usize> {
        let mut locked = self.source.lock().unwrap();
        let max_read = self.len - self.source_offset;
        if max_read == 0 {
            return Ok(0);
        } else if max_read < buf.len() as u64 {
            let max_read: usize = max_read.try_into().unwrap_or(usize::MAX);
            buf = &mut buf[..max_read];
        }
        let seek_pos = self.offset + self.source_offset;
        if Some(seek_pos) != locked.pos {
            locked.pos = Some(locked.source.seek(SeekFrom::Start(seek_pos))?);
        }
        let have_read = locked.source.read(buf)?;
        self.source_offset += have_read as u64;
        if let Some(ref mut pos) = locked.pos {
            *pos += have_read as u64;
        }
        Ok(have_read)
    }
}

trait ReadSeek: Read + Seek + Send + Sync + Debug {}
impl<T: Read + Seek + Send + Sync + Debug> ReadSeek for T {}

#[cfg(feature = "async")]
mod async_reader {
    use super::*;
    use futures::{
        future::FutureExt, io::Cursor, lock::Mutex, ready, AsyncRead, AsyncReadExt, AsyncSeek,
        AsyncSeekExt, Future,
    };
    use smart_default::SmartDefault;
    use std::{
        fmt,
        pin::Pin,
        sync::atomic::{AtomicU64, Ordering::Relaxed},
        task::{Context, Poll},
    };

    #[derive(Debug)]
    pub(in super::super) struct AsyncSeekableSource {
        source: Arc<Mutex<AsyncSeekableSourceInner<dyn ReadSeek>>>,
        source_offset: Arc<AtomicU64>,
        offset: u64,
        len: u64,
        step: AsyncSeekableSourceReadStep,
    }

    #[derive(Debug)]
    struct AsyncSeekableSourceInner<T: AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin + ?Sized> {
        pos: Option<u64>,
        source: T,
    }

    impl<T: AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin> AsyncSeekableSourceInner<T> {
        fn new(source: T) -> Self {
            Self { source, pos: None }
        }
    }

    #[derive(SmartDefault)]
    enum AsyncSeekableSourceReadStep {
        #[default]
        Buffered {
            buffer: Vec<u8>,
            consumed: usize,
        },
        Waiting {
            task: Pin<Box<dyn Future<Output = IoResult<Vec<u8>>> + Send + Sync + 'static>>,
        },
        Done,
    }

    impl Debug for AsyncSeekableSourceReadStep {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Buffered { buffer, consumed } => f
                    .debug_struct("Buffered")
                    .field("buffer", buffer)
                    .field("consumed", consumed)
                    .finish(),
                Self::Waiting { .. } => f.debug_struct("Waiting").finish(),
                Self::Done => write!(f, "Done"),
            }
        }
    }

    impl AsyncSeekableSource {
        pub(super) fn new(
            source: impl AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin + 'static,
            offset: u64,
            len: u64,
        ) -> Self {
            Self {
                step: Default::default(),
                source: Arc::new(Mutex::new(AsyncSeekableSourceInner::new(source))),
                source_offset: Arc::new(AtomicU64::new(0)),
                offset,
                len,
            }
        }

        pub(super) fn clone_with_new_offset_and_length(&self, offset: u64, len: u64) -> Self {
            Self {
                step: Default::default(),
                source: self.source.to_owned(),
                source_offset: Arc::new(AtomicU64::new(0)),
                offset,
                len,
            }
        }

        fn offset(&self) -> u64 {
            self.offset
        }

        fn len(&self) -> u64 {
            self.len
        }

        fn poll_from_task(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<IoResult<usize>> {
            match &mut self.step {
                AsyncSeekableSourceReadStep::Waiting { task } => {
                    let buffer = ready!(task.poll_unpin(cx))?;
                    self.step = if buffer.is_empty() {
                        AsyncSeekableSourceReadStep::Done
                    } else {
                        AsyncSeekableSourceReadStep::Buffered {
                            buffer,
                            consumed: 0,
                        }
                    };
                    self.poll_read(cx, buf)
                }
                _ => unreachable!(),
            }
        }

        fn poll_from_buffer(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<IoResult<usize>> {
            match &mut self.step {
                AsyncSeekableSourceReadStep::Buffered { buffer, consumed } => {
                    let rested = buf.len().min(buffer.len() - *consumed);
                    if rested > 0 {
                        buf[..rested].copy_from_slice(&buffer[*consumed..(*consumed + rested)]);
                        *consumed += rested;
                        Poll::Ready(Ok(rested))
                    } else {
                        let buffer_request_size = buf.len().max(1 << 22);
                        let source = self.source.to_owned();
                        let source_offset = self.source_offset.to_owned();
                        let len = self.len;
                        let offset = self.offset;
                        self.step = AsyncSeekableSourceReadStep::Waiting {
                            task: Box::pin(async move {
                                let mut locked = source.lock().await;
                                let source_offset_value = source_offset.load(Relaxed);
                                let max_read = len - source_offset_value;
                                if max_read == 0 {
                                    Ok(Vec::new())
                                } else {
                                    let max_read: usize = max_read.try_into().unwrap_or(usize::MAX);
                                    let mut buffer = vec![0u8; buffer_request_size.min(max_read)];
                                    let seek_pos = offset + source_offset_value;
                                    if Some(seek_pos) != locked.pos {
                                        locked.pos = Some(
                                            locked.source.seek(SeekFrom::Start(seek_pos)).await?,
                                        );
                                    }
                                    let have_read = locked.source.read(&mut buffer).await?;
                                    buffer.truncate(have_read);
                                    let have_read = have_read as u64;
                                    source_offset.fetch_add(have_read, Relaxed);
                                    if let Some(ref mut pos) = locked.pos {
                                        *pos += have_read;
                                    }
                                    Ok(buffer)
                                }
                            }),
                        };
                        self.poll_read(cx, buf)
                    }
                }
                _ => unreachable!(),
            }
        }

        fn poll_done(self: Pin<&mut Self>) -> Poll<IoResult<usize>> {
            match &self.step {
                AsyncSeekableSourceReadStep::Done => Poll::Ready(Ok(0)),
                _ => unreachable!(),
            }
        }
    }

    impl AsyncRead for AsyncSeekableSource {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<IoResult<usize>> {
            match self.step {
                AsyncSeekableSourceReadStep::Waiting { .. } => self.poll_from_task(cx, buf),
                AsyncSeekableSourceReadStep::Buffered { .. } => self.poll_from_buffer(cx, buf),
                AsyncSeekableSourceReadStep::Done => self.poll_done(),
            }
        }
    }

    #[derive(Debug)]
    pub(in super::super) struct AsyncDataSourceReader(AsyncDataSourceReaderInner);

    #[derive(Debug)]
    enum AsyncDataSourceReaderInner {
        ReadSeekable(AsyncSeekableSource),
        Readable { data: Cursor<Vec<u8>>, offset: u64 },
    }

    impl AsyncDataSourceReader {
        pub(in super::super) fn seekable(source: AsyncSeekableSource) -> Self {
            Self(AsyncDataSourceReaderInner::ReadSeekable(source))
        }

        pub(in super::super) fn unseekable(data: Vec<u8>, offset: u64) -> Self {
            Self(AsyncDataSourceReaderInner::Readable {
                data: Cursor::new(data),
                offset,
            })
        }

        pub(in super::super) fn offset(&self) -> u64 {
            match &self.0 {
                AsyncDataSourceReaderInner::ReadSeekable(source) => source.offset(),
                AsyncDataSourceReaderInner::Readable { offset, .. } => *offset,
            }
        }

        pub(in super::super) fn len(&self) -> u64 {
            match &self.0 {
                AsyncDataSourceReaderInner::ReadSeekable(source) => source.len(),
                AsyncDataSourceReaderInner::Readable { data, .. } => data.get_ref().len() as u64,
            }
        }
    }

    impl AsyncRead for AsyncDataSourceReader {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<IoResult<usize>> {
            match &mut self.0 {
                AsyncDataSourceReaderInner::ReadSeekable(source) => {
                    Pin::new(source).poll_read(cx, buf)
                }
                AsyncDataSourceReaderInner::Readable { data, .. } => {
                    Pin::new(data).poll_read(cx, buf)
                }
            }
        }
    }

    trait ReadSeek: AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin {}
    impl<T: AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin> ReadSeek for T {}
}

#[cfg(feature = "async")]
pub(super) use async_reader::*;

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rand::{thread_rng, RngCore};
    use std::{
        fs::OpenOptions,
        io::{copy as io_copy, Read},
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
        let mut r1 = DataSourceReader::seekable(s1);
        let r1_buf = Vec::<u8>::with_capacity(FILE_SIZE as usize);
        let r1_buf = Arc::new(Mutex::new(Cursor::new(r1_buf)));
        let mut r2 = DataSourceReader::seekable(s2);
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
        let mut r1 = AsyncDataSourceReader::seekable(s1);
        let r1_buf = Vec::<u8>::with_capacity(FILE_SIZE as usize);
        let r1_buf = Arc::new(Mutex::new(Cursor::new(r1_buf)));
        let mut r2 = AsyncDataSourceReader::seekable(s2);
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
        let rng = Box::new(thread_rng()) as Box<dyn RngCore>;
        io_copy(&mut rng.take(FILE_SIZE), &mut temp_file)?;
        temp_file.seek(SeekFrom::Start(0))?;
        Ok(temp_file)
    }
}

mod file;
pub(super) use file::FileDataSource;

mod unseekable;
pub(super) use unseekable::UnseekableDataSource;

#[cfg(feature = "async")]
pub(super) use unseekable::AsyncUnseekableDataSource;
