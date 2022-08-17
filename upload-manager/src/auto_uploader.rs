use super::{
    callbacks::Callbacks, ConcurrencyProvider, ConcurrentMultiPartsUploaderScheduler, DataPartitionProvider,
    FileSystemResumableRecorder, FixedConcurrencyProvider, FixedDataPartitionProvider, FixedThresholdResumablePolicy,
    FormUploader, MultiPartsUploaderScheduler, MultiPartsUploaderSchedulerExt, MultiPartsUploaderWithCallbacks,
    MultiPartsV1Uploader, MultiPartsV2Uploader, ObjectParams, ObjectParamsBuilder, ResumablePolicy,
    ResumablePolicyProvider, ResumableRecorder, SerialMultiPartsUploaderScheduler, SinglePartUploader, UploadManager,
    UploadedPart, UploaderWithCallbacks, UploadingProgressInfo,
};
use assert_impl::assert_impl;
use qiniu_apis::{
    http::ResponseParts,
    http_client::{ApiResult, CallbackResult, RequestBuilderParts, ResponseError},
};
use serde_json::Value;
use smart_default::SmartDefault;
use std::{
    fmt::Debug,
    fs::metadata,
    io::Read,
    ops::{Deref, DerefMut},
    path::Path,
    sync::Arc,
};

#[cfg(feature = "async")]
use {async_std::fs::metadata as async_metadata, futures::AsyncRead};

/// 自动上传器
///
/// 使用设置的各种提供者，将文件或是二进制流数据上传。
///
/// 该类型包含多个泛型参数，
/// `CP` 表示并发数提供者，默认为固定并发数提供者；
/// `DPP` 表示分片大小提供者，默认为固定分片大小提供者；
/// `RR` 表示断点恢复记录器，默认为文件系统断点恢复记录器；
/// `RPP` 表示可恢复策略，默认为固定阀值可恢复策略。
///
/// ### 用自动上传器上传文件
///
/// ##### 阻塞代码示例
///
/// ```
/// use qiniu_upload_manager::{
///     apis::credential::Credential, AutoUploader, AutoUploaderObjectParams, UploadManager,
///     UploadTokenSigner,
/// };
/// use std::time::Duration;
///
/// # fn example() -> anyhow::Result<()> {
/// let bucket_name = "test-bucket";
/// let object_name = "test-object";
/// let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
///     Credential::new("abcdefghklmnopq", "1234567890"),
///     bucket_name,
///     Duration::from_secs(3600),
/// ))
/// .build();
/// let params = AutoUploaderObjectParams::builder().object_name(object_name).file_name(object_name).build();
/// let uploader: AutoUploader = upload_manager.auto_uploader();
/// uploader.upload_path("/home/qiniu/test.png", params)?;
/// # Ok(())
/// # }
/// ```
///
/// ##### 异步代码示例
///
/// ```
/// use qiniu_upload_manager::{
///     apis::credential::Credential, AutoUploader, AutoUploaderObjectParams, UploadManager,
///     UploadTokenSigner,
/// };
/// use std::time::Duration;
///
/// # async fn example() -> anyhow::Result<()> {
/// let bucket_name = "test-bucket";
/// let object_name = "test-object";
/// let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
///     Credential::new("abcdefghklmnopq", "1234567890"),
///     bucket_name,
///     Duration::from_secs(3600),
/// ))
/// .build();
/// let params = AutoUploaderObjectParams::builder().object_name(object_name).file_name(object_name).build();
/// let uploader: AutoUploader = upload_manager.auto_uploader();
/// uploader.async_upload_path("/home/qiniu/test.png", params).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct AutoUploader<
    CP = FixedConcurrencyProvider,
    DPP = FixedDataPartitionProvider,
    RR = FileSystemResumableRecorder,
    RPP = FixedThresholdResumablePolicy,
> {
    upload_manager: UploadManager,
    callbacks: Callbacks<'static>,
    concurrency_provider: Arc<CP>,
    data_partition_provider: Arc<DPP>,
    resumable_recorder: Arc<RR>,
    resumable_policy_provider: RPP,
}

impl<CP: Default, DPP: Default, RR: Default, RPP: Default> AutoUploader<CP, DPP, RR, RPP> {
    /// 创建自动上传器
    #[inline]
    pub fn new(upload_manager: UploadManager) -> Self {
        Self {
            upload_manager,
            callbacks: Default::default(),
            concurrency_provider: Default::default(),
            data_partition_provider: Default::default(),
            resumable_recorder: Default::default(),
            resumable_policy_provider: Default::default(),
        }
    }

