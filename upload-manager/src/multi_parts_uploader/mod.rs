use super::{ObjectParams, ResumableRecorder, UploadManager, UploaderWithCallbacks};
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::{fmt::Debug, io::Read};

#[cfg(feature = "async")]
use futures::{future::BoxFuture, AsyncRead};

pub trait MultiPartsUploader: UploaderWithCallbacks + Debug {
    type ResumableRecorder: ResumableRecorder;
    type InitializedParts;
    type UploadedPart;

    fn new(upload_manager: UploadManager, resumable_recorder: Self::ResumableRecorder) -> Self;

    fn initialize_parts(&self, params: ObjectParams) -> Self::InitializedParts;
    fn upload_part<R: Read + 'static>(
        &self,
        initialized: &Self::InitializedParts,
        reader: R,
    ) -> Self::UploadedPart;
    fn complete_parts(
        &self,
        initialized: &Self::InitializedParts,
        parts: Vec<Self::UploadedPart>,
    ) -> ApiResult<Value>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_initialize_parts(&self, params: ObjectParams) -> BoxFuture<Self::InitializedParts>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_part<'r, R: Read + 'r>(
        &'r self,
        initialized: &'r Self::InitializedParts,
        reader: R,
    ) -> BoxFuture<'r, Self::UploadedPart>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_complete_parts<'r>(
        &'r self,
        initialized: &'r Self::InitializedParts,
        parts: Vec<Self::UploadedPart>,
    ) -> BoxFuture<'r, ApiResult<Value>>;
}
