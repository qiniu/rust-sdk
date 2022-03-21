use qiniu_apis::{
    http::ResponseParts,
    http_client::{CallbackResult, RequestBuilderParts, Response, ResponseError},
};
use std::fmt::{self, Debug};

type BeforeRequestCallback<'c> = Box<dyn FnMut(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'c>;
type AfterResponseOkCallback<'c> = Box<dyn FnMut(&mut ResponseParts) -> CallbackResult + Send + Sync + 'c>;
type AfterResponseErrorCallback<'c> = Box<dyn FnMut(&ResponseError) -> CallbackResult + Send + Sync + 'c>;

#[derive(Default)]
pub(super) struct Callbacks<'a> {
    before_request_callbacks: Vec<BeforeRequestCallback<'a>>,
    after_response_ok_callbacks: Vec<AfterResponseOkCallback<'a>>,
    after_response_error_callbacks: Vec<AfterResponseErrorCallback<'a>>,
}

impl<'a> Callbacks<'a> {
    pub(super) fn insert_before_request_callback(
        &mut self,
        callback: impl FnMut(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'a,
    ) -> &mut Self {
        self.before_request_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn insert_after_response_ok_callback(
        &mut self,
        callback: impl FnMut(&mut ResponseParts) -> CallbackResult + Send + Sync + 'a,
    ) -> &mut Self {
        self.after_response_ok_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn insert_after_response_error_callback(
        &mut self,
        callback: impl FnMut(&ResponseError) -> CallbackResult + Send + Sync + 'a,
    ) -> &mut Self {
        self.after_response_error_callbacks.push(Box::new(callback));
        self
    }

    pub(super) fn before_request(&mut self, builder_parts: &mut RequestBuilderParts) -> CallbackResult {
        for callback in self.before_request_callbacks.iter_mut() {
            if callback(builder_parts) == CallbackResult::Cancel {
                return CallbackResult::Cancel;
            }
        }
        CallbackResult::Continue
    }

    pub(super) fn after_response<B>(&mut self, result: &mut Result<Response<B>, ResponseError>) -> CallbackResult {
        match result {
            Ok(response) => {
                for callback in self.after_response_ok_callbacks.iter_mut() {
                    if callback(response.parts_mut()) == CallbackResult::Cancel {
                        return CallbackResult::Cancel;
                    }
                }
            }
            Err(err) => {
                for callback in self.after_response_error_callbacks.iter_mut() {
                    if callback(err) == CallbackResult::Cancel {
                        return CallbackResult::Cancel;
                    }
                }
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