    /// 构建自动上传构建器
    #[inline]
    pub fn builder(upload_manager: UploadManager) -> AutoUploaderBuilder<CP, DPP, RR, RPP> {
        AutoUploaderBuilder {
            upload_manager,
            callbacks: Default::default(),
            concurrency_provider: Default::default(),
            data_partition_provider: Default::default(),
            resumable_recorder: Default::default(),
            resumable_policy_provider: Default::default(),
        }
    }
}

impl<CP: ConcurrencyProvider, DPP: DataPartitionProvider, RR: ResumableRecorder, RPP: ResumablePolicyProvider>
    UploaderWithCallbacks for AutoUploader<CP, DPP, RR, RPP>
{
    fn on_before_request<F: Fn(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_before_request_callback(callback);
        self
    }

    fn on_upload_progress<F: Fn(&UploadingProgressInfo) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_upload_progress_callback(callback);
        self
    }

    fn on_response_ok<F: Fn(&mut ResponseParts) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_after_response_ok_callback(callback);
        self
    }

    fn on_response_error<F: Fn(&ResponseError) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_after_response_error_callback(callback);
        self
    }
}

impl<CP: ConcurrencyProvider, DPP: DataPartitionProvider, RR: ResumableRecorder, RPP: ResumablePolicyProvider>
    MultiPartsUploaderWithCallbacks for AutoUploader<CP, DPP, RR, RPP>
{
    fn on_part_uploaded<F: Fn(&dyn UploadedPart) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_part_uploaded_callback(callback);
        self
    }
}

macro_rules! sync_block {
    ($code:block) => {
        $code
    };
}

#[cfg(feature = "async")]
macro_rules! async_block {
    ($code:block) => {
        $code.await
    };
}

macro_rules! with_uploader {
    ($uploader:ident, $resumable_policy:expr, $params:expr, $wrapper:ident, $method:ident, $($args:expr,)*) => {
        match $resumable_policy {
            ResumablePolicy::SinglePartUploading => match $params.single_part_uploader_prefer() {
                SinglePartUploaderPrefer::Form => {
                    let uploader = FormUploader::new_with_callbacks(
                        $uploader.upload_manager.to_owned(),
                        $uploader.callbacks.to_owned(),
                    );
                    $wrapper!({uploader.$method($($args),*)})
                }
            },
            ResumablePolicy::MultiPartsUploading => {
                match (
                    $params.multi_parts_uploader_prefer(),
                    $params.multi_parts_uploader_scheduler_prefer(),
                ) {
                    (MultiPartsUploaderPrefer::V1, MultiPartsUploaderSchedulerPrefer::Concurrent) => {
                        let mut uploader =
                            ConcurrentMultiPartsUploaderScheduler::new(MultiPartsV1Uploader::new_with_callbacks(
                                $uploader.upload_manager.to_owned(),
                                $uploader.callbacks.to_owned(),
                                $uploader.resumable_recorder.to_owned(),
                            ));
                        uploader.set_concurrency_provider($uploader.concurrency_provider.to_owned());
                        uploader.set_data_partition_provider($uploader.data_partition_provider.to_owned());
                        $wrapper!({uploader.$method($($args),*)})
                    }
                    (MultiPartsUploaderPrefer::V1, MultiPartsUploaderSchedulerPrefer::Serial) => {
                        let mut uploader =
                            SerialMultiPartsUploaderScheduler::new(MultiPartsV1Uploader::new_with_callbacks(
                                $uploader.upload_manager.to_owned(),
                                $uploader.callbacks.to_owned(),
                                $uploader.resumable_recorder.to_owned(),
                            ));
                        uploader.set_concurrency_provider($uploader.concurrency_provider.to_owned());
                        uploader.set_data_partition_provider($uploader.data_partition_provider.to_owned());
                        $wrapper!({uploader.$method($($args),*)})
                    }
                    (MultiPartsUploaderPrefer::V2, MultiPartsUploaderSchedulerPrefer::Concurrent) => {
                        let mut uploader =
                            ConcurrentMultiPartsUploaderScheduler::new(MultiPartsV2Uploader::new_with_callbacks(
                                $uploader.upload_manager.to_owned(),
                                $uploader.callbacks.to_owned(),
                                $uploader.resumable_recorder.to_owned(),
                            ));
                        uploader.set_concurrency_provider($uploader.concurrency_provider.to_owned());
                        uploader.set_data_partition_provider($uploader.data_partition_provider.to_owned());
                        $wrapper!({uploader.$method($($args),*)})
                    }
                    (MultiPartsUploaderPrefer::V2, MultiPartsUploaderSchedulerPrefer::Serial) => {
                        let mut uploader =
                            SerialMultiPartsUploaderScheduler::new(MultiPartsV2Uploader::new_with_callbacks(
                                $uploader.upload_manager.to_owned(),
                                $uploader.callbacks.to_owned(),
                                $uploader.resumable_recorder.to_owned(),
                            ));
                        uploader.set_concurrency_provider($uploader.concurrency_provider.to_owned());
                        uploader.set_data_partition_provider($uploader.data_partition_provider.to_owned());
                        $wrapper!({uploader.$method($($args),*)})
                    }
                }
            }
        }
    };
}

