use super::super::{HttpClient, HttpClientBuilder};
use qiniu_http::{
    HeaderMap, HttpCaller, ResponseError, ResponseErrorKind, StatusCode, SyncRequest as SyncHttpRequest, SyncResponse,
    SyncResponseBody, SyncResponseResult,
};

#[cfg(feature = "async")]
use {
    futures::future::BoxFuture,
    qiniu_http::{AsyncRequest as AsyncHttpRequest, AsyncResponse, AsyncResponseBody, AsyncResponseResult},
};

pub(crate) fn make_dumb_client_builder() -> HttpClientBuilder {
    #[derive(Debug, Default)]
    struct FakeHttpCaller;

    impl HttpCaller for FakeHttpCaller {
        fn call(&self, _request: &mut SyncHttpRequest<'_>) -> SyncResponseResult {
            Ok(Default::default())
        }

        #[cfg(feature = "async")]
        fn async_call<'a>(&'a self, _request: &'a mut AsyncHttpRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
            Box::pin(async { Ok(Default::default()) })
        }
    }

    HttpClient::builder(FakeHttpCaller)
}

pub(crate) fn make_fixed_response_client_builder(
    status_code: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
    is_resolved_ip_addrs_supported: bool,
) -> HttpClientBuilder {
    #[derive(Debug)]
    struct RedirectHttpCaller {
        status_code: StatusCode,
        headers: HeaderMap,
        body: Vec<u8>,
        is_resolved_ip_addrs_supported: bool,
    }

    impl HttpCaller for RedirectHttpCaller {
        fn call(&self, _request: &mut SyncHttpRequest<'_>) -> SyncResponseResult {
            Ok(SyncResponse::builder()
                .status_code(self.status_code)
                .headers(self.headers.to_owned())
                .body(SyncResponseBody::from_bytes(self.body.to_owned()))
                .build())
        }

        #[cfg(feature = "async")]
        fn async_call<'a>(&'a self, _request: &'a mut AsyncHttpRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
            Box::pin(async move {
                Ok(AsyncResponse::builder()
                    .status_code(self.status_code)
                    .headers(self.headers.to_owned())
                    .body(AsyncResponseBody::from_bytes(self.body.to_owned()))
                    .build())
            })
        }

        fn is_resolved_ip_addrs_supported(&self) -> bool {
            self.is_resolved_ip_addrs_supported
        }
    }

    HttpClient::builder(RedirectHttpCaller {
        status_code,
        headers,
        body,
        is_resolved_ip_addrs_supported,
    })
}

pub(crate) fn make_error_response_client_builder(
    error_kind: ResponseErrorKind,
    message: impl Into<String>,
    is_resolved_ip_addrs_supported: bool,
) -> HttpClientBuilder {
    #[derive(Debug)]
    struct ErrorHttpCaller {
        error_kind: ResponseErrorKind,
        message: String,
        is_resolved_ip_addrs_supported: bool,
    }

    impl HttpCaller for ErrorHttpCaller {
        fn call(&self, _request: &mut SyncHttpRequest<'_>) -> SyncResponseResult {
            Err(ResponseError::builder_with_msg(self.error_kind, self.message.to_owned()).build())
        }

        #[cfg(feature = "async")]
        fn async_call<'a>(&'a self, _request: &'a mut AsyncHttpRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
            Box::pin(
                async move { Err(ResponseError::builder_with_msg(self.error_kind, self.message.to_owned()).build()) },
            )
        }

        fn is_resolved_ip_addrs_supported(&self) -> bool {
            self.is_resolved_ip_addrs_supported
        }
    }

    HttpClient::builder(ErrorHttpCaller {
        error_kind,
        is_resolved_ip_addrs_supported,
        message: message.into(),
    })
}
