use super::{
    super::{AsyncDataSource, AsyncDataSourceReader, PartSize},
    async_seekable::AsyncSeekableDataSource,
    async_unseekable::AsyncUnseekableDataSource,
    SourceKey,
};
use async_once_cell::OnceCell as AsyncOnceCell;
use digest::Digest;
use futures::future::BoxFuture;
use os_str_bytes::OsStrBytes;
use qiniu_utils::async_fs::{canonicalize as async_canonicalize, File as AsyncFile};
use std::{
    fmt::{self, Debug},
    io::Result as IoResult,
    path::PathBuf,
    sync::Arc,
};

/// 异步文件数据源
///
/// 基于一个文件实现了数据源接口
pub struct AsyncFileDataSource<A: Digest> {
    path: PathBuf,
    canonicalized_path: Arc<AsyncOnceCell<PathBuf>>,
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
                let file = AsyncFile::open(&self.path).await?;
                if let Ok(metadata) = file.metadata().await {
                    match AsyncSeekableDataSource::new(file, metadata.len()).await {
                        Ok(source) => Ok(AsyncSource::Seekable(source)),
                        Err((_, file)) => Ok(AsyncSource::Unseekable(AsyncUnseekableDataSource::new(file))),
                    }
                } else {
                    Ok(AsyncSource::Unseekable(AsyncUnseekableDataSource::new(file)))
                }
            })
            .await
    }

    async fn get_async_path(&self) -> IoResult<&PathBuf> {
        self.canonicalized_path
            .get_or_try_init(async { async_canonicalize(&self.path).await })
            .await
    }
}

impl<A: Digest + Send> AsyncDataSource<A> for AsyncFileDataSource<A> {
    #[inline]
    fn slice(&self, size: PartSize) -> BoxFuture<IoResult<Option<AsyncDataSourceReader>>> {
        Box::pin(async move {
            match self.get_async_seekable_source().await? {
                AsyncSource::Seekable(source) => AsyncDataSource::<A>::slice(source, size).await,
                AsyncSource::Unseekable(source) => source.slice(size).await,
            }
        })
    }

    #[inline]
    fn reset(&self) -> BoxFuture<IoResult<()>> {
        Box::pin(async move {
            match self.get_async_seekable_source().await? {
                AsyncSource::Seekable(source) => AsyncDataSource::<A>::reset(source).await,
                AsyncSource::Unseekable(source) => source.reset().await,
            }
        })
    }

    #[inline]
    fn source_key(&self) -> BoxFuture<IoResult<Option<SourceKey<A>>>> {
        Box::pin(async move {
            match self.get_async_seekable_source().await? {
                AsyncSource::Seekable { .. } => {
                    let mut hasher = A::new();
                    hasher.update(b"file://");
                    hasher.update(self.get_async_path().await?.as_os_str().to_raw_bytes());
                    Ok(Some(hasher.finalize().into()))
                }
                AsyncSource::Unseekable(source) => source.source_key().await,
            }
        })
    }

    #[inline]
    fn total_size(&self) -> BoxFuture<IoResult<Option<u64>>> {
        Box::pin(async move {
            match self.get_async_seekable_source().await? {
                AsyncSource::Seekable(source) => AsyncDataSource::<A>::total_size(source).await,
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

pub(super) enum AsyncSource<A: Digest> {
    Seekable(AsyncSeekableDataSource),
    Unseekable(AsyncUnseekableDataSource<AsyncFile, A>),
}

impl<A: Digest> Debug for AsyncSource<A> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Seekable(seekable) => f.debug_tuple("Seekable").field(seekable).finish(),
            Self::Unseekable(file) => f.debug_tuple("Unseekable").field(file).finish(),
        }
    }
}