impl<
        CP: ConcurrencyProvider + 'static,
        DPP: DataPartitionProvider + 'static,
        RR: ResumableRecorder + 'static,
        RPP: ResumablePolicyProvider,
    > AutoUploader<CP, DPP, RR, RPP>
where
    RR::HashAlgorithm: Send,
{
    /// 阻塞上传指定路径的文件
    ///
    /// 该方法的异步版本为 [`Self::async_upload_path`]。
    pub fn upload_path(&self, path: impl AsRef<Path>, params: impl Into<AutoUploaderObjectParams>) -> ApiResult<Value> {
        let params = params.into();
        let size = metadata(path.as_ref())?.len();
        with_uploader!(
            self,
            self.resumable_policy_provider
                .get_policy_from_size(size, Default::default()),
            params,
            sync_block,
            upload_path,
            path.as_ref(),
            params.into(),
        )
    }

    /// 阻塞上传阅读器的数据
    ///
    /// 该方法的异步版本为 [`Self::async_upload_reader`]。
    pub fn upload_reader<R: Read + Debug + Send + Sync + 'static>(
        &self,
        reader: R,
        params: impl Into<AutoUploaderObjectParams>,
    ) -> ApiResult<Value> {
        let params = params.into();
        let (policy, reader) = self
            .resumable_policy_provider
            .get_policy_from_reader(reader, Default::default())?;
        with_uploader!(self, policy, params, sync_block, upload_reader, reader, params.into(),)
    }

    /// 异步上传指定路径的文件
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub async fn async_upload_path<'a>(
        &'a self,
        path: impl AsRef<Path> + Send + Sync + 'a,
        params: impl Into<AutoUploaderObjectParams>,
    ) -> ApiResult<Value> {
        let params = params.into();
        let size = async_metadata(path.as_ref()).await?.len();
        with_uploader!(
            self,
            self.resumable_policy_provider
                .get_policy_from_size(size, Default::default()),
            params,
            async_block,
            async_upload_path,
            path.as_ref(),
            params.into(),
        )
    }

    /// 异步上传阅读器的数据
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub async fn async_upload_reader<R: AsyncRead + Unpin + Debug + Send + Sync + 'static>(
        &self,
        reader: R,
        params: impl Into<AutoUploaderObjectParams>,
    ) -> ApiResult<Value> {
        let params = params.into();
        let (policy, reader) = self
            .resumable_policy_provider
            .get_policy_from_async_reader(reader, Default::default())
            .await?;
        with_uploader!(
            self,
            policy,
            params,
            async_block,
            async_upload_reader,
            reader,
            params.into(),
        )
    }
}

impl<CP: Sync + Send, DPP: Sync + Send, RR: Sync + Send, RPP: Sync + Send> AutoUploader<CP, DPP, RR, RPP> {
    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// 自动上传器对象参数
#[derive(Debug, Default)]
pub struct AutoUploaderObjectParams {
    object_params: ObjectParams,
    multi_parts_uploader_scheduler_prefer: MultiPartsUploaderSchedulerPrefer,
    single_part_uploader_prefer: SinglePartUploaderPrefer,
    multi_parts_uploader_prefer: MultiPartsUploaderPrefer,
}

/// 期望的分片上传调度器
#[derive(Clone, Copy, Debug, PartialEq, Eq, SmartDefault)]
#[non_exhaustive]
pub enum MultiPartsUploaderSchedulerPrefer {
    /// 串行上传调度器
    ///
    /// 即 [`crate::SerialMultiPartsUploaderScheduler`]。
    ///
    /// 使用该方式，则始终使用单并发上传，不会使用 [`crate::DataPartitionProvider`] 的值。
    Serial,

