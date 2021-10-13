use super::super::{ResponseError, ResponseErrorKind, RetryDecision, RetryResult};
use qiniu_http::{
    Extensions, RequestParts as HTTPRequestParts, ResponseErrorKind as HTTPResponseErrorKind,
};
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
    pub(super) fn response_error(&self) -> &ResponseError {
        &self.response_error
    }

    #[inline]
    pub(super) fn feedback_response_error(&self) -> Option<&ResponseError> {
        match &self.response_error.kind() {
            ResponseErrorKind::HTTPError(error_kind) => match error_kind {
                HTTPResponseErrorKind::ConnectError
                | HTTPResponseErrorKind::ProxyError
                | HTTPResponseErrorKind::DNSServerError
                | HTTPResponseErrorKind::UnknownHostError
                | HTTPResponseErrorKind::SendError
                | HTTPResponseErrorKind::ReceiveError
                | HTTPResponseErrorKind::UserCanceled => Some(self.response_error()),
                _ => None,
            },
            ResponseErrorKind::StatusCodeError(status_code) => match status_code.as_u16() {
                500..=599 => Some(self.response_error()),
                _ => None,
            },
            ResponseErrorKind::UnexpectedEof
            | ResponseErrorKind::ParseResponseError
            | ResponseErrorKind::MaliciousResponse => Some(self.response_error()),
            ResponseErrorKind::UnexpectedStatusCode(_) | ResponseErrorKind::NoTry => None,
        }
    }

    #[inline]
    pub(super) fn into_response_error(self) -> ResponseError {
        self.response_error
    }

    #[inline]
    pub(super) fn retry_decision(&self) -> RetryDecision {
        self.retry_result.decision()
    }

    #[inline]
    pub(super) fn with_extensions(self, extensions: Extensions) -> TryErrorWithExtensions {
        TryErrorWithExtensions {
            inner: self,
            extensions,
        }
    }

    #[inline]
    pub(super) fn with_request(self, request: &mut HTTPRequestParts) -> TryErrorWithExtensions {
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
    pub(super) fn into_response_error(self) -> ResponseError {
        self.inner.into_response_error()
    }

    #[inline]
    pub(super) fn retry_decision(&self) -> RetryDecision {
        self.inner.retry_decision()
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
