use super::{DataSource, DataSourceReader, SeekableSource, SourceKey, UnseekableDataSource};
use digest::OutputSizeUser;
use once_cell::sync::OnceCell;
use sha1::{Digest, Sha1};
use std::{
    fmt::{self, Debug},
    fs::File,
    io::{Result as IoResult, Seek},
    path::PathBuf,
    sync::Mutex,
};

#[cfg(feature = "async")]
use {
    super::{AsyncDataSourceReader, AsyncSeekableSource, AsyncUnseekableDataSource},
    async_once_cell::OnceCell as AsyncOnceCell,
    async_std::{fs::File as AsyncFile, path::PathBuf as AsyncPathBuf},
    futures::{future::BoxFuture, lock::Mutex as AsyncMutex, AsyncSeekExt},
};

enum Source<A: OutputSizeUser> {
    Seekable {
        source: SeekableSource,
        current_offset: Mutex<u64>,
        file_size: u64,
    },
    Unseekable(UnseekableDataSource<File, A>),
}

impl<A: OutputSizeUser> Debug for Source<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Seekable {
                source,
                current_offset,
                file_size,
            } => f
                .debug_struct("Seekable")
                .field("source", source)
                .field("current_offset", current_offset)
                .field("file_size", file_size)
                .finish(),
            Self::Unseekable(file) => f.debug_tuple("Unseekable").field(file).finish(),
        }
    }
}

pub struct FileDataSource<A: OutputSizeUser> {
    path: PathBuf,
    canonicalized_path: OnceCell<PathBuf>,
    source: OnceCell<Source<A>>,

    #[cfg(feature = "async")]
    async_source: AsyncFileDataSource<A>,
}

impl<A: OutputSizeUser> FileDataSource<A> {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            canonicalized_path: Default::default(),
            source: Default::default(),

            #[cfg(feature = "async")]
            async_source: Default::default(),
        }
    }

    fn get_seekable_source(&self) -> IoResult<&Source<A>> {
        self.source.get_or_try_init(|| {
            let mut file = File::open(&self.path)?;
            if let Ok(current_offset) = file.stream_position() {
                Ok(Source::Seekable {
                    file_size: file.metadata()?.len(),
                    source: SeekableSource::new(file, 0, 0),
                    current_offset: Mutex::new(current_offset),
                })
            } else {
                Ok(Source::Unseekable(UnseekableDataSource::new(file)))
            }
        })
    }

    fn get_path(&self) -> IoResult<&PathBuf> {
        self.canonicalized_path
            .get_or_try_init(|| self.path.canonicalize())
    }

    #[cfg(feature = "async")]
    async fn get_async_seekable_source(&self) -> IoResult<&AsyncSource<A>> {
        self.async_source
            .source
            .get_or_try_init(async {
                let mut file = AsyncFile::open(&self.path).await?;
                if let Ok(current_offset) = file.stream_position().await {
                    Ok(AsyncSource::Seekable {
                        file_size: file.metadata().await?.len(),
                        source: AsyncSeekableSource::new(file, 0, 0),
                        current_offset: AsyncMutex::new(current_offset),
                    })
                } else {
                    Ok(AsyncSource::Unseekable(AsyncUnseekableDataSource::new(
                        file,
                    )))
                }
            })
            .await
    }

    #[cfg(feature = "async")]
    async fn get_async_path(&self) -> IoResult<&AsyncPathBuf> {
        self.async_source
            .path
            .get_or_try_init(async { AsyncPathBuf::from(&self.path).canonicalize().await })
            .await
    }
}