    /// 并行上传调度器
    ///
    /// 即 [`crate::ConcurrentMultiPartsUploaderScheduler`]。
    #[default]
    Concurrent,
}

/// 期望的对象单请求上传器
#[derive(Clone, Copy, Debug, PartialEq, Eq, SmartDefault)]
#[non_exhaustive]
pub enum SinglePartUploaderPrefer {
    /// 表单上传器
    ///
    /// 即 [`crate::FormUploader`]。
    #[default]
    Form,
}

/// 期望的对象分片上传器
#[derive(Clone, Copy, Debug, PartialEq, Eq, SmartDefault)]
#[non_exhaustive]
pub enum MultiPartsUploaderPrefer {
    /// 分片上传器 V1
    ///
    /// 即 [`crate::MultiPartsV1Uploader`]。
    V1,

    /// 分片上传器 V2
    ///
    /// 即 [`crate::MultiPartsV2Uploader`]。
    #[default]
    V2,
}

impl AutoUploaderObjectParams {
    /// 创建自动上传器对象参数构建器
    #[inline]
    pub fn builder() -> AutoUploaderObjectParamsBuilder {
        Default::default()
    }

    /// 获取期望的分片上传调度器
    #[inline]
    pub fn multi_parts_uploader_scheduler_prefer(&self) -> MultiPartsUploaderSchedulerPrefer {
        self.multi_parts_uploader_scheduler_prefer
    }

    /// 获取期望的分片上传调度器的可变引用
    #[inline]
    pub fn multi_parts_uploader_scheduler_prefer_mut(&mut self) -> &mut MultiPartsUploaderSchedulerPrefer {
        &mut self.multi_parts_uploader_scheduler_prefer
    }

    /// 期望的对象单请求上传器
    #[inline]
    pub fn single_part_uploader_prefer(&self) -> SinglePartUploaderPrefer {
        self.single_part_uploader_prefer
    }

    /// 期望的对象单请求上传器的可变引用
    #[inline]
    pub fn single_part_uploader_prefer_mut(&mut self) -> &mut SinglePartUploaderPrefer {
        &mut self.single_part_uploader_prefer
    }

    /// 期望的对象分片上传器
    #[inline]
    pub fn multi_parts_uploader_prefer(&self) -> MultiPartsUploaderPrefer {
        self.multi_parts_uploader_prefer
    }

    /// 期望的对象分片上传器的可变引用
    #[inline]
    pub fn multi_parts_uploader_prefer_mut(&mut self) -> &mut MultiPartsUploaderPrefer {
        &mut self.multi_parts_uploader_prefer
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl Deref for AutoUploaderObjectParams {
    type Target = ObjectParams;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.object_params
    }
}

impl DerefMut for AutoUploaderObjectParams {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.object_params
    }
}

impl From<ObjectParams> for AutoUploaderObjectParams {
    #[inline]
    fn from(object_params: ObjectParams) -> Self {
        Self {
            object_params,
            multi_parts_uploader_scheduler_prefer: Default::default(),
            single_part_uploader_prefer: Default::default(),
            multi_parts_uploader_prefer: Default::default(),
        }
    }
}

impl From<AutoUploaderObjectParams> for ObjectParams {
    #[inline]
    fn from(auto_uploader_object_params: AutoUploaderObjectParams) -> Self {
        auto_uploader_object_params.object_params
    }
}

/// 自动上传器对象参数构建器
#[derive(Debug, Default)]
pub struct AutoUploaderObjectParamsBuilder {
    object_params_builder: ObjectParamsBuilder,
    multi_parts_uploader_scheduler_prefer: MultiPartsUploaderSchedulerPrefer,
    single_part_uploader_prefer: SinglePartUploaderPrefer,
    multi_parts_uploader_prefer: MultiPartsUploaderPrefer,
}

impl Deref for AutoUploaderObjectParamsBuilder {
    type Target = ObjectParamsBuilder;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.object_params_builder
    }
}

impl DerefMut for AutoUploaderObjectParamsBuilder {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.object_params_builder
    }
}

impl AutoUploaderObjectParamsBuilder {
    /// 设置期望的分片上传调度器
    #[inline]
    pub fn multi_parts_uploader_scheduler_prefer(
        &mut self,
        multi_parts_uploader_scheduler_prefer: MultiPartsUploaderSchedulerPrefer,
    ) -> &mut Self {
        self.multi_parts_uploader_scheduler_prefer = multi_parts_uploader_scheduler_prefer;
        self
    }

    /// 设置对象单请求上传器
    #[inline]
    pub fn single_part_uploader_prefer(&mut self, single_part_uploader_prefer: SinglePartUploaderPrefer) -> &mut Self {
        self.single_part_uploader_prefer = single_part_uploader_prefer;
        self
    }

