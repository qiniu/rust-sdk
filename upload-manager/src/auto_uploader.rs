use super::{
    callbacks::Callbacks, ConcurrencyProvider, ConcurrentMultiPartsUploaderScheduler, DataPartitionProvider,
    FileSystemResumableRecorder, FixedConcurrencyProvider, FixedDataPartitionProvider, FixedThresholdResumablePolicy,
    FormUploader, MultiPartsUploaderScheduler, MultiPartsUploaderSchedulerExt, MultiPartsUploaderWithCallbacks,
    MultiPartsV1Uploader, MultiPartsV2Uploader, ObjectParams, ObjectParamsBuilder, ResumablePolicy,
    ResumablePolicyProvider, ResumableRecorder, SerialMultiPartsUploaderScheduler, SinglePartUploader, UploadManager,
    UploadedPart, UploaderWithCallbacks, UploadingProgressInfo,
};
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
    pub fn upload_path(&self, path: &Path, params: impl Into<AutoUploaderObjectParams>) -> ApiResult<Value> {
        let params = params.into();
        let size = metadata(path)?.len();
        with_uploader!(
            self,
            self.resumable_policy_provider
                .get_policy_from_size(size, &Default::default()),
            params,
            sync_block,
            upload_path,
            path,
            params.into(),
        )
    }

    pub fn upload_reader<R: Read + Debug + Send + Sync + 'static>(
        &self,
        reader: R,
        params: impl Into<AutoUploaderObjectParams>,
    ) -> ApiResult<Value> {
        let params = params.into();
        let (policy, reader) = self
            .resumable_policy_provider
            .get_policy_from_reader(reader, &Default::default())?;
        with_uploader!(self, policy, params, sync_block, upload_reader, reader, params.into(),)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub async fn async_upload_path<'a>(
        &'a self,
        path: &'a Path,
        params: impl Into<AutoUploaderObjectParams>,
    ) -> ApiResult<Value> {
        let params = params.into();
        let size = async_metadata(path).await?.len();
        with_uploader!(
            self,
            self.resumable_policy_provider
                .get_policy_from_size(size, &Default::default()),
            params,
            async_block,
            async_upload_path,
            path,
            params.into(),
        )
    }

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
            .get_policy_from_async_reader(reader, &Default::default())
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

#[derive(Debug, Default)]
pub struct AutoUploaderObjectParams {
    object_params: ObjectParams,
    multi_parts_uploader_scheduler_prefer: MultiPartsUploaderSchedulerPrefer,
    single_part_uploader_prefer: SinglePartUploaderPrefer,
    multi_parts_uploader_prefer: MultiPartsUploaderPrefer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, SmartDefault)]
#[non_exhaustive]
pub enum MultiPartsUploaderSchedulerPrefer {
    Serial,
    #[default]
    Concurrent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, SmartDefault)]
#[non_exhaustive]
pub enum SinglePartUploaderPrefer {
    #[default]
    Form,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, SmartDefault)]
#[non_exhaustive]
pub enum MultiPartsUploaderPrefer {
    V1,
    #[default]
    V2,
}

impl AutoUploaderObjectParams {
    #[inline]
    pub fn builder() -> AutoUploaderObjectParamsBuilder {
        Default::default()
    }

    #[inline]
    pub fn multi_parts_uploader_scheduler_prefer(&self) -> MultiPartsUploaderSchedulerPrefer {
        self.multi_parts_uploader_scheduler_prefer
    }

    #[inline]
    pub fn multi_parts_uploader_scheduler_prefer_mut(&mut self) -> &mut MultiPartsUploaderSchedulerPrefer {
        &mut self.multi_parts_uploader_scheduler_prefer
    }

    #[inline]
    pub fn single_part_uploader_prefer(&self) -> SinglePartUploaderPrefer {
        self.single_part_uploader_prefer
    }

    #[inline]
    pub fn single_part_uploader_prefer_mut(&mut self) -> &mut SinglePartUploaderPrefer {
        &mut self.single_part_uploader_prefer
    }

    #[inline]
    pub fn multi_parts_uploader_prefer(&self) -> MultiPartsUploaderPrefer {
        self.multi_parts_uploader_prefer
    }

    #[inline]
    pub fn multi_parts_uploader_prefer_mut(&mut self) -> &mut MultiPartsUploaderPrefer {
        &mut self.multi_parts_uploader_prefer
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
    #[inline]
    pub fn multi_parts_uploader_scheduler_prefer(
        &mut self,
        multi_parts_uploader_scheduler_prefer: MultiPartsUploaderSchedulerPrefer,
    ) -> &mut Self {
        self.multi_parts_uploader_scheduler_prefer = multi_parts_uploader_scheduler_prefer;
        self
    }

    #[inline]
    pub fn single_part_uploader_prefer(&mut self, single_part_uploader_prefer: SinglePartUploaderPrefer) -> &mut Self {
        self.single_part_uploader_prefer = single_part_uploader_prefer;
        self
    }

    #[inline]
    pub fn multi_parts_uploader_prefer(&mut self, multi_parts_uploader_prefer: MultiPartsUploaderPrefer) -> &mut Self {
        self.multi_parts_uploader_prefer = multi_parts_uploader_prefer;
        self
    }

    #[inline]
    pub fn build(&mut self) -> AutoUploaderObjectParams {
        AutoUploaderObjectParams {
            object_params: self.object_params_builder.build(),
            multi_parts_uploader_scheduler_prefer: self.multi_parts_uploader_scheduler_prefer,
            single_part_uploader_prefer: self.single_part_uploader_prefer,
            multi_parts_uploader_prefer: self.multi_parts_uploader_prefer,
        }
    }
}

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
    #[inline]
    pub fn concurrency_provider(mut self, concurrency_provider: CP) -> Self {
        self.concurrency_provider = concurrency_provider;
        self
    }

    #[inline]
    pub fn data_partition_provider(mut self, data_partition_provider: DPP) -> Self {
        self.data_partition_provider = data_partition_provider;
        self
    }

    #[inline]
    pub fn resumable_recorder(mut self, resumable_recorder: RR) -> Self {
        self.resumable_recorder = resumable_recorder;
        self
    }

    #[inline]
    pub fn resumable_policy_provider(mut self, resumable_policy_provider: RPP) -> Self {
        self.resumable_policy_provider = resumable_policy_provider;
        self
    }
}

impl<CP, DPP, RR, RPP> AutoUploaderBuilder<CP, DPP, RR, RPP> {}

impl<CP, DPP, RR, RPP> AutoUploaderBuilder<CP, DPP, RR, RPP> {
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