impl DataSource<Sha1> for FileDataSource<Sha1> {
    fn slice(&self, size: u64) -> IoResult<Option<DataSourceReader>> {
        match self.get_seekable_source()? {
            Source::Seekable {
                source,
                file_size,
                current_offset,
            } => {
                let mut cur_off = current_offset.lock().unwrap();
                if *cur_off < *file_size {
                    let source_reader = DataSourceReader::seekable(
                        source.clone_with_new_offset_and_length(*cur_off, size),
                    );
                    *cur_off += size;
                    Ok(Some(source_reader))
                } else {
                    Ok(None)
                }
            }
            Source::Unseekable(source) => source.slice(size),
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_slice(&self, size: u64) -> BoxFuture<IoResult<Option<AsyncDataSourceReader>>> {
        Box::pin(async move {
            match self.get_async_seekable_source().await? {
                AsyncSource::Seekable {
                    source,
                    current_offset,
                    file_size,
                } => {
                    let mut cur_off = current_offset.lock().await;
                    if *cur_off < *file_size {
                        let source_reader = AsyncDataSourceReader::seekable(
                            source.clone_with_new_offset_and_length(*cur_off, size),
                        );
                        *cur_off += size;
                        Ok(Some(source_reader))
                    } else {
                        Ok(None)
                    }
                }
                AsyncSource::Unseekable(source) => source.async_slice(size).await,
            }
        })
    }

    fn source_key(&self) -> IoResult<Option<SourceKey<Sha1>>> {
        match self.get_seekable_source()? {
            Source::Seekable { .. } => {
                let mut hasher = Sha1::new();
                hasher.update(b"file://");
                hasher.update(&self.get_path()?.display().to_string());
                Ok(Some(hasher.finalize().into()))
            }
            Source::Unseekable(source) => source.source_key(),
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_source_key(&self) -> BoxFuture<IoResult<Option<SourceKey<Sha1>>>> {
        Box::pin(async move {
            match self.get_async_seekable_source().await? {
                AsyncSource::Seekable { .. } => {
                    let mut hasher = Sha1::new();
                    hasher.update(b"file://");
                    hasher.update(&self.get_async_path().await?.display().to_string());
                    Ok(Some(hasher.finalize().into()))
                }
                AsyncSource::Unseekable(source) => source.async_source_key().await,
            }
        })
    }
}

impl<A: OutputSizeUser> Debug for FileDataSource<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("FileDataSource");
        d.field("path", &self.path)
            .field("canonicalized_path", &self.canonicalized_path)
            .field("source", &self.source);

        #[cfg(feature = "async")]
        d.field("async_source", &self.async_source);

        d.finish()
    }
}

#[cfg(feature = "async")]
struct AsyncFileDataSource<A: OutputSizeUser> {
    path: AsyncOnceCell<AsyncPathBuf>,
    source: AsyncOnceCell<AsyncSource<A>>,
}

#[cfg(feature = "async")]
impl<A: OutputSizeUser> Default for AsyncFileDataSource<A> {
    #[inline]
    fn default() -> Self {
        Self {
            path: AsyncOnceCell::new(),
            source: AsyncOnceCell::new(),
        }
    }
}

#[cfg(feature = "async")]
impl<A: OutputSizeUser> Debug for AsyncFileDataSource<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncFileDataSource")
            .field("path", &self.path)
            .field("source", &self.source)
            .finish()
    }
}

#[cfg(feature = "async")]
enum AsyncSource<A: OutputSizeUser> {
    Seekable {
        source: AsyncSeekableSource,
        current_offset: AsyncMutex<u64>,
        file_size: u64,
    },
    Unseekable(AsyncUnseekableDataSource<AsyncFile, A>),
}

#[cfg(feature = "async")]
impl<A: OutputSizeUser> Debug for AsyncSource<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Seekable {
                source,
                current_offset,
                file_size,
            } => f
                .debug_struct("Seekable")
                .field("source", source)
                .field("current_offset", current_offset)
                .field("file_size", file_size)
                .finish(),
            Self::Unseekable(file) => f.debug_tuple("Unseekable").field(file).finish(),
        }
    }
}

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
        let data_source = FileDataSource::new(&temp_file_path);
        let mut source_readers = Vec::with_capacity(256);
        for _ in 0..255u8 {
            source_readers.push(data_source.slice(1 << 20)?.unwrap());
        }
        assert!(data_source.slice(1 << 20)?.is_none());

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
