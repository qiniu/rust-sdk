use anyhow::Result as AnyResult;
use qiniu_apis::{
    http::ResponseParts,
    http_client::{RequestBuilderParts, Response, ResponseError},
};
use std::fmt::{self, Debug};

type BeforeRequestCallback<'c> = Box<dyn FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'c>;
type AfterResponseOkCallback<'c> = Box<dyn FnMut(&mut ResponseParts) -> AnyResult<()> + Send + Sync + 'c>;
type AfterResponseErrorCallback<'c> = Box<dyn FnMut(&ResponseError) -> AnyResult<()> + Send + Sync + 'c>;

#[derive(Default)]
pub(super) struct Callbacks<'a> {
    before_request_callbacks: Vec<BeforeRequestCallback<'a>>,
    after_response_ok_callbacks: Vec<AfterResponseOkCallback<'a>>,
    after_response_error_callbacks: Vec<AfterResponseErrorCallback<'a>>,
}

impl<'a> Callbacks<'a> {
    pub(super) fn insert_before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn insert_after_response_ok_callback(
        &mut self,
        callback: impl FnMut(&mut ResponseParts) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.after_response_ok_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn insert_after_response_error_callback(
        &mut self,
        callback: impl FnMut(&ResponseError) -> AnyResult<()> + Send + Sync + 'a,
    ) -> &mut Self {
        self.after_response_error_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn before_request(&mut self, builder_parts: &mut RequestBuilderParts) -> AnyResult<()> {
        self.before_request_callbacks
            .iter_mut()
            .try_for_each(|callback| callback(builder_parts))
    }

    pub(super) fn after_response<B>(&mut self, result: &mut Result<Response<B>, ResponseError>) -> AnyResult<()> {
        match result {
            Ok(response) => self
                .after_response_ok_callbacks
                .iter_mut()
                .try_for_each(|callback| callback(response)),
            Err(err) => self
                .after_response_error_callbacks
                .iter_mut()
                .try_for_each(|callback| callback(err)),
        }
    }
}

impl Debug for Callbacks<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Callbacks").finish()
    }
}
