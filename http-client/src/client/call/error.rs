use super::super::{ResponseError, RetryResult};
use serde::Deserialize;

#[derive(Debug)]
pub(super) struct TryError {
    response_error: ResponseError,
    retry_result: RetryResult,
}

impl TryError {
    #[inline]
    pub(super) fn new(response_error: ResponseError, retry_result: RetryResult) -> Self {
        Self {
            response_error,
            retry_result,
        }
    }

    #[inline]
    pub(super) fn response_error(&self) -> &ResponseError {
        &self.response_error
    }

    #[inline]
    pub(super) fn into_response_error(self) -> ResponseError {
        self.response_error
    }

    #[inline]
    pub(super) fn retry_result(&self) -> RetryResult {
        self.retry_result
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ErrorResponseBody {
    error: String,
}

impl ErrorResponseBody {
    #[inline]
    pub(super) fn into_error(self) -> String {
        self.error
    }
}
