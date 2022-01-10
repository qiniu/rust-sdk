use super::{
    AppendOnlyResumableRecorderMedium, ReadOnlyResumableRecorderMedium, ResumableRecorder,
    SourceKey,
};
use sha1::Sha1;
use std::{
    fs::{remove_file, File, OpenOptions},
    io::{Read, Result as IoResult, Write},
    path::PathBuf,
};

#[cfg(feature = "async")]
use {async_std::fs::remove_file as async_remove_file, futures::future::BoxFuture};

#[derive(Debug)]
pub struct FileSystemResumableRecorder {
    path: PathBuf,
}

impl FileSystemResumableRecorder {
    #[inline]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn path_of(
        &self,
        source_key: &SourceKey<<Self as ResumableRecorder>::HashAlgorithm>,
    ) -> PathBuf {
        self.path.join(&hex::encode(source_key.as_slice()))
    }
}

impl ResumableRecorder for FileSystemResumableRecorder {
    type HashAlgorithm = Sha1;
    type ReadOnlyMedium = FileSystemReadOnlyResumableRecorderMedium;
    type AppendOnlyMedium = FileSystemAppendOnlyResumableRecorderMedium;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    type AsyncReadOnlyMedium = FileSystemReadOnlyAsyncResumableRecorderMedium;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    type AsyncAppendOnlyMedium = FileSystemAppendOnlyAsyncResumableRecorderMedium;

    #[inline]
    fn open_for_read(
        &self,
        source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Self::ReadOnlyMedium> {
        Self::ReadOnlyMedium::new_for_read(self.path_of(source_key))
    }

    fn open_for_append(
        &self,
        source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Self::AppendOnlyMedium> {
        Self::AppendOnlyMedium::new_for_append(self.path_of(source_key))
    }

    fn open_for_create_new(
        &self,
        source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Self::AppendOnlyMedium> {
        Self::AppendOnlyMedium::new_for_create_new(self.path_of(source_key))
    }

    fn delete(&self, source_key: &SourceKey<Self::HashAlgorithm>) -> IoResult<()> {
        remove_file(self.path_of(source_key))
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_read<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Self::AsyncReadOnlyMedium>> {
        Box::pin(
            async move { Self::AsyncReadOnlyMedium::new_for_read(self.path_of(source_key)).await },
        )
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_append<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Self::AsyncAppendOnlyMedium>> {
        Box::pin(async move {
            Self::AsyncAppendOnlyMedium::new_for_append(self.path_of(source_key)).await
        })
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_create_new<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Self::AsyncAppendOnlyMedium>> {
        Box::pin(async move {
            Self::AsyncAppendOnlyMedium::new_for_create_new(self.path_of(source_key)).await
        })
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_delete<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<()>> {
        Box::pin(async move { async_remove_file(self.path_of(source_key)).await })
    }
}

#[derive(Debug)]
pub struct FileSystemReadOnlyResumableRecorderMedium {
    path: PathBuf,
    file: File,
}

impl FileSystemReadOnlyResumableRecorderMedium {
    fn new_for_read(path: PathBuf) -> IoResult<Self> {
        let file = OpenOptions::new().read(true).open(&path)?;
        Ok(Self { file, path })
    }
}

impl Read for FileSystemReadOnlyResumableRecorderMedium {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.file.read(buf)
    }
}

impl ReadOnlyResumableRecorderMedium for FileSystemReadOnlyResumableRecorderMedium {
    type AppendOnlyMedium = FileSystemAppendOnlyResumableRecorderMedium;

    #[inline]
    fn into_medium_for_append(self) -> IoResult<Self::AppendOnlyMedium> {
        Self::AppendOnlyMedium::new_for_append(self.path)
    }

    #[inline]
    fn into_medium_for_create_new(self) -> IoResult<Self::AppendOnlyMedium> {
        Self::AppendOnlyMedium::new_for_create_new(self.path)
    }
}

#[derive(Debug)]
pub struct FileSystemAppendOnlyResumableRecorderMedium {
    path: PathBuf,
    file: File,
}

impl FileSystemAppendOnlyResumableRecorderMedium {
    fn new_for_append(path: PathBuf) -> IoResult<Self> {
        let file = OpenOptions::new().append(true).open(&path)?;
        Ok(Self { file, path })
    }

    fn new_for_create_new(path: PathBuf) -> IoResult<Self> {
        let file = OpenOptions::new()
            .create(true)
            .create_new(true)
            .write(true)
            .open(&path)?;
        Ok(Self { file, path })
    }
}

impl Write for FileSystemAppendOnlyResumableRecorderMedium {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.file.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> IoResult<()> {
        self.file.flush()
    }
}

impl AppendOnlyResumableRecorderMedium for FileSystemAppendOnlyResumableRecorderMedium {
    type ReadOnlyMedium = FileSystemReadOnlyResumableRecorderMedium;

    #[inline]
    fn into_medium_for_read(self) -> IoResult<Self::ReadOnlyMedium> {
        Self::ReadOnlyMedium::new_for_read(self.path)
    }
}

#[cfg(feature = "async")]
mod async_medium {
    use super::{
        super::{AppendOnlyAsyncResumableRecorderMedium, ReadOnlyAsyncResumableRecorderMedium},
        *,
    };
    use async_std::fs::{File, OpenOptions};
    use futures::{future::BoxFuture, AsyncRead, AsyncWrite};
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };

    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    #[derive(Debug)]
    pub struct FileSystemReadOnlyAsyncResumableRecorderMedium {
        path: PathBuf,
        file: File,
    }

    impl FileSystemReadOnlyAsyncResumableRecorderMedium {
        pub(super) async fn new_for_read(path: PathBuf) -> IoResult<Self> {
            let file = OpenOptions::new().read(true).open(&path).await?;
            Ok(Self { file, path })
        }
    }

    impl AsyncRead for FileSystemReadOnlyAsyncResumableRecorderMedium {
        #[inline]
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<IoResult<usize>> {
            Pin::new(&mut self.file).poll_read(cx, buf)
        }
    }

    impl ReadOnlyAsyncResumableRecorderMedium for FileSystemReadOnlyAsyncResumableRecorderMedium {
        type AppendOnlyMedium = FileSystemAppendOnlyAsyncResumableRecorderMedium;

        #[inline]
        fn into_medium_for_append(self) -> BoxFuture<'static, IoResult<Self::AppendOnlyMedium>> {
            Box::pin(async move { Self::AppendOnlyMedium::new_for_append(self.path).await })
        }

        #[inline]
        fn into_medium_for_create_new(
            self,
        ) -> BoxFuture<'static, IoResult<Self::AppendOnlyMedium>> {
            Box::pin(async move { Self::AppendOnlyMedium::new_for_create_new(self.path).await })
        }
    }

    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    #[derive(Debug)]
    pub struct FileSystemAppendOnlyAsyncResumableRecorderMedium {
        path: PathBuf,
        file: File,
    }

    impl FileSystemAppendOnlyAsyncResumableRecorderMedium {
        pub(super) async fn new_for_append(path: PathBuf) -> IoResult<Self> {
            let file = OpenOptions::new().append(true).open(&path).await?;
            Ok(Self { file, path })
        }

        pub(super) async fn new_for_create_new(path: PathBuf) -> IoResult<Self> {
            let file = OpenOptions::new()
                .create(true)
                .create_new(true)
                .write(true)
                .open(&path)
                .await?;
            Ok(Self { file, path })
        }
    }

    impl AsyncWrite for FileSystemAppendOnlyAsyncResumableRecorderMedium {
        #[inline]
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<IoResult<usize>> {
            Pin::new(&mut self.file).poll_write(cx, buf)
        }

        #[inline]
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
            Pin::new(&mut self.file).poll_flush(cx)
        }

        #[inline]
        fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
            Pin::new(&mut self.file).poll_close(cx)
        }
    }

    impl AppendOnlyAsyncResumableRecorderMedium for FileSystemAppendOnlyAsyncResumableRecorderMedium {
        type ReadOnlyMedium = FileSystemReadOnlyAsyncResumableRecorderMedium;

        #[inline]
        fn into_medium_for_read(self) -> BoxFuture<'static, IoResult<Self::ReadOnlyMedium>> {
            Box::pin(async move { Self::ReadOnlyMedium::new_for_read(self.path).await })
        }
    }
}

