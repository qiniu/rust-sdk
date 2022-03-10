use super::UploadedPart;
use qiniu_apis::{
    http::{ResponseParts, TransferProgressInfo},
    http_client::{CallbackResult, RequestBuilderParts, Response, ResponseError},
};
use std::fmt::{self, Debug};

type BeforeRequestCallback<'c> = Box<dyn Fn(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'c>;
type UploadProgressCallback<'c> = Box<dyn Fn(&UploadingProgressInfo) -> CallbackResult + Send + Sync + 'c>;
type PartUploadedCallback<'c> = Box<dyn Fn(&dyn UploadedPart) -> CallbackResult + Send + Sync + 'c>;
type AfterResponseOkCallback<'c> = Box<dyn Fn(&mut ResponseParts) -> CallbackResult + Send + Sync + 'c>;
type AfterResponseErrorCallback<'c> = Box<dyn Fn(&ResponseError) -> CallbackResult + Send + Sync + 'c>;

pub trait UploaderWithCallbacks {
    fn on_before_request<F: Fn(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'static>(
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
}

pub trait MultiPartsUploaderWithCallbacks: UploaderWithCallbacks {
    fn on_part_uploaded<F: Fn(&dyn UploadedPart) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self;
}

#[derive(Default)]
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
        callback: impl Fn(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn insert_upload_progress_callback(
        &mut self,
        callback: impl Fn(&UploadingProgressInfo) -> CallbackResult + Send + Sync + 'a,
    ) -> &mut Self {
        self.upload_progress_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn insert_part_uploaded_callback(
        &mut self,
        callback: impl Fn(&dyn UploadedPart) -> CallbackResult + Send + Sync + 'a,
    ) -> &mut Self {
        self.part_uploaded_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn insert_after_response_ok_callback(
        &mut self,
        callback: impl Fn(&mut ResponseParts) -> CallbackResult + Send + Sync + 'a,
    ) -> &mut Self {
        self.after_response_ok_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn insert_after_response_error_callback(
        &mut self,
        callback: impl Fn(&ResponseError) -> CallbackResult + Send + Sync + 'a,
    ) -> &mut Self {
        self.after_response_error_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn before_request(&self, builder_parts: &mut RequestBuilderParts) -> CallbackResult {
        for callback in self.before_request_callbacks.iter() {
            if callback(builder_parts) == CallbackResult::Cancel {
                return CallbackResult::Cancel;
            }
        }
        CallbackResult::Continue
    }

    pub(super) fn upload_progress(&self, progress_info: &UploadingProgressInfo) -> CallbackResult {
        for callback in self.upload_progress_callbacks.iter() {
            if callback(progress_info) == CallbackResult::Cancel {
                return CallbackResult::Cancel;
            }
        }
        CallbackResult::Continue
    }

    pub(super) fn part_uploaded(&self, progress_info: &dyn UploadedPart) -> CallbackResult {
        for callback in self.part_uploaded_callbacks.iter() {
            if callback(progress_info) == CallbackResult::Cancel {
                return CallbackResult::Cancel;
            }
        }
        CallbackResult::Continue
    }

    pub(super) fn after_response<B>(&self, result: &mut Result<Response<B>, ResponseError>) -> CallbackResult {
        match result {
            Ok(response) => {
                for callback in self.after_response_ok_callbacks.iter() {
                    if callback(response.parts_mut()) == CallbackResult::Cancel {
                        return CallbackResult::Cancel;
                    }
                }
            }
            Err(err) => {
                for callback in self.after_response_error_callbacks.iter() {
                    if callback(err) == CallbackResult::Cancel {
                        return CallbackResult::Cancel;
                    }
                }
            }
        }
        CallbackResult::Continue
    }
}

impl<'a> Debug for Callbacks<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Callbacks").finish()
    }
}

pub struct UploadingProgressInfo<'b> {
    transferred_bytes: u64,
    total_bytes: Option<u64>,
    body: &'b [u8],
}

impl<'b> UploadingProgressInfo<'b> {
    #[inline]
    pub fn new(transferred_bytes: u64, total_bytes: Option<u64>, body: &'b [u8]) -> Self {
        Self {
            transferred_bytes,
            total_bytes,
            body,
        }
    }

    #[inline]
    pub fn transferred_bytes(&self) -> u64 {
        self.transferred_bytes
    }

    #[inline]
    pub fn total_bytes(&self) -> Option<u64> {
        self.total_bytes
    }

    #[inline]
    pub fn body(&self) -> &[u8] {
        self.body
    }
}

impl<'b> From<&'b TransferProgressInfo<'b>> for UploadingProgressInfo<'b> {
    #[inline]
    fn from(t: &'b TransferProgressInfo<'b>) -> Self {
        Self::new(t.transferred_bytes(), Some(t.total_bytes()), t.body())
    }
}
