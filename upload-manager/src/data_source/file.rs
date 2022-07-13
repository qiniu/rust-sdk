use super::{seekable::SeekableDataSource, DataSource, DataSourceReader, PartSize, SourceKey, UnseekableDataSource};
use digest::Digest;
use once_cell::sync::OnceCell;
use os_str_bytes::OsStrBytes;
use sha1::Sha1;
use std::{
    fmt::{self, Debug},
    fs::File,
    io::Result as IoResult,
    path::PathBuf,
    sync::Arc,
};

#[cfg(feature = "async")]
use {
    super::{AsyncDataSourceReader, AsyncSeekableSource, AsyncUnseekableDataSource},
    async_once_cell::OnceCell as AsyncOnceCell,
    async_std::{fs::File as AsyncFile, path::PathBuf as AsyncPathBuf},
    futures::{future::BoxFuture, lock::Mutex as AsyncMutex, AsyncSeekExt},
    std::num::NonZeroUsize,
};

enum Source<A: Digest> {
    Seekable(SeekableDataSource),
    Unseekable(UnseekableDataSource<File, A>),
}

impl<A: Digest> Debug for Source<A> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Seekable(source) => f.debug_struct("Seekable").field("source", source).finish(),
            Self::Unseekable(source) => f.debug_tuple("Unseekable").field(source).finish(),
        }
    }
}

/// 文件数据源
///
/// 基于一个文件实现了数据源接口
pub struct FileDataSource<A: Digest = Sha1> {
    path: PathBuf,
    canonicalized_path: Arc<OnceCell<PathBuf>>,
    source: Arc<OnceCell<Source<A>>>,
}

impl<A: Digest> FileDataSource<A> {
    /// 创建文件数据源
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            canonicalized_path: Default::default(),
            source: Default::default(),
        }
    }

    fn get_seekable_source(&self) -> IoResult<&Source<A>> {
        self.source.get_or_try_init(|| {
            let file = File::open(&self.path)?;
            let file_size = file.metadata()?.len();
            SeekableDataSource::new(file, file_size)
                .map(Source::Seekable)
                .or_else(|_| {
                    File::open(&self.path)
                        .map(UnseekableDataSource::new)
                        .map(Source::Unseekable)
                })
        })
    }

    fn get_path(&self) -> IoResult<&PathBuf> {
        self.canonicalized_path.get_or_try_init(|| self.path.canonicalize())
    }
}

impl<D: Digest + Send> DataSource<D> for FileDataSource<D> {
    fn slice(&self, size: PartSize) -> IoResult<Option<DataSourceReader>> {
        match self.get_seekable_source()? {
            Source::Seekable(source) => DataSource::<D>::slice(source, size),
            Source::Unseekable(source) => source.slice(size),
        }
    }

    fn source_key(&self) -> IoResult<Option<SourceKey<D>>> {
        match self.get_seekable_source()? {
            Source::Seekable { .. } => {
                let mut hasher = D::new();
                hasher.update(b"file://");
                hasher.update(&self.get_path()?.to_raw_bytes());
                Ok(Some(hasher.finalize().into()))
            }
            Source::Unseekable(source) => source.source_key(),
        }
    }

    fn total_size(&self) -> IoResult<Option<u64>> {
        match self.get_seekable_source()? {
            Source::Seekable(source) => DataSource::<D>::total_size(source),
            Source::Unseekable(source) => source.total_size(),
        }
    }
}

impl<A: Digest> Debug for FileDataSource<A> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileDataSource")
            .field("path", &self.path)
            .field("canonicalized_path", &self.canonicalized_path)
            .field("source", &self.source)
            .finish()
    }
}

impl<A: Digest> Clone for FileDataSource<A> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            canonicalized_path: self.canonicalized_path.clone(),
            source: self.source.clone(),
        }
    }
}

#[cfg(feature = "async")]
mod async_reader {
    use super::{super::AsyncDataSource, *};

