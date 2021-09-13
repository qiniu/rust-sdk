use super::super::{HTTPClient, HTTPClientBuilder};
use qiniu_http::{
    HTTPCaller, HeaderMap, Request as HTTPRequest, ResponseError, ResponseErrorKind, StatusCode,
    SyncResponse, SyncResponseResult,
};
use std::any::Any;

#[cfg(feature = "async")]
use {
    futures::future::BoxFuture,
    qiniu_http::{AsyncResponse, AsyncResponseResult},
};

pub(crate) fn make_dumb_client_builder() -> HTTPClientBuilder {
    #[derive(Debug, Default)]
    struct FakeHTTPCaller;

    impl HTTPCaller for FakeHTTPCaller {
        #[inline]
        fn call(&self, _request: &HTTPRequest) -> SyncResponseResult {
            Ok(Default::default())
        }

        #[inline]
        #[cfg(feature = "async")]
        fn async_call<'a>(
            &'a self,
            _request: &'a HTTPRequest<'_>,
        ) -> BoxFuture<'a, AsyncResponseResult> {
            Box::pin(async { Ok(Default::default()) })
        }

        #[inline]
        fn as_http_caller(&self) -> &dyn HTTPCaller {
            self
        }

        #[inline]
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    HTTPClient::builder(Box::new(FakeHTTPCaller))
}

pub(crate) fn make_fixed_response_client_builder(
    status_code: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
) -> HTTPClientBuilder {
    #[derive(Debug)]
    struct RedirectHTTPCaller {
        status_code: StatusCode,
        headers: HeaderMap,
        body: Vec<u8>,
    }

    impl HTTPCaller for RedirectHTTPCaller {
        #[inline]
        fn call(&self, _request: &HTTPRequest) -> SyncResponseResult {
            Ok(SyncResponse::builder()
                .status_code(self.status_code)
                .headers(self.headers.to_owned())
                .bytes_as_body(self.body.to_owned())
                .build())
        }

        #[cfg(feature = "async")]
        fn async_call<'a>(
            &'a self,
            _request: &'a HTTPRequest<'_>,
        ) -> BoxFuture<'a, AsyncResponseResult> {
            Box::pin(async move {
                Ok(AsyncResponse::builder()
                    .status_code(self.status_code)
                    .headers(self.headers.to_owned())
                    .bytes_as_body(self.body.to_owned())
                    .build())
            })
        }

        #[inline]
        fn as_http_caller(&self) -> &dyn HTTPCaller {
            self
        }

        #[inline]
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    let http_caller = Box::new(RedirectHTTPCaller {
        status_code,
        headers,
        body,
    });

    HTTPClient::builder(http_caller)
}

pub(crate) fn make_error_response_client_builder(
    error_kind: ResponseErrorKind,
    message: impl Into<String>,
) -> HTTPClientBuilder {
    #[derive(Debug)]
    struct ErrorHTTPCaller {
        error_kind: ResponseErrorKind,
        message: String,
    }

    impl HTTPCaller for ErrorHTTPCaller {
        #[inline]
        fn call(&self, _request: &HTTPRequest) -> SyncResponseResult {
            Err(ResponseError::builder(self.error_kind, self.message.to_owned()).build())
        }

        #[cfg(feature = "async")]
        fn async_call<'a>(
            &'a self,
            _request: &'a HTTPRequest<'_>,
        ) -> BoxFuture<'a, AsyncResponseResult> {
            Box::pin(async move {
                Err(ResponseError::builder(self.error_kind, self.message.to_owned()).build())
            })
        }

        #[inline]
        fn as_http_caller(&self) -> &dyn HTTPCaller {
            self
        }

        #[inline]
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    let http_caller = Box::new(ErrorHTTPCaller {
        error_kind,
        message: message.into(),
    });

    HTTPClient::builder(http_caller)
}
