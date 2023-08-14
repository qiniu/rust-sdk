use super::{
    super::PartSize, seekable::SeekableDataSource, DataSource, DataSourceReader, SourceKey, UnseekableDataSource,
};
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

    fn reset(&self) -> IoResult<()> {
        match self.get_seekable_source()? {
            Source::Seekable(source) => DataSource::<D>::reset(source),
            Source::Unseekable(source) => source.reset(),
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
