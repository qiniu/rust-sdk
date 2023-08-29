mod source_key;
pub use source_key::SourceKey;

mod file;
pub use file::FileDataSource;

mod seekable;
pub(crate) use seekable::SeekableDataSource;
pub use seekable::SeekableSource;

mod unseekable;
pub use unseekable::UnseekableDataSource;

mod reader;
pub(crate) use reader::Digestible;
pub use reader::{DataSource, DataSourceReader};

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
mod async_seekable;

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
mod async_unseekable;

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
mod async_file;

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
mod async_reader;

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
pub use {
    async_file::AsyncFileDataSource,
    async_reader::{AsyncDataSource, AsyncDataSourceReader},
    async_seekable::AsyncSeekableSource,
    async_unseekable::AsyncUnseekableDataSource,
};

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
pub(crate) use {async_reader::AsyncDigestible, async_seekable::AsyncSeekableDataSource};

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rand::{thread_rng, RngCore};
    use std::{
        fs::OpenOptions,
        io::{copy as io_copy, Cursor, Read, Seek, SeekFrom},
        num::NonZeroUsize,
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

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    #[qiniu_utils::async_runtime::test]
    async fn test_async_data_source_reader() -> Result<()> {
        use futures::{
            future::join,
            io::{copy as io_copy, AsyncReadExt, AsyncSeekExt, Cursor},
            lock::Mutex,
        };
        use qiniu_utils::async_fs::OpenOptions;

        env_logger::builder().is_test(true).try_init().ok();

        let temp_path = new_temp_file()?.into_temp_path();
        let temp_file = OpenOptions::new().read(true).write(true).open(&*temp_path).await?;
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
        temp_file.rewind()?;
        Ok(temp_file)
    }
}
