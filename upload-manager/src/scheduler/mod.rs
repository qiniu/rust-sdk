use super::{
    data_source::{FileDataSource, SeekableDataSource, UnseekableDataSource},
    ConcurrencyProvider, DataPartitionProvider, DataSource, ObjectParams,
};
use auto_impl::auto_impl;
use digest::Digest;
use dyn_clonable::clonable;
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::{
    fmt::Debug,
    io::{Error as IoError, Read, Result as IoResult, Seek, SeekFrom},
    path::Path,
};

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
use {
    super::data_source::{AsyncDataSource, AsyncFileDataSource, AsyncSeekableDataSource, AsyncUnseekableDataSource},
    futures::{future::BoxFuture, AsyncRead, AsyncSeek, AsyncSeekExt},
};

/// 分片上传调度器接口
///
/// 负责分片上传的调度，包括初始化分片信息、上传分片、完成分片上传。
#[clonable]
#[auto_impl(&mut, Box)]
pub trait MultiPartsUploaderScheduler<A: Digest>: Clone + Send + Sync + Debug {
    /// 设置并发数提供者
    fn set_concurrency_provider(&mut self, concurrency_provider: Box<dyn ConcurrencyProvider>);

    /// 设置分片大小提供者
    fn set_data_partition_provider(&mut self, data_partition_provider: Box<dyn DataPartitionProvider>);

    /// 上传数据源
    ///
    /// 该方法的异步版本为 [`Self::async_upload`]。
    fn upload(&self, source: Box<dyn DataSource<A>>, params: ObjectParams) -> ApiResult<Value>;

    /// 异步上传数据源
    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
    )]
    fn async_upload(&self, source: Box<dyn AsyncDataSource<A>>, params: ObjectParams) -> BoxFuture<ApiResult<Value>>;
}

/// 分片上传调度器扩展接口
pub trait MultiPartsUploaderSchedulerExt<A: Digest + Send + 'static>: MultiPartsUploaderScheduler<A> {
    /// 上传指定路径的文件
    fn upload_path(&self, path: impl AsRef<Path>, params: ObjectParams) -> ApiResult<Value> {
        self.upload(Box::new(FileDataSource::new(path.as_ref())), params)
    }

    /// 上传不可寻址的输入流的数据
    fn upload_reader<R: Read + Debug + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> ApiResult<Value> {
        self.upload(Box::new(UnseekableDataSource::new(reader)), params)
    }

    /// 上传可寻址的输入流的数据
    fn upload_seekable_reader<R: Read + Seek + Debug + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> ApiResult<Value> {
        let data_source = _make_seekable_data_source(reader)
            .map(|source| Box::new(source) as Box<dyn DataSource<A>>)
            .unwrap_or_else(|(_, reader)| Box::new(UnseekableDataSource::new(reader)));
        return self.upload(data_source, params);

        fn _make_seekable_data_source<R: Read + Seek + Debug + Send + Sync + 'static>(
            mut reader: R,
        ) -> Result<SeekableDataSource, (IoError, R)> {
            match _get_len(&mut reader) {
                Ok(len) => SeekableDataSource::new(reader, len),
                Err(err) => Err((err, reader)),
            }
        }

        fn _get_len<R: Read + Seek + Debug + Send + Sync + 'static>(reader: &mut R) -> IoResult<u64> {
            let old_pos = reader.stream_position()?;
            let len = reader.seek(SeekFrom::End(0))?;
            if old_pos != len {
                reader.seek(SeekFrom::Start(old_pos))?;
            }
            Ok(len)
        }
    }

    /// 异步上传指定路径的文件
    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
    )]
    fn async_upload_path<'a>(
        &'a self,
        path: impl AsRef<Path> + Send + Sync + 'a,
        params: ObjectParams,
    ) -> BoxFuture<'a, ApiResult<Value>> {
        Box::pin(async move {
            self.async_upload(Box::new(AsyncFileDataSource::new(path.as_ref())), params)
                .await
        })
    }

    /// 异步上传不可寻址的输入流的数据
    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
    )]
    fn async_upload_reader<R: AsyncRead + Unpin + Debug + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>> {
        Box::pin(async move {
            self.async_upload(Box::new(AsyncUnseekableDataSource::new(reader)), params)
                .await
        })
    }

    /// 异步上传可寻址的输入流的数据
    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
    )]
    fn async_upload_seekable_reader<R: AsyncRead + AsyncSeek + Unpin + Debug + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>> {
        return Box::pin(async move {
            let data_source = _make_seekable_data_source(reader)
                .await
                .map(|source| Box::new(source) as Box<dyn AsyncDataSource<A>>)
                .unwrap_or_else(|(_, reader)| Box::new(AsyncUnseekableDataSource::new(reader)));
            self.async_upload(data_source, params).await
        });

        async fn _make_seekable_data_source<R: AsyncRead + AsyncSeek + Unpin + Debug + Send + Sync + 'static>(
            mut reader: R,
        ) -> Result<AsyncSeekableDataSource, (IoError, R)> {
            match _get_len(&mut reader).await {
                Ok(len) => AsyncSeekableDataSource::new(reader, len).await,
                Err(err) => Err((err, reader)),
            }
        }

        async fn _get_len<R: AsyncRead + AsyncSeek + Unpin + Debug + Send + Sync + 'static>(
            reader: &mut R,
        ) -> IoResult<u64> {
            let old_pos = reader.stream_position().await?;
            let len = reader.seek(SeekFrom::End(0)).await?;
            if old_pos != len {
                reader.seek(SeekFrom::Start(old_pos)).await?;
            }
            Ok(len)
        }
    }
}

impl<A: Digest + Send + 'static, T: MultiPartsUploaderScheduler<A>> MultiPartsUploaderSchedulerExt<A> for T {}

mod serial_multi_parts_uploader_scheduler;
pub use serial_multi_parts_uploader_scheduler::SerialMultiPartsUploaderScheduler;
mod concurrent_multi_parts_uploader_scheduler;
pub use concurrent_multi_parts_uploader_scheduler::ConcurrentMultiPartsUploaderScheduler;

mod utils;
