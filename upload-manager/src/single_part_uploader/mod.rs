use super::{ObjectParams, UploadManager, UploaderWithCallbacks};
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::{fmt::Debug, io::Read, path::Path};

#[cfg(feature = "async")]
use futures::{future::BoxFuture, AsyncRead};

pub trait SinglePartUploader: UploaderWithCallbacks + Sync + Send + Debug {
    fn new(upload_manager: UploadManager) -> Self;

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