    /// 异步文件数据源
    ///
    /// 基于一个文件实现了数据源接口
    pub struct AsyncFileDataSource<A: Digest> {
        path: PathBuf,
        canonicalized_path: Arc<AsyncOnceCell<AsyncPathBuf>>,
        source: Arc<AsyncOnceCell<AsyncSource<A>>>,
    }

    impl<A: Digest> AsyncFileDataSource<A> {
        /// 创建文件数据源
        pub fn new(path: impl Into<PathBuf>) -> Self {
            Self {
                path: path.into(),
                canonicalized_path: Arc::new(AsyncOnceCell::new()),
                source: Arc::new(AsyncOnceCell::new()),
            }
        }

        async fn get_async_seekable_source(&self) -> IoResult<&AsyncSource<A>> {
            self.source
                .get_or_try_init(async {
                    let mut file = AsyncFile::open(&self.path).await?;
                    if let Ok(offset) = file.stream_position().await {
                        Ok(AsyncSource::Seekable {
                            file_size: file.metadata().await?.len(),
                            source: AsyncSeekableSource::new(file, 0, 0),
                            current: AsyncMutex::new(SourceOffset {
                                offset,
                                #[allow(unsafe_code)]
                                part_number: unsafe { NonZeroUsize::new_unchecked(1) },
                            }),
                        })
                    } else {
                        Ok(AsyncSource::Unseekable(AsyncUnseekableDataSource::new(file)))
                    }
                })
                .await
        }

        async fn get_async_path(&self) -> IoResult<&AsyncPathBuf> {
            self.canonicalized_path
                .get_or_try_init(async { AsyncPathBuf::from(&self.path).canonicalize().await })
                .await
        }
    }

    impl<A: Digest + Send> AsyncDataSource<A> for AsyncFileDataSource<A> {
        fn slice(&self, size: PartSize) -> BoxFuture<IoResult<Option<AsyncDataSourceReader>>> {
            Box::pin(async move {
                match self.get_async_seekable_source().await? {
                    AsyncSource::Seekable {
                        source,
                        current,
                        file_size,
                    } => {
                        let mut cur = current.lock().await;
                        if cur.offset < *file_size {
                            let size = size.as_u64();
                            let source_reader = AsyncDataSourceReader::seekable(
                                cur.part_number,
                                source.clone_with_new_offset_and_length(cur.offset, size),
                            );
                            cur.offset += size;
                            cur.part_number =
                                NonZeroUsize::new(cur.part_number.get() + 1).expect("Page number is too big");
                            Ok(Some(source_reader))
                        } else {
                            Ok(None)
                        }
                    }
                    AsyncSource::Unseekable(source) => source.slice(size).await,
                }
            })
        }

        fn source_key(&self) -> BoxFuture<IoResult<Option<SourceKey<A>>>> {
            Box::pin(async move {
                match self.get_async_seekable_source().await? {
                    AsyncSource::Seekable { .. } => {
                        let mut hasher = A::new();
                        hasher.update(b"file://");
                        hasher.update(&self.get_async_path().await?.as_os_str().to_raw_bytes());
                        Ok(Some(hasher.finalize().into()))
                    }
                    AsyncSource::Unseekable(source) => source.source_key().await,
                }
            })
        }

        fn total_size(&self) -> BoxFuture<IoResult<Option<u64>>> {
            Box::pin(async move {
                match self.get_async_seekable_source().await? {
                    AsyncSource::Seekable { file_size, .. } => Ok(Some(*file_size)),
                    AsyncSource::Unseekable(source) => source.total_size().await,
                }
            })
        }
    }

