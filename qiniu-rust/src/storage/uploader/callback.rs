use crate::http::{Error as HTTPError, ErrorKind as HTTPErrorKind, Response, Result as HTTPResult};

pub(super) fn upload_response_callback(response: &mut Response) -> HTTPResult<()> {
    if with_reqid(response) {
        Ok(())
    } else {
        Err(HTTPError::new_retryable_error(
            HTTPErrorKind::MaliciousResponse,
            true,
            Some(response.method()),
            Some((response.base_url().to_owned() + response.path()).into()),
            response.request_id().map(|request_id| request_id.into()),
        ))
    }
}

fn with_reqid(response: &mut Response) -> bool {
    response.header("X-ReqId").is_some()
}
