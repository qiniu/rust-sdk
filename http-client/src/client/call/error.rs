use super::super::{ResponseError, ResponseErrorKind, RetryDecision, RetryResult};
use qiniu_http::{Extensions, RequestParts as HttpRequestParts, ResponseErrorKind as HttpResponseErrorKind};
use serde::Deserialize;
use std::mem::take;

#[derive(Debug)]
pub(super) struct TryError {
    response_error: ResponseError,
    retry_result: RetryResult,
}

impl TryError {
    pub(super) fn new(response_error: ResponseError, retry_result: RetryResult) -> Self {
        Self {
            response_error,
            retry_result,
        }
    }

    pub(super) fn response_error(&self) -> &ResponseError {
        &self.response_error
    }

    pub(super) fn feedback_response_error(&self) -> Option<&ResponseError> {
        match &self.response_error.kind() {
            ResponseErrorKind::HttpError(error_kind) => match error_kind {
                HttpResponseErrorKind::ConnectError
                | HttpResponseErrorKind::ProxyError
                | HttpResponseErrorKind::DnsServerError
                | HttpResponseErrorKind::UnknownHostError
                | HttpResponseErrorKind::SendError
                | HttpResponseErrorKind::ReceiveError
                | HttpResponseErrorKind::CallbackError => Some(self.response_error()),
                _ => None,
            },
            ResponseErrorKind::StatusCodeError(status_code) => match status_code.as_u16() {
                500..=599 => Some(self.response_error()),
                _ => None,
            },
            ResponseErrorKind::ParseResponseError
            | ResponseErrorKind::UnexpectedEof
            | ResponseErrorKind::MaliciousResponse => Some(self.response_error()),
            ResponseErrorKind::UnexpectedStatusCode(_)
            | ResponseErrorKind::SystemCallError
            | ResponseErrorKind::NoTry => None,
        }
    }

    pub(super) fn into_response_error(self) -> ResponseError {
        self.response_error
    }

    pub(super) fn retry_decision(&self) -> RetryDecision {
        self.retry_result.decision()
    }

    pub(super) fn with_extensions(self, extensions: Extensions) -> TryErrorWithExtensions {
        TryErrorWithExtensions {
            inner: self,
            extensions,
        }
    }

    pub(super) fn with_request(self, request: &mut HttpRequestParts) -> TryErrorWithExtensions {
        self.with_extensions(take(request.extensions_mut()))
    }
}

#[derive(Debug)]
pub(super) struct TryErrorWithExtensions {
    inner: TryError,
    extensions: Extensions,
}

impl TryErrorWithExtensions {
    pub(super) fn into_response_error(self) -> ResponseError {
        self.inner.into_response_error()
    }

    pub(super) fn retry_decision(&self) -> RetryDecision {
        self.inner.retry_decision()
    }

    pub(super) fn split(self) -> (TryError, Extensions) {
        (self.inner, self.extensions)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ErrorResponseBody {
    error: String,
}

impl ErrorResponseBody {
    pub(super) fn into_error(self) -> String {
        self.error
    }
}