#[cfg(feature = "async")]
pub use async_medium::*;

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rand::{thread_rng, RngCore};
    use tempfile::tempdir;

    #[test]
    fn test_file_system_resumable_recorder() -> Result<()> {
        let dir = tempdir()?;
        let source_key = SourceKey::<Sha1>::new([0u8; 20]);
        let recorder = FileSystemResumableRecorder::new(dir.path());
        let mut rander = thread_rng();
        let mut buf = vec![0u8; 1 << 20];
        rander.fill_bytes(&mut buf);

        recorder.open_for_append(&source_key).unwrap_err();
        {
            let mut medium = recorder.open_for_create_new(&source_key)?;
            medium.write_all(&buf)?;
        }
        {
            let mut medium = recorder.open_for_append(&source_key)?;
            medium.write_all(&buf)?;
        }
        {
            let mut medium = recorder.open_for_read(&source_key)?;
            let mut buf1 = Vec::new();
            let mut buf2 = Vec::new();
            (&mut medium).take(1 << 20).read_to_end(&mut buf1)?;
            (&mut medium).take(1 << 20).read_to_end(&mut buf2)?;
            assert_eq!(buf1.len(), buf.len());
            assert_eq!(buf2.len(), buf.len());
            assert!(buf1 == buf);
            assert!(buf2 == buf);
        }

        Ok(())
    }

    #[cfg(feature = "async")]
    #[async_std::test]
    async fn test_async_file_system_resumable_recorder() -> Result<()> {
        use super::super::AppendOnlyAsyncResumableRecorderMedium;
        use futures::{AsyncReadExt, AsyncWriteExt};

        let dir = tempdir()?;
        let source_key = SourceKey::<Sha1>::new([0u8; 20]);
        let recorder = FileSystemResumableRecorder::new(dir.path());
        let mut rander = thread_rng();
        let mut buf = vec![0u8; 1 << 20];
        rander.fill_bytes(&mut buf);

        recorder.open_for_append(&source_key).unwrap_err();
        {
            let mut medium = recorder.open_for_async_create_new(&source_key).await?;
            medium.write_all(&buf).await?;
        }
        {
            let mut medium = recorder.open_for_async_append(&source_key).await?;
            medium.write_all(&buf).await?;
            let mut medium = medium.into_medium_for_read().await?;
            let mut buf1 = Vec::new();
            let mut buf2 = Vec::new();
            (&mut medium).take(1 << 20).read_to_end(&mut buf1).await?;
            (&mut medium).take(1 << 20).read_to_end(&mut buf2).await?;
            assert_eq!(buf1.len(), buf.len());
            assert_eq!(buf2.len(), buf.len());
            assert!(buf1 == buf);
            assert!(buf2 == buf);
        }
        {}

        Ok(())
    }
}
