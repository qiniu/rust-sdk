use super::{AppendOnlyResumableRecorderMedium, ReadOnlyResumableRecorderMedium, ResumableRecorder, SourceKey};
use digest::Digest;
use sha1::Sha1;
use std::{
    env::temp_dir,
    fmt::{self, Debug},
    fs::{remove_file, DirBuilder, OpenOptions},
    io::Result as IoResult,
    marker::PhantomData,
    path::PathBuf,
};

#[cfg(feature = "async")]
use {
    super::{AppendOnlyAsyncResumableRecorderMedium, ReadOnlyAsyncResumableRecorderMedium},
    async_std::fs::{remove_file as async_remove_file, DirBuilder as AsyncDirBuilder, OpenOptions as AsyncOpenOptions},
    futures::future::BoxFuture,
};

/// 文件系统断点恢复记录器
///
/// 基于文件系统提供断点恢复记录功能
pub struct FileSystemResumableRecorder<O = Sha1> {
    path: PathBuf,
    _unused: PhantomData<O>,
}

const DEFAULT_DIRECTORY_NAME: &str = ".qiniu-rust-sdk";

impl<O> FileSystemResumableRecorder<O> {
    /// 创建文件系统断点恢复记录器，传入一个目录路径用于储存断点记录
    #[inline]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            _unused: Default::default(),
        }
    }

    fn create_directory(&self) -> IoResult<()> {
        DirBuilder::new().recursive(true).create(&self.path)
    }

    #[cfg(feature = "async")]
    async fn async_create_directory(&self) -> IoResult<()> {
        AsyncDirBuilder::new().recursive(true).create(&self.path).await
    }
}

impl<O: Digest> ResumableRecorder for FileSystemResumableRecorder<O> {
    type HashAlgorithm = O;

    #[inline]
    fn open_for_read(
        &self,
        source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Box<dyn ReadOnlyResumableRecorderMedium>> {
        self.create_directory()?;
        let medium = OpenOptions::new().read(true).open(self.path_of(source_key))?;
        Ok(Box::new(medium))
    }

    fn open_for_append(
        &self,
        source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Box<dyn AppendOnlyResumableRecorderMedium>> {
        self.create_directory()?;
        let medium = OpenOptions::new().append(true).open(self.path_of(source_key))?;
        Ok(Box::new(medium))
    }

    fn open_for_create_new(
        &self,
        source_key: &SourceKey<Self::HashAlgorithm>,
    ) -> IoResult<Box<dyn AppendOnlyResumableRecorderMedium>> {
        self.create_directory()?;
        let medium = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(self.path_of(source_key))?;
        Ok(Box::new(medium))
    }

    fn delete(&self, source_key: &SourceKey<Self::HashAlgorithm>) -> IoResult<()> {
        remove_file(self.path_of(source_key))
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_read<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Box<dyn ReadOnlyAsyncResumableRecorderMedium>>> {
        Box::pin(async move {
            self.async_create_directory().await?;
            let medium = AsyncOpenOptions::new()
                .read(true)
                .open(self.path_of(source_key))
                .await?;
            Ok(Box::new(medium) as Box<dyn ReadOnlyAsyncResumableRecorderMedium>)
        })
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_append<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Box<dyn AppendOnlyAsyncResumableRecorderMedium>>> {
        Box::pin(async move {
            self.async_create_directory().await?;
            let medium = AsyncOpenOptions::new()
                .append(true)
                .open(self.path_of(source_key))
                .await?;
            Ok(Box::new(medium) as Box<dyn AppendOnlyAsyncResumableRecorderMedium>)
        })
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn open_for_async_create_new<'a>(
        &'a self,
        source_key: &'a SourceKey<Self::HashAlgorithm>,
    ) -> BoxFuture<'a, IoResult<Box<dyn AppendOnlyAsyncResumableRecorderMedium>>> {
        Box::pin(async move {
            self.async_create_directory().await?;
            let medium = AsyncOpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(self.path_of(source_key))
                .await?;
            Ok(Box::new(medium) as Box<dyn AppendOnlyAsyncResumableRecorderMedium>)
        })
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_delete<'a>(&'a self, source_key: &'a SourceKey<Self::HashAlgorithm>) -> BoxFuture<'a, IoResult<()>> {
        Box::pin(async move { async_remove_file(self.path_of(source_key)).await })
    }
}

impl<O: Digest> FileSystemResumableRecorder<O> {
    fn path_of(&self, source_key: &SourceKey<O>) -> PathBuf {
        self.path.join(&hex::encode(source_key.as_slice()))
    }
}

impl<O> Default for FileSystemResumableRecorder<O> {
    #[inline]
    fn default() -> Self {
        Self::new(temp_dir().join(DEFAULT_DIRECTORY_NAME))
    }
}

impl<O> Debug for FileSystemResumableRecorder<O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileSystemResumableRecorder")
            .field("path", &self.path)
            .finish()
    }
}

impl<O> Clone for FileSystemResumableRecorder<O> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            _unused: self._unused,
        }
    }
}

#[allow(unsafe_code)]
unsafe impl<O> Send for FileSystemResumableRecorder<O> {}

#[allow(unsafe_code)]
unsafe impl<O> Sync for FileSystemResumableRecorder<O> {}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rand::{thread_rng, RngCore};
    use std::io::{Read, Write};
    use tempfile::tempdir;

    #[test]
    fn test_file_system_resumable_recorder() -> Result<()> {
        let dir = tempdir()?;
        let source_key = SourceKey::new([0u8; 20]);
        let recorder = FileSystemResumableRecorder::<Sha1>::new(dir.path());
        let mut rander = thread_rng();
        let mut buf = vec![0u8; 1 << 20];
        rander.fill_bytes(&mut buf);

        recorder.delete(&source_key).ok();
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
        use futures::{AsyncReadExt, AsyncWriteExt};

        let dir = tempdir()?;
        let source_key = SourceKey::new([0u8; 20]);
        let recorder = FileSystemResumableRecorder::<Sha1>::new(dir.path());
        let mut rander = thread_rng();
        let mut buf = vec![0u8; 1 << 20];
        rander.fill_bytes(&mut buf);

        recorder.async_delete(&source_key).await.ok();
        recorder.open_for_async_append(&source_key).await.unwrap_err();
        {
            let mut medium = recorder.open_for_async_create_new(&source_key).await?;
            medium.write_all(&buf).await?;
            medium.flush().await?;
        }
        {
            let mut medium = recorder.open_for_async_append(&source_key).await?;
            medium.write_all(&buf).await?;
            medium.flush().await?;
            let mut medium = recorder.open_for_async_read(&source_key).await?;
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
