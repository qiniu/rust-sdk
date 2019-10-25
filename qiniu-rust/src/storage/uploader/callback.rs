use crate::http::Response;
use qiniu_http::{Error as HTTPError, ErrorKind as HTTPErrorKind, Result as HTTPResult};
use std::borrow::Cow;

pub(super) fn upload_response_callback(response: &mut Response) -> HTTPResult<()> {
    if response.status_code() == 200 && is_response_body_contains_error(response) || is_not_qiniu(response) {
        Err(HTTPError::new_retryable_error_from_parts(
            HTTPErrorKind::MaliciousResponse,
            true,
            Some(response.method()),
            Some((response.base_url().to_owned() + response.path()).into()),
        ))
    } else {
        Ok(())
    }
}

fn is_response_body_contains_error(response: &mut Response) -> bool {
    let result: HTTPResult<serde_json::Value> = response.parse_json_clone();
    match result {
        Err(_) => false,
        Ok(value) => value.get("error").is_some(),
    }
}

fn is_not_qiniu(response: &mut Response) -> bool {
    response.header("X-ReqId").is_none()
        && (response.header("Content-Type") != Some(&Cow::Borrowed("application/json"))
            || (response.parse_json_clone() as HTTPResult<serde_json::Value>).is_err())
}
