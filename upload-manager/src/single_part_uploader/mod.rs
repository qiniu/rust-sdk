use super::{ObjectParams, UploadManager, UploadingProgressInfo};
use qiniu_apis::{
    http::ResponseParts,
    http_client::{ApiResult, CallbackResult, RequestBuilderParts, ResponseError},
};
use serde_json::Value;
use std::{fmt::Debug, io::Read, path::Path};

#[cfg(feature = "async")]
use futures::{future::BoxFuture, AsyncRead};

pub trait SinglePartUploader: Debug {
    fn new(upload_manager: UploadManager) -> Self;
    fn on_before_request<
        F: Fn(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'static,
    >(
        &mut self,
        callback: F,
    ) -> &mut Self;
    fn on_upload_progress<F: Fn(&UploadingProgressInfo) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self;
    fn on_response_ok<F: Fn(&mut ResponseParts) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self;
    fn on_response_error<F: Fn(&ResponseError) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self;

    fn upload_path(&self, path: &Path, params: ObjectParams) -> ApiResult<Value>;

    fn upload_reader<R: Read + 'static>(&self, reader: R, params: ObjectParams)
        -> ApiResult<Value>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_path<'a>(
        &'a self,
        path: &'a Path,
        params: ObjectParams,
    ) -> BoxFuture<'a, ApiResult<Value>>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_reader<R: AsyncRead + Unpin + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>>;
}

mod form_uploader;
pub use form_uploader::FormUploader;
