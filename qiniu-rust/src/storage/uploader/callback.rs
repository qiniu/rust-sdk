use super::super::UploadPolicy;
use crate::http::{request::ResponseCallback, response::Response};
use qiniu_http::{Error as HTTPError, Request as HTTPRequest, Result as HTTPResult};
use std::borrow::Cow;

pub(super) struct UploadResponseCallback<'u>(pub(super) &'u UploadPolicy<'u>);

impl<'a> ResponseCallback for UploadResponseCallback<'_> {
    fn on_response_callback(&self, response: &mut Response, _request: &HTTPRequest) -> HTTPResult<()> {
        if self.is_server_error(response)
            || response.status_code() == 200 && self.is_response_body_contains_error(response)
            || self.is_not_qiniu(response) && !self.uptoken_has_url()
        {
            Err(HTTPError::new_retryable_error_from_parts(
                error::Error::from(error::ErrorKind::RetryError),
                true,
                Some(response.method()),
                Some((response.host().to_owned() + response.path()).into()),
            ))
        } else {
            Ok(())
        }
    }
}

impl UploadResponseCallback<'_> {
    fn is_server_error(&self, response: &Response) -> bool {
        match response.status_code() {
            406 | 996 | 500..=599 => true,
            _ => false,
        }
    }

    fn is_response_body_contains_error(&self, response: &mut Response) -> bool {
        let result: HTTPResult<serde_json::Value> = response.parse_json_clone();
        match result {
            Err(_) => false,
            Ok(value) => value.get("error").is_some(),
        }
    }

    fn is_not_qiniu(&self, response: &Response) -> bool {
        (200..500).contains(&response.status_code())
            && response.header("X-ReqId").is_none()
            && response.header("Content-Type") != Some(&Cow::Borrowed("application/json"))
    }

    fn uptoken_has_url(&self) -> bool {
        self.0.return_url().is_some()
    }
}

pub mod error {
    use error_chain::error_chain;
    error_chain! {
        errors {
            RetryError {
                description("HTTP call should be retry")
                display("HTTP call should be retry")
            }
        }
    }
}
