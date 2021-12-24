use qiniu_apis::{
    http::ResponseParts,
    http_client::{CallbackResult, RequestBuilderParts, Response, ResponseError},
};

type BeforeRequestCallback<'c> =
    Box<dyn FnMut(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'c>;
type AfterResponseOkCallback<'c> =
    Box<dyn FnMut(&mut ResponseParts) -> CallbackResult + Send + Sync + 'c>;
type AfterResponseErrorCallback<'c> =
    Box<dyn FnMut(&ResponseError) -> CallbackResult + Send + Sync + 'c>;

#[derive(Default)]
pub(super) struct Callbacks<'a> {
    pub(super) before_request_callback: Option<BeforeRequestCallback<'a>>,
    pub(super) after_response_ok_callback: Option<AfterResponseOkCallback<'a>>,
    pub(super) after_response_error_callback: Option<AfterResponseErrorCallback<'a>>,
}

impl Callbacks<'_> {
    pub(super) fn before_request(
        &mut self,
        builder_parts: &mut RequestBuilderParts,
    ) -> CallbackResult {
        if let Some(before_request_callback) = self.before_request_callback.as_mut() {
            before_request_callback(builder_parts)
        } else {
            CallbackResult::Continue
        }
    }

    pub(super) fn after_response<B>(
        &mut self,
        result: &mut Result<Response<B>, ResponseError>,
    ) -> CallbackResult {
        match (
            result,
            self.after_response_ok_callback.as_mut(),
            self.after_response_error_callback.as_mut(),
        ) {
            (Ok(response), Some(after_response_ok_callback), _) => {
                after_response_ok_callback(response.parts_mut())
            }
            (Err(err), _, Some(after_response_error_callback)) => {
                after_response_error_callback(err)
            }
            _ => CallbackResult::Continue,
        }
    }
}
