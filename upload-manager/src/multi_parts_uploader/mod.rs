use super::{
    DataPartitionProvider, DataSource, MultiPartsUploaderWithCallbacks, ObjectParams, ResumableRecorder, UploadManager,
};
use digest::Digest;
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::{fmt::Debug, num::NonZeroU64};

#[cfg(feature = "async")]
use {super::AsyncDataSource, futures::future::BoxFuture};

/// 分片上传器接口
///
/// 将数据源通过多个分片的方式逐一上传，适合数据量较大的数据源，可以提供断点恢复的能力。
pub trait MultiPartsUploader: MultiPartsUploaderWithCallbacks + Clone + Send + Sync + Debug {
    /// 数据源 KEY 的哈希算法
    type HashAlgorithm: Digest + Send + 'static;

    /// 初始化的分片信息
    type InitializedParts: InitializedParts + 'static;

    /// 已经上传的分片信息
    type UploadedPart: UploadedPart;

    /// 创建分片上传器
    fn new<R: ResumableRecorder<HashAlgorithm = Self::HashAlgorithm> + 'static>(
        upload_manager: UploadManager,
        resumable_recorder: R,
    ) -> Self;

    /// 初始化分片信息
    ///
    /// 该步骤只负责初始化分片，但不实际上传数据，如果提供了有效的断点续传记录器，则可以尝试在这一步找到记录。
    ///
    /// 该方法的异步版本为 [`Self::async_initialize_parts`]。
    fn initialize_parts<D: DataSource<Self::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> ApiResult<Self::InitializedParts>;

    /// 上传分片
    ///
    /// 实际上传的分片大小由提供的分片大小提供者获取。
    ///
    /// 如果返回 [`Ok(None)`] 则表示已经没有更多分片可以上传。
    ///
    /// 该方法的异步版本为 [`Self::async_upload_part`]。
    fn upload_part(
        &self,
        initialized: &Self::InitializedParts,
        data_partitioner_provider: &dyn DataPartitionProvider,
    ) -> ApiResult<Option<Self::UploadedPart>>;

    /// 完成分片上传
    ///
    /// 在这步成功返回后，对象即可被读取。
    ///
    /// 该方法的异步版本为 [`Self::async_complete_parts`]。
    fn complete_parts(&self, initialized: &Self::InitializedParts, parts: &[Self::UploadedPart]) -> ApiResult<Value>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    /// 初始化的异步分片信息
    type AsyncInitializedParts: InitializedParts + 'static;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    /// 已经上传的异步分片信息
    type AsyncUploadedPart: UploadedPart;

    /// 异步初始化分片信息
    ///
    /// 该步骤只负责初始化分片，但不实际上传数据，如果提供了有效的断点续传记录器，则可以尝试在这一步找到记录。
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_initialize_parts<D: AsyncDataSource<Self::HashAlgorithm> + 'static>(
        &self,
        source: D,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Self::AsyncInitializedParts>>;

    /// 异步上传分片
    ///
    /// 实际上传的分片大小由提供的分片大小提供者获取。
    ///
    /// 如果返回 [`Ok(None)`] 则表示已经没有更多分片可以上传。
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_part<'r>(
        &'r self,
        initialized: &'r Self::AsyncInitializedParts,
        data_partitioner_provider: &'r dyn DataPartitionProvider,
    ) -> BoxFuture<'r, ApiResult<Option<Self::AsyncUploadedPart>>>;

    /// 异步完成分片上传
    ///
    /// 在这步成功返回后，对象即可被读取。
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_complete_parts<'r>(
        &'r self,
        initialized: &'r Self::AsyncInitializedParts,
        parts: &'r [Self::AsyncUploadedPart],
    ) -> BoxFuture<'r, ApiResult<Value>>;
}

/// 初始化的分片信息
pub trait InitializedParts: Send + Sync + Debug {
    /// 获取对象上传参数
    fn params(&self) -> &ObjectParams;
}

/// 已经上传的分片信息
pub trait UploadedPart: Send + Sync + Debug {
    /// 分片大小
    fn size(&self) -> NonZeroU64;

    /// 分片偏移量
    fn offset(&self) -> u64;

    /// 是否来自于断点恢复
    fn resumed(&self) -> bool;
}

mod v1;
pub use v1::{MultiPartsV1Uploader, MultiPartsV1UploaderInitializedObject, MultiPartsV1UploaderUploadedPart};

mod v2;
pub use v2::{MultiPartsV2Uploader, MultiPartsV2UploaderInitializedObject, MultiPartsV2UploaderUploadedPart};

mod progress;
mod up_endpoints;
