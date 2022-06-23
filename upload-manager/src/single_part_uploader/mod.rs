use super::{ObjectParams, UploadManager, UploaderWithCallbacks};
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::{fmt::Debug, io::Read, path::Path};

#[cfg(feature = "async")]
use futures::{future::BoxFuture, AsyncRead};

/// 单请求上传器接口
///
/// 仅通过一次 HTTP 请求即上传整个数据源，适合数据量较小的数据源，不提供断点恢复的能力。
pub trait SinglePartUploader: UploaderWithCallbacks + Clone + Sync + Send + Debug {
    /// 创建单请求上传器
    fn new(upload_manager: UploadManager) -> Self;

    /// 上传指定路径的文件
    ///
    /// 该方法的异步版本为 [`Self::async_upload_path`]。
    fn upload_path(&self, path: impl AsRef<Path>, params: ObjectParams) -> ApiResult<Value>;

    /// 上传输入流的数据
    ///
    /// 该方法的异步版本为 [`Self::async_upload_reader`]。
    fn upload_reader<R: Read + 'static>(&self, reader: R, params: ObjectParams) -> ApiResult<Value>;

    /// 异步上传指定路径的文件
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_path<'a>(
        &'a self,
        path: impl AsRef<Path> + Send + Sync + 'a,
        params: ObjectParams,
    ) -> BoxFuture<'a, ApiResult<Value>>;

    /// 上传异步输入流的数据
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
