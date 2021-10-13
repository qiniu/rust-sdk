use super::super::{HTTPClient, HTTPClientBuilder};
use qiniu_http::{
    HTTPCaller, HeaderMap, ResponseError, ResponseErrorKind, StatusCode,
    SyncRequest as SyncHttpRequest, SyncResponse, SyncResponseBody, SyncResponseResult,
};

#[cfg(feature = "async")]
use {
    futures::future::BoxFuture,
    qiniu_http::{AsyncRequest as AsyncHTTPRequest, AsyncResponse, AsyncResponseResult},
};

pub(crate) fn make_dumb_client_builder() -> HTTPClientBuilder {
    #[derive(Debug, Default)]
    struct FakeHTTPCaller;

    impl HTTPCaller for FakeHTTPCaller {
        #[inline]
        fn call(&self, _request: &mut SyncHttpRequest) -> SyncResponseResult {
            Ok(Default::default())
        }

        #[inline]
        #[cfg(feature = "async")]
        fn async_call<'a>(
            &'a self,
            _request: &'a AsyncHTTPRequest<'_>,
        ) -> BoxFuture<'a, AsyncResponseResult> {
            Box::pin(async { Ok(Default::default()) })
        }
    }

    HTTPClient::builder(Box::new(FakeHTTPCaller))
}

pub(crate) fn make_fixed_response_client_builder(
    status_code: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
    is_resolved_ip_addrs_supported: bool,
) -> HTTPClientBuilder {
    #[derive(Debug)]
    struct RedirectHTTPCaller {
        status_code: StatusCode,
        headers: HeaderMap,
        body: Vec<u8>,
        is_resolved_ip_addrs_supported: bool,
    }

    impl HTTPCaller for RedirectHTTPCaller {
        #[inline]
        fn call(&self, _request: &mut SyncHttpRequest) -> SyncResponseResult {
            Ok(SyncResponse::builder()
                .status_code(self.status_code)
                .headers(self.headers.to_owned())
                .body(SyncResponseBody::from_bytes(self.body.to_owned()))
                .build())
        }

        #[cfg(feature = "async")]
        fn async_call<'a>(
            &'a self,
            _request: &'a AsyncHTTPRequest<'_>,
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
        fn is_resolved_ip_addrs_supported(&self) -> bool {
            self.is_resolved_ip_addrs_supported
        }
    }

    let http_caller = Box::new(RedirectHTTPCaller {
        status_code,
        headers,
        body,
        is_resolved_ip_addrs_supported,
    });

    HTTPClient::builder(http_caller)
}

pub(crate) fn make_error_response_client_builder(
    error_kind: ResponseErrorKind,
    message: impl Into<String>,
    is_resolved_ip_addrs_supported: bool,
) -> HTTPClientBuilder {
    #[derive(Debug)]
    struct ErrorHTTPCaller {
        error_kind: ResponseErrorKind,
        message: String,
        is_resolved_ip_addrs_supported: bool,
    }

    impl HTTPCaller for ErrorHTTPCaller {
        #[inline]
        fn call(&self, _request: &mut SyncHttpRequest) -> SyncResponseResult {
            Err(ResponseError::builder(self.error_kind, self.message.to_owned()).build())
        }

        #[cfg(feature = "async")]
        fn async_call<'a>(
            &'a self,
            _request: &'a AsyncHTTPRequest<'_>,
        ) -> BoxFuture<'a, AsyncResponseResult> {
            Box::pin(async move {
                Err(ResponseError::builder(self.error_kind, self.message.to_owned()).build())
            })
        }

        #[inline]
        fn is_resolved_ip_addrs_supported(&self) -> bool {
            self.is_resolved_ip_addrs_supported
        }
    }

    let http_caller = Box::new(ErrorHTTPCaller {
        error_kind,
        is_resolved_ip_addrs_supported,
        message: message.into(),
    });

    HTTPClient::builder(http_caller)
}
