use qiniu_apis::{
    http::{ResponseParts as HttpResponseParts, TransferProgressInfo},
    http_client::{CallbackResult, RequestBuilderParts, Response, ResponseError},
};
use std::fmt::{self, Debug};

type BeforeRequestCallback<'c> = Box<dyn Fn(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'c>;
type DownloadProgressCallback<'c> = Box<dyn Fn(&TransferProgressInfo) -> CallbackResult + Send + Sync + 'c>;
type AfterResponseOkCallback<'c> = Box<dyn Fn(&mut HttpResponseParts) -> CallbackResult + Send + Sync + 'c>;
type AfterResponseErrorCallback<'c> = Box<dyn Fn(&ResponseError) -> CallbackResult + Send + Sync + 'c>;

#[derive(Default)]
pub(super) struct Callbacks<'a> {
    before_request_callbacks: Vec<BeforeRequestCallback<'a>>,
    download_progress_callbacks: Vec<DownloadProgressCallback<'a>>,
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

    pub(super) fn insert_download_progress_callback(
        &mut self,
        callback: impl Fn(&TransferProgressInfo) -> CallbackResult + Send + Sync + 'a,
    ) -> &mut Self {
        self.download_progress_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn insert_after_response_ok_callback(
        &mut self,
        callback: impl Fn(&mut HttpResponseParts) -> CallbackResult + Send + Sync + 'a,
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

    pub(super) fn download_progress(&self, progress_info: &TransferProgressInfo) -> CallbackResult {
        for callback in self.download_progress_callbacks.iter() {
            if callback(progress_info) == CallbackResult::Cancel {
                return CallbackResult::Cancel;
            }
        }
        CallbackResult::Continue
    }

    pub(super) fn after_response<B>(&self, result: &mut Result<Response<B>, ResponseError>) -> CallbackResult {
        match result {
            Ok(response) => self.after_response_ok(response.parts_mut()),
            Err(err) => self.after_response_error(err),
        }
    }

    fn after_response_ok(&self, response_parts: &mut HttpResponseParts) -> CallbackResult {
        for callback in self.after_response_ok_callbacks.iter() {
            if callback(response_parts) == CallbackResult::Cancel {
                return CallbackResult::Cancel;
            }
        }
        CallbackResult::Continue
    }

    fn after_response_error(&self, error: &ResponseError) -> CallbackResult {
        for callback in self.after_response_error_callbacks.iter() {
            if callback(error) == CallbackResult::Cancel {
                return CallbackResult::Cancel;
            }
        }
        CallbackResult::Continue
    }
}

impl Debug for Callbacks<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Callbacks").finish()
    }
}
