use super::DownloadingProgressInfo;
use anyhow::Result as AnyResult;
use qiniu_apis::{
    http::ResponseParts as HttpResponseParts,
    http_client::{RequestBuilderParts, Response, ResponseError},
};
use std::fmt::{self, Debug};

type BeforeRequestCallback<'c> = Box<dyn Fn(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'c>;
type DownloadProgressCallback<'c> = Box<dyn Fn(DownloadingProgressInfo) -> AnyResult<()> + Send + Sync + 'c>;
type AfterResponseOkCallback<'c> = Box<dyn Fn(&mut HttpResponseParts) -> AnyResult<()> + Send + Sync + 'c>;
type AfterResponseErrorCallback<'c> = Box<dyn Fn(&ResponseError) -> AnyResult<()> + Send + Sync + 'c>;

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
        callback: impl Fn(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn insert_download_progress_callback(
        &mut self,
        callback: impl Fn(DownloadingProgressInfo) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.download_progress_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn insert_after_response_ok_callback(
        &mut self,
        callback: impl Fn(&mut HttpResponseParts) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.after_response_ok_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn insert_after_response_error_callback(
        &mut self,
        callback: impl Fn(&ResponseError) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.after_response_error_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn before_request(&self, builder_parts: &mut RequestBuilderParts) -> AnyResult<()> {
        self.before_request_callbacks
            .iter()
            .try_for_each(|callback| callback(builder_parts))
    }

    pub(super) fn download_progress(&self, progress_info: DownloadingProgressInfo) -> AnyResult<()> {
        self.download_progress_callbacks
            .iter()
            .try_for_each(|callback| callback(progress_info))
    }

    pub(super) fn after_response<B>(&self, result: &mut Result<Response<B>, ResponseError>) -> AnyResult<()> {
        match result {
            Ok(response) => self.after_response_ok(response.parts_mut()),
            Err(err) => self.after_response_error(err),
        }
    }

    fn after_response_ok(&self, response_parts: &mut HttpResponseParts) -> AnyResult<()> {
        self.after_response_ok_callbacks
            .iter()
            .try_for_each(|callback| callback(response_parts))
    }

    fn after_response_error(&self, error: &ResponseError) -> AnyResult<()> {
        self.after_response_error_callbacks
            .iter()
            .try_for_each(|callback| callback(error))
    }
}

impl Debug for Callbacks<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Callbacks").finish()
    }
}
