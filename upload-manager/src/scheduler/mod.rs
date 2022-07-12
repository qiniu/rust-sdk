use super::{ConcurrencyProvider, DataPartitionProvider, DataSource, ObjectParams};
use crate::{data_source::FileDataSource, data_source::UnseekableDataSource};
use auto_impl::auto_impl;
use digest::Digest;
use dyn_clonable::clonable;
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::{fmt::Debug, io::Read, path::Path};

#[cfg(feature = "async")]
use {
    crate::data_source::{AsyncDataSource, AsyncFileDataSource, AsyncUnseekableDataSource},
    futures::{future::BoxFuture, AsyncRead},
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
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload(&self, source: Box<dyn AsyncDataSource<A>>, params: ObjectParams) -> BoxFuture<ApiResult<Value>>;
}

/// 分片上传调度器扩展接口
pub trait MultiPartsUploaderSchedulerExt<A: Digest + Send + 'static>: MultiPartsUploaderScheduler<A> {
    /// 上传指定路径的文件
    fn upload_path(&self, path: impl AsRef<Path>, params: ObjectParams) -> ApiResult<Value> {
        self.upload(Box::new(FileDataSource::new(path.as_ref())), params)
    }

    /// 上传输入流的数据
    fn upload_reader<R: Read + Debug + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> ApiResult<Value> {
        self.upload(Box::new(UnseekableDataSource::new(reader)), params)
    }

    /// 异步上传指定路径的文件
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
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

    /// 异步上传输入流的数据
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
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
}

impl<A: Digest + Send + 'static, T: MultiPartsUploaderScheduler<A>> MultiPartsUploaderSchedulerExt<A> for T {}

mod serial_multi_parts_uploader_scheduler;
pub use serial_multi_parts_uploader_scheduler::SerialMultiPartsUploaderScheduler;
mod concurrent_multi_parts_uploader_scheduler;
pub use concurrent_multi_parts_uploader_scheduler::ConcurrentMultiPartsUploaderScheduler;
