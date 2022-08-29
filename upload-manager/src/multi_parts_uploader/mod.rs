use super::{
    DataPartitionProvider, DataSource, MultiPartsUploaderWithCallbacks, ObjectParams, ResumableRecorder, UploadManager,
};
use digest::Digest;
use qiniu_apis::http_client::{ApiResult, RegionsProvider};
use serde_json::Value;
use smart_default::SmartDefault;
use std::{fmt::Debug, num::NonZeroU64};

#[cfg(feature = "async")]
use {super::AsyncDataSource, futures::future::BoxFuture};

pub(super) struct PartsExpiredError;

/// 分片上传器接口
///
/// 将数据源通过多个分片的方式逐一上传，适合数据量较大的数据源，可以提供断点恢复的能力。
pub trait MultiPartsUploader:
    __private::Sealed + MultiPartsUploaderWithCallbacks + Clone + Send + Sync + Debug
{
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

    /// 重新初始化分片信息
    ///
    /// 该步骤负责将先前已经初始化过的分片信息全部重置，清空断点续传记录器中的记录，之后从头上传整个文件
    ///
    /// 该方法的异步版本为 [`Self::async_reinitialize_parts`]。
    fn reinitialize_parts(
        &self,
        initialized: &mut Self::InitializedParts,
        options: ReinitializeOptions,
    ) -> ApiResult<()>;

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

    /// 异步重新初始化分片信息
    ///
    /// 该步骤负责将先前已经初始化过的分片信息全部重置，清空断点续传记录器中的记录，之后从头上传整个文件
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_reinitialize_parts<'r>(
        &'r self,
        initialized: &'r mut Self::AsyncInitializedParts,
        options: ReinitializeOptions,
    ) -> BoxFuture<'r, ApiResult<()>>;
}

/// 初始化的分片信息
pub trait InitializedParts: __private::Sealed + Clone + Send + Sync + Debug {
    /// 获取对象上传参数
    fn params(&self) -> &ObjectParams;
}

/// 已经上传的分片信息
pub trait UploadedPart: __private::Sealed + Send + Sync + Debug {
    /// 分片大小
    fn size(&self) -> NonZeroU64;

    /// 分片偏移量
    fn offset(&self) -> u64;

    /// 是否来自于断点恢复
    fn resumed(&self) -> bool;
}

/// 重新初始化分片信息的选项
#[derive(Debug, Clone, Default)]
pub struct ReinitializeOptions {
    endpoints_provider: ReinitializedUpEndpointsProvider,
}

impl ReinitializeOptions {
    /// 创建重新初始化分片信息的选项构建器
    #[inline]
    pub fn builder() -> ReinitializeOptionsBuilder {
        ReinitializeOptionsBuilder(Self::default())
    }
}

#[derive(Debug, Clone, SmartDefault)]
enum ReinitializedUpEndpointsProvider {
    #[default]
    KeepOriginalUpEndpoints,
    RefreshUpEndpoints,
    SpecifiedRegionsProvider(Box<dyn RegionsProvider>),
}

/// 重新初始化分片信息的选项构建器
#[derive(Debug, Clone)]
pub struct ReinitializeOptionsBuilder(ReinitializeOptions);

impl ReinitializeOptionsBuilder {
    /// 复用先前的区域信息
    #[inline]
    pub fn keep_original_region(&mut self) -> &mut Self {
        self.0.endpoints_provider = ReinitializedUpEndpointsProvider::KeepOriginalUpEndpoints;
        self
    }

    /// 刷新区域信息
    #[inline]
    pub fn refresh_regions(&mut self) -> &mut Self {
        self.0.endpoints_provider = ReinitializedUpEndpointsProvider::RefreshUpEndpoints;
        self
    }

    /// 指定区域信息
    #[inline]
    pub fn regions_provider(&mut self, regions: impl RegionsProvider + 'static) -> &mut Self {
        self.0.endpoints_provider = ReinitializedUpEndpointsProvider::SpecifiedRegionsProvider(Box::new(regions));
        self
    }

    /// 构建重新初始化分片信息的选项
    #[inline]
    pub fn build(&self) -> ReinitializeOptions {
        self.0.to_owned()
    }
}

mod v1;
pub use v1::{MultiPartsV1Uploader, MultiPartsV1UploaderInitializedObject, MultiPartsV1UploaderUploadedPart};

mod v2;
pub use v2::{MultiPartsV2Uploader, MultiPartsV2UploaderInitializedObject, MultiPartsV2UploaderUploadedPart};

mod progress;

mod __private {
    pub trait Sealed {}
}
