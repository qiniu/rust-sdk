use super::{ObjectParams, UploadManager};
use qiniu_apis::{
    http::{ResponseParts, TransferProgressInfo},
    http_client::{ApiResult, CallbackResult, RequestBuilderParts, ResponseError},
};
use serde_json::Value;
use std::{fmt::Debug, io::Read, path::Path};

#[cfg(feature = "async")]
use futures::{future::BoxFuture, AsyncRead};

pub trait SinglePartUploader: Debug {
    fn new(upload_manager: UploadManager) -> Self;
    fn on_before_request<
        F: FnMut(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'static,
    >(
        &mut self,
        callback: F,
    ) -> &mut Self;
    fn on_upload_progress<F: Fn(&TransferProgressInfo) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self;
    fn on_response_ok<F: FnMut(&mut ResponseParts) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self;
    fn on_response_error<F: FnMut(&ResponseError) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self;

    fn upload_path(&self, path: &Path, params: ObjectParams) -> ApiResult<Value>;

    fn upload_reader<R: Read + 'static>(&self, reader: R, params: ObjectParams)
        -> ApiResult<Value>;

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_path(&self, path: &Path, params: ObjectParams) -> BoxFuture<ApiResult<Value>>;

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