    /// 设置对象分片上传器
    #[inline]
    pub fn multi_parts_uploader_prefer(&mut self, multi_parts_uploader_prefer: MultiPartsUploaderPrefer) -> &mut Self {
        self.multi_parts_uploader_prefer = multi_parts_uploader_prefer;
        self
    }

    /// 构建自动上传器对象参数
    #[inline]
    pub fn build(&mut self) -> AutoUploaderObjectParams {
        AutoUploaderObjectParams {
            object_params: self.object_params_builder.build(),
            multi_parts_uploader_scheduler_prefer: self.multi_parts_uploader_scheduler_prefer,
            single_part_uploader_prefer: self.single_part_uploader_prefer,
            multi_parts_uploader_prefer: self.multi_parts_uploader_prefer,
        }
    }

    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// 自动上传构建器
#[derive(Debug)]
pub struct AutoUploaderBuilder<
    CP = FixedConcurrencyProvider,
    DPP = FixedDataPartitionProvider,
    RR = FileSystemResumableRecorder,
    RPP = FixedThresholdResumablePolicy,
> {
    upload_manager: UploadManager,
    callbacks: Callbacks<'static>,
    concurrency_provider: CP,
    data_partition_provider: DPP,
    resumable_recorder: RR,
    resumable_policy_provider: RPP,
}

impl<CP, DPP, RR, RPP> AutoUploaderBuilder<CP, DPP, RR, RPP> {
    /// 设置并发数提供者
    #[inline]
    pub fn concurrency_provider<CP2>(self, concurrency_provider: CP2) -> AutoUploaderBuilder<CP2, DPP, RR, RPP> {
        AutoUploaderBuilder {
            upload_manager: self.upload_manager,
            callbacks: self.callbacks,
            concurrency_provider,
            data_partition_provider: self.data_partition_provider,
            resumable_recorder: self.resumable_recorder,
            resumable_policy_provider: self.resumable_policy_provider,
        }
    }

    /// 设置分片大小提供者
    #[inline]
    pub fn data_partition_provider<DPP2>(
        self,
        data_partition_provider: DPP2,
    ) -> AutoUploaderBuilder<CP, DPP2, RR, RPP> {
        AutoUploaderBuilder {
            upload_manager: self.upload_manager,
            callbacks: self.callbacks,
            concurrency_provider: self.concurrency_provider,
            data_partition_provider,
            resumable_recorder: self.resumable_recorder,
            resumable_policy_provider: self.resumable_policy_provider,
        }
    }

    /// 设置断点恢复记录器
    #[inline]
    pub fn resumable_recorder<RR2>(self, resumable_recorder: RR2) -> AutoUploaderBuilder<CP, DPP, RR2, RPP> {
        AutoUploaderBuilder {
            upload_manager: self.upload_manager,
            callbacks: self.callbacks,
            concurrency_provider: self.concurrency_provider,
            data_partition_provider: self.data_partition_provider,
            resumable_recorder,
            resumable_policy_provider: self.resumable_policy_provider,
        }
    }

    /// 设置可恢复策略
    #[inline]
    pub fn resumable_policy_provider<RPP2>(
        self,
        resumable_policy_provider: RPP2,
    ) -> AutoUploaderBuilder<CP, DPP, RR, RPP2> {
        AutoUploaderBuilder {
            upload_manager: self.upload_manager,
            callbacks: self.callbacks,
            concurrency_provider: self.concurrency_provider,
            data_partition_provider: self.data_partition_provider,
            resumable_recorder: self.resumable_recorder,
            resumable_policy_provider,
        }
    }
}

impl<CP, DPP, RR, RPP> AutoUploaderBuilder<CP, DPP, RR, RPP> {
    /// 构建上传提供者
    #[inline]
    pub fn build(self) -> AutoUploader<CP, DPP, RR, RPP> {
        AutoUploader {
            upload_manager: self.upload_manager,
            callbacks: self.callbacks,
            resumable_policy_provider: self.resumable_policy_provider,
            concurrency_provider: Arc::new(self.concurrency_provider),
            data_partition_provider: Arc::new(self.data_partition_provider),
            resumable_recorder: Arc::new(self.resumable_recorder),
        }
    }
}

impl<CP: Sync + Send, DPP: Sync + Send, RR: Sync + Send, RPP: Sync + Send> AutoUploaderBuilder<CP, DPP, RR, RPP> {
    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}
