use super::UploadedPart;
use anyhow::Result as AnyResult;
use qiniu_apis::{
    http::{ResponseParts, TransferProgressInfo},
    http_client::{RequestBuilderParts, Response, ResponseError},
};
use std::{
    fmt::{self, Debug},
    sync::Arc,
};

type BeforeRequestCallback<'c> = Arc<dyn Fn(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'c>;
type UploadProgressCallback<'c> = Arc<dyn Fn(&UploadingProgressInfo) -> AnyResult<()> + Send + Sync + 'c>;
type PartUploadedCallback<'c> = Arc<dyn Fn(&dyn UploadedPart) -> AnyResult<()> + Send + Sync + 'c>;
type AfterResponseOkCallback<'c> = Arc<dyn Fn(&mut ResponseParts) -> AnyResult<()> + Send + Sync + 'c>;
type AfterResponseErrorCallback<'c> = Arc<dyn Fn(&ResponseError) -> AnyResult<()> + Send + Sync + 'c>;

/// 上传回调函数提供者
pub trait UploaderWithCallbacks {
    /// 设置请求前的回调函数
    fn on_before_request<F: Fn(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self;

    /// 设置上传进度回调函数
    fn on_upload_progress<F: Fn(&UploadingProgressInfo) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self;

    /// 设置响应成功的回调函数
    fn on_response_ok<F: Fn(&mut ResponseParts) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self;

    /// 设置响应错误的回调函数
    fn on_response_error<F: Fn(&ResponseError) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self;
}

/// 分片上传回调函数提供者
pub trait MultiPartsUploaderWithCallbacks: UploaderWithCallbacks {
    /// 设置分片上传回调函数
    fn on_part_uploaded<F: Fn(&dyn UploadedPart) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self;
}

#[derive(Default, Clone)]
pub(super) struct Callbacks<'a> {
    before_request_callbacks: Vec<BeforeRequestCallback<'a>>,
    upload_progress_callbacks: Vec<UploadProgressCallback<'a>>,
    part_uploaded_callbacks: Vec<PartUploadedCallback<'a>>,
    after_response_ok_callbacks: Vec<AfterResponseOkCallback<'a>>,
    after_response_error_callbacks: Vec<AfterResponseErrorCallback<'a>>,
}

impl<'a> Callbacks<'a> {
    pub(super) fn insert_before_request_callback(
        &mut self,
        callback: impl Fn(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callbacks.push(Arc::new(callback));
        self
    }

    pub(super) fn insert_upload_progress_callback(
        &mut self,
        callback: impl Fn(&UploadingProgressInfo) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.upload_progress_callbacks.push(Arc::new(callback));
        self
    }

    pub(super) fn insert_part_uploaded_callback(
        &mut self,
        callback: impl Fn(&dyn UploadedPart) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.part_uploaded_callbacks.push(Arc::new(callback));
        self
    }

    pub(super) fn insert_after_response_ok_callback(
        &mut self,
        callback: impl Fn(&mut ResponseParts) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.after_response_ok_callbacks.push(Arc::new(callback));
        self
    }

    pub(super) fn insert_after_response_error_callback(
        &mut self,
        callback: impl Fn(&ResponseError) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.after_response_error_callbacks.push(Arc::new(callback));
        self
    }

    pub(super) fn before_request(&self, builder_parts: &mut RequestBuilderParts) -> AnyResult<()> {
        for callback in self.before_request_callbacks.iter() {
            callback(builder_parts)?;
        }
        Ok(())
    }

    pub(super) fn upload_progress(&self, progress_info: &UploadingProgressInfo) -> AnyResult<()> {
        for callback in self.upload_progress_callbacks.iter() {
            callback(progress_info)?;
        }
        Ok(())
    }

    pub(super) fn part_uploaded(&self, progress_info: &dyn UploadedPart) -> AnyResult<()> {
        for callback in self.part_uploaded_callbacks.iter() {
            callback(progress_info)?;
        }
        Ok(())
    }

    pub(super) fn after_response<B>(&self, result: &mut Result<Response<B>, ResponseError>) -> AnyResult<()> {
        match result {
            Ok(response) => self.after_response_ok(response.parts_mut()),
            Err(err) => self.after_response_error(err),
        }
    }

    fn after_response_ok(&self, response_parts: &mut ResponseParts) -> AnyResult<()> {
        for callback in self.after_response_ok_callbacks.iter() {
            callback(response_parts)?;
        }
        Ok(())
    }

    fn after_response_error(&self, error: &ResponseError) -> AnyResult<()> {
        for callback in self.after_response_error_callbacks.iter() {
            callback(error)?;
        }
        Ok(())
    }
}

impl Debug for Callbacks<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Callbacks").finish()
    }
}

/// 上传进度信息
#[derive(Debug, Clone, Copy)]
pub struct UploadingProgressInfo {
    transferred_bytes: u64,
    total_bytes: Option<u64>,
}

impl UploadingProgressInfo {
    /// 创建上传进度信息
    #[inline]
    pub fn new(transferred_bytes: u64, total_bytes: Option<u64>) -> Self {
        Self {
            transferred_bytes,
            total_bytes,
        }
    }

    /// 获取已传输的字节数
    #[inline]
    pub fn transferred_bytes(&self) -> u64 {
        self.transferred_bytes
    }

    /// 获取总字节数
    #[inline]
    pub fn total_bytes(&self) -> Option<u64> {
        self.total_bytes
    }
}

impl<'a> From<&'a TransferProgressInfo<'a>> for UploadingProgressInfo {
    #[inline]
    fn from(t: &'a TransferProgressInfo<'a>) -> Self {
        Self::new(t.transferred_bytes(), Some(t.total_bytes()))
    }
}

impl From<TransferProgressInfo<'_>> for UploadingProgressInfo {
    #[inline]
    fn from(t: TransferProgressInfo<'_>) -> Self {
        Self::new(t.transferred_bytes(), Some(t.total_bytes()))
    }
}
