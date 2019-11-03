use crate::http::Response;
use qiniu_http::{Error as HTTPError, ErrorKind as HTTPErrorKind, Result as HTTPResult};

pub(super) fn upload_response_callback(response: &mut Response) -> HTTPResult<()> {
    if with_reqid(response) {
        Ok(())
    } else {
        Err(HTTPError::new_retryable_error_from_parts(
            HTTPErrorKind::MaliciousResponse,
            true,
            Some(response.method()),
            Some((response.base_url().to_owned() + response.path()).into()),
        ))
    }
}

fn with_reqid(response: &mut Response) -> bool {
    response.header("X-ReqId").is_some()
}
