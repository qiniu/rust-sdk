use super::{DataSource, DataSourceReader, SeekableSource, SourceKey, UnseekableDataSource};
use once_cell::sync::OnceCell;
use sha1::{Digest, Sha1};
use std::{
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

#[derive(Debug)]
enum Source {
    Seekable {
        source: SeekableSource,
        current_offset: Mutex<u64>,
        file_size: u64,
    },
    Unseekable(UnseekableDataSource<File>),
}

#[derive(Debug)]
pub(crate) struct FileDataSource {
    path: PathBuf,
    canonicalized_path: OnceCell<PathBuf>,
    source: OnceCell<Source>,

    #[cfg(feature = "async")]
    async_source: AsyncFileDataSource,
}

impl FileDataSource {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            canonicalized_path: Default::default(),
            source: Default::default(),

            #[cfg(feature = "async")]
            async_source: Default::default(),
        }
    }

    fn get_seekable_source(&self) -> IoResult<&Source> {
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
    async fn get_async_seekable_source(&self) -> IoResult<&AsyncSource> {
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

impl DataSource for FileDataSource {
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

    fn source_key(&self) -> IoResult<Option<SourceKey>> {
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
    fn async_source_key(&self) -> BoxFuture<IoResult<Option<SourceKey>>> {
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

#[cfg(feature = "async")]
#[derive(Debug)]
struct AsyncFileDataSource {
    path: AsyncOnceCell<AsyncPathBuf>,
    source: AsyncOnceCell<AsyncSource>,
}

#[cfg(feature = "async")]
impl Default for AsyncFileDataSource {
    #[inline]
    fn default() -> Self {
        Self {
            path: AsyncOnceCell::new(),
            source: AsyncOnceCell::new(),
        }
    }
}

#[cfg(feature = "async")]
#[derive(Debug)]
enum AsyncSource {
    Seekable {
        source: AsyncSeekableSource,
        current_offset: AsyncMutex<u64>,
        file_size: u64,
    },
    Unseekable(AsyncUnseekableDataSource<AsyncFile>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use defer_lite::defer;
    use ipipe::Pipe;
    use std::{
        fs::remove_file,
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
    fn test_sync_unseekable_file_data_source() -> Result<()> {
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
