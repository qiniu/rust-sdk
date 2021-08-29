use super::super::{ResponseError, RetryResult};
use qiniu_http::{Extensions, Request as HTTPRequest};
use serde::Deserialize;
use std::mem::take;

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
    #[allow(dead_code)]
    pub(super) fn response_error(&self) -> &ResponseError {
        &self.response_error
    }

    #[inline]
    #[allow(dead_code)]
    pub(super) fn into_response_error(self) -> ResponseError {
        self.response_error
    }

    #[inline]
    #[allow(dead_code)]
    pub(super) fn retry_result(&self) -> RetryResult {
        self.retry_result
    }

    #[inline]
    pub(super) fn with_extensions(self, extensions: Extensions) -> TryErrorWithExtensions {
        TryErrorWithExtensions {
            inner: self,
            extensions,
        }
    }

    #[inline]
    pub(super) fn with_request(self, request: &mut HTTPRequest) -> TryErrorWithExtensions {
        self.with_extensions(take(request.extensions_mut()))
    }
}

#[derive(Debug)]
pub(super) struct TryErrorWithExtensions {
    inner: TryError,
    extensions: Extensions,
}

impl TryErrorWithExtensions {
    #[inline]
    #[allow(dead_code)]
    pub(super) fn response_error(&self) -> &ResponseError {
        &self.inner.response_error
    }

    #[inline]
    #[allow(dead_code)]
    pub(super) fn into_response_error(self) -> ResponseError {
        self.inner.response_error
    }

    #[inline]
    #[allow(dead_code)]
    pub(super) fn retry_result(&self) -> RetryResult {
        self.inner.retry_result
    }

    #[inline]
    pub(super) fn split(self) -> (TryError, Extensions) {
        (self.inner, self.extensions)
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