    impl<A: Digest> Debug for AsyncFileDataSource<A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("AsyncFileDataSource")
                .field("path", &self.path)
                .field("canonicalized_path", &self.canonicalized_path)
                .field("source", &self.source)
                .finish()
        }
    }

    impl<A: Digest> Clone for AsyncFileDataSource<A> {
        #[inline]
        fn clone(&self) -> Self {
            Self {
                path: self.path.clone(),
                canonicalized_path: self.canonicalized_path.clone(),
                source: self.source.clone(),
            }
        }
    }

    #[derive(Debug)]
    struct SourceOffset {
        offset: u64,
        part_number: NonZeroUsize,
    }

    enum AsyncSource<A: Digest> {
        Seekable {
            source: AsyncSeekableSource,
            current: AsyncMutex<SourceOffset>,
            file_size: u64,
        },
        Unseekable(AsyncUnseekableDataSource<AsyncFile, A>),
    }

    impl<A: Digest> Debug for AsyncSource<A> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Seekable {
                    source,
                    current,
                    file_size,
                } => f
                    .debug_struct("Seekable")
                    .field("source", source)
                    .field("current", current)
                    .field("file_size", file_size)
                    .finish(),
                Self::Unseekable(file) => f.debug_tuple("Unseekable").field(file).finish(),
            }
        }
    }
}

#[cfg(feature = "async")]
pub use async_reader::*;

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::{
        io::{Read, Write},
        thread::{spawn as thread_spawn, JoinHandle as ThreadJoinHandle},
    };
    use tempfile::Builder as TempfileBuilder;

    #[test]
    fn test_sync_seekable_file_data_source() -> Result<()> {
        let temp_file_path = {
            let mut temp_file = TempfileBuilder::new().tempfile()?;
            for i in 0..255u8 {
                let buf = vec![i; 1 << 20];
                temp_file.write_all(&buf)?;
            }
            temp_file.into_temp_path()
        };
        let data_source = FileDataSource::<Sha1>::new(&temp_file_path);
        let mut source_readers = Vec::with_capacity(256);
        for _ in 0..255u8 {
            source_readers.push(data_source.slice(PartSize::new(1 << 20).unwrap())?.unwrap());
        }
        assert!(data_source.slice(PartSize::new(1 << 20).unwrap())?.is_none());

        let mut threads: Vec<ThreadJoinHandle<IoResult<()>>> = Vec::with_capacity(256);
        for (i, mut source_reader) in source_readers.into_iter().enumerate() {
            threads.push(thread_spawn(move || {
                let mut buf = Vec::new();
                let have_read = source_reader.read_to_end(&mut buf)?;
                assert_eq!(have_read, 1 << 20);
                assert_eq!(buf, vec![i as u8; 1 << 20]);
                Ok(())
            }));
        }
        for thread in threads {
            thread.join().unwrap()?;
        }

        Ok(())
    }

    #[test]
    #[cfg(target_os = "unix")]
    fn test_sync_unseekable_file_data_source() -> Result<()> {
        use defer_lite::defer;
        use ipipe::Pipe;
        use std::fs::remove_file;

        let mut pipe = Pipe::create()?;
        let pipe_path = pipe.path().to_owned();
        defer! {
            remove_file(&pipe_path).ok();
        }
        let producer_thread: ThreadJoinHandle<IoResult<()>> = {
            thread_spawn(move || {
                for i in 0..255u8 {
                    let buf = vec![i; 1 << 20];
                    pipe.write_all(&buf)?;
                }
                pipe.close()?;
                Ok(())
            })
        };
        let data_source = FileDataSource::new(&pipe_path);
        let mut source_readers = Vec::with_capacity(256);
        for _ in 0..255u8 {
            source_readers.push(data_source.slice(1 << 20)?.unwrap());
        }
        assert!(data_source.slice(1 << 20)?.is_none());

        let mut threads: Vec<ThreadJoinHandle<IoResult<()>>> = Vec::with_capacity(257);
        for (i, mut source_reader) in source_readers.into_iter().enumerate() {
            threads.push(thread_spawn(move || {
                let mut buf = Vec::new();
                let have_read = source_reader.read_to_end(&mut buf)?;
                assert_eq!(have_read, 1 << 20);
                assert_eq!(buf, vec![i as u8; 1 << 20]);
                Ok(())
            }));
        }
        threads.push(producer_thread);
        for thread in threads {
            thread.join().unwrap()?;
        }

        Ok(())
    }
}
