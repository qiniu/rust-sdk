use super::{ObjectParams, UploadManager, UploaderWithCallbacks};
use qiniu_apis::http_client::ApiResult;
use serde_json::Value;
use std::{
    fmt::Debug,
    io::{Read, Seek},
    path::Path,
};

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
use futures::{future::BoxFuture, AsyncRead, AsyncSeek};

/// 单请求上传器接口
///
/// 仅通过一次 HTTP 请求即上传整个数据源，适合数据量较小的数据源，不提供断点恢复的能力。
pub trait SinglePartUploader: __private::Sealed + UploaderWithCallbacks + Clone + Sync + Send + Debug {
    /// 创建单请求上传器
    fn new(upload_manager: UploadManager) -> Self;

    /// 上传指定路径的文件
    ///
    /// 该方法的异步版本为 [`Self::async_upload_path`]。
    fn upload_path(&self, path: impl AsRef<Path>, params: ObjectParams) -> ApiResult<Value>;

    /// 上传不可寻址的输入流的数据
    ///
    /// 该方法的异步版本为 [`Self::async_upload_reader`]。
    fn upload_reader<R: Read + Send + Sync>(&self, reader: R, params: ObjectParams) -> ApiResult<Value>;

    /// 上传可寻址的输入流的数据
    ///
    /// 该方法的异步版本为 [`Self::async_upload_seekable_reader`]。
    fn upload_seekable_reader<R: Read + Seek + Send + Sync>(&self, reader: R, params: ObjectParams)
        -> ApiResult<Value>;

    /// 异步上传指定路径的文件
    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
    )]
    fn async_upload_path<'a>(
        &'a self,
        path: impl AsRef<Path> + Send + Sync + 'a,
        params: ObjectParams,
    ) -> BoxFuture<'a, ApiResult<Value>>;

    /// 上传不可寻址的异步输入流的数据
    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
    )]
    fn async_upload_reader<R: AsyncRead + Unpin + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>>;

    /// 上传可寻址的异步输入流的数据
    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
    )]
    fn async_upload_seekable_reader<R: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>>;
}

mod form_uploader;
pub use form_uploader::FormUploader;

mod __private {
    pub trait Sealed {}
}
