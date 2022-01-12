use super::{
    DataPartitionProvider, DataSource, ObjectParams, ResumableRecorder, UploadManager,
    UploaderWithCallbacks,
};
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::{fmt::Debug, time::Duration};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

pub trait MultiPartsUploader: UploaderWithCallbacks + Debug {
    type ResumableRecorder: ResumableRecorder;
    type InitializedParts;
    type UploadedPart;

    fn new(upload_manager: UploadManager, resumable_recorder: Self::ResumableRecorder) -> Self;
    fn uploaded_part_ttl(&self) -> Duration;
    fn uploaded_part_lifetime_mut(&mut self) -> &mut Duration;

    fn initialize_parts<
        D: DataSource<<Self::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static,
    >(
        &self,
        source: D,
        params: ObjectParams,
    ) -> ApiResult<Self::InitializedParts>;
    fn upload_part(
        &self,
        initialized: &Self::InitializedParts,
        data_partitioner_provider: &dyn DataPartitionProvider,
    ) -> ApiResult<Option<Self::UploadedPart>>;
    fn complete_parts(
        &self,
        initialized: Self::InitializedParts,
        parts: Vec<Self::UploadedPart>,
    ) -> ApiResult<Value>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_initialize_parts<
        D: DataSource<<Self::ResumableRecorder as ResumableRecorder>::HashAlgorithm> + 'static,
    >(
        &self,
        source: D,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Self::InitializedParts>>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_part<'r>(
        &'r self,
        initialized: &'r Self::InitializedParts,
        data_partitioner_provider: &'r dyn DataPartitionProvider,
    ) -> BoxFuture<'r, ApiResult<Option<Self::UploadedPart>>>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_complete_parts(
        &self,
        initialized: Self::InitializedParts,
        parts: Vec<Self::UploadedPart>,
    ) -> BoxFuture<'_, ApiResult<Value>>;
}

mod v1;
pub use v1::{
    MultiPartsV1Uploader, MultiPartsV1UploaderInitializedObject, MultiPartsV1UploaderUploadedPart,
};

mod progress;
