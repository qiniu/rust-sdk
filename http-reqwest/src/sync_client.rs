use super::extensions::TimeoutExtension;
use qiniu_http::{
    HTTPCaller, HeaderMap, HeaderValue, Request, ResponseError, ResponseErrorKind, StatusCode,
    SyncResponse, SyncResponseResult, TransferProgressInfo, Uri,
};
use reqwest::{
    blocking::{
        Body as SyncBody, Client as SyncReqwestClient, Request as SyncReqwestRequest,
        Response as SyncReqwestResponse,
    },
    header::USER_AGENT,
    Error as ReqwestError, Url,
};
use std::{
    any::Any,
    io::{Cursor, Error as IOError, ErrorKind as IOErrorKind, Read, Result as IOResult},
    mem::take,
    mem::transmute,
    num::NonZeroU16,
};

#[cfg(feature = "async")]
use {super::BoxFuture, qiniu_http::AsyncResponseResult};

#[derive(Debug, Default)]
pub struct SyncReqwestHTTPCaller {
    sync_client: SyncReqwestClient,
}

impl SyncReqwestHTTPCaller {
    #[inline]
    pub fn new(sync_client: SyncReqwestClient) -> Self {
        Self { sync_client }
    }
}

impl HTTPCaller for SyncReqwestHTTPCaller {
    #[inline]
    fn call(&self, request: &Request) -> SyncResponseResult {
        let mut user_cancelled_error: Option<ResponseError> = None;
        let reqwest_request = make_sync_reqwest_request(request, &mut user_cancelled_error)?;
        match self.sync_client.execute(reqwest_request) {
            Ok(reqwest_response) => from_sync_response(reqwest_response, request),
            Err(err) => {
                user_cancelled_error.map_or_else(|| Err(from_reqwest_error(err, request)), Err)
            }
        }
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_call<'a>(&'a self, _request: &'a Request) -> BoxFuture<'a, AsyncResponseResult> {
        unimplemented!("SyncReqwestHTTPCaller does not support async call")
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

fn make_sync_reqwest_request(
    request: &Request,
    user_cancelled_error: &mut Option<ResponseError>,
) -> Result<SyncReqwestRequest, ResponseError> {
    let url = Url::parse(&request.url().to_string()).map_err(|err| {
        ResponseError::builder(ResponseErrorKind::InvalidURL, err)
            .uri(request.url())
            .build()
    })?;
    let mut reqwest_request = SyncReqwestRequest::new(request.method().to_owned(), url);
    for (header_name, header_value) in request.headers() {
        reqwest_request
            .headers_mut()
            .insert(header_name, header_value.to_owned());
    }
    reqwest_request
        .headers_mut()
        .insert(USER_AGENT, make_user_agent(request, "sync")?);
    *reqwest_request.body_mut() = Some(SyncBody::sized(
        RequestBodyWithCallbacks::new(
            request.url(),
            request.body(),
            request.on_uploading_progress(),
            user_cancelled_error,
        ),
        request.body().len() as u64,
    ));

    if let Some(timeout) = request.extensions().get::<TimeoutExtension>() {
        *reqwest_request.timeout_mut() = Some(timeout.get());
    }

    return Ok(reqwest_request);

    type OnProgress<'r> = &'r (dyn Fn(&TransferProgressInfo) -> bool + Send + Sync);

    struct RequestBodyWithCallbacks {
        request_uri: &'static Uri,
        body: Cursor<&'static [u8]>,
        size: u64,
        on_uploading_progress: Option<OnProgress<'static>>,
        user_cancelled_error: &'static mut Option<ResponseError>,
    }

    impl RequestBodyWithCallbacks {
        fn new(
            request_uri: &Uri,
            body: &[u8],
            on_uploading_progress: Option<OnProgress>,
            user_cancelled_error: &mut Option<ResponseError>,
        ) -> Self {
            Self {
                size: body.len() as u64,
                body: Cursor::new(unsafe { transmute(body) }),
                on_uploading_progress: on_uploading_progress
                    .map(|callback| unsafe { transmute(callback) }),
                user_cancelled_error: unsafe { transmute(user_cancelled_error) },
                request_uri: unsafe { transmute(request_uri) },
            }
        }
    }

    impl Read for RequestBodyWithCallbacks {
        fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
            match self.body.read(buf) {
                Err(err) => Err(err),
                Ok(0) => Ok(0),
                Ok(n) => {
                    let buf = &buf[..n];
                    if let Some(on_uploading_progress) = self.on_uploading_progress {
                        if !on_uploading_progress(&TransferProgressInfo::new(
                            self.body.position(),
                            self.size,
                            buf,
                        )) {
                            const ERROR_MESSAGE: &str = "on_uploading_progress() returns false";
                            *self.user_cancelled_error = Some(
                                ResponseError::builder(
                                    ResponseErrorKind::UserCanceled,
                                    ERROR_MESSAGE,
                                )
                                .uri(self.request_uri)
                                .build(),
                            );
                            return Err(IOError::new(IOErrorKind::Other, ERROR_MESSAGE));
                        }
                    }
                    Ok(n)
                }
            }
        }
    }
}

#[inline]
pub(super) fn make_user_agent(
    request: &Request,
    suffix: &str,
) -> Result<HeaderValue, ResponseError> {
    HeaderValue::from_str(&format!(
        "{}/qiniu-http-reqwest-{}/{}",
        request.user_agent(),
        env!("CARGO_PKG_VERSION"),
        suffix
    ))
    .map_err(|err| {
        ResponseError::builder(ResponseErrorKind::InvalidHeader, err)
            .uri(request.url())
            .build()
    })
}

fn from_sync_response(mut response: SyncReqwestResponse, request: &Request) -> SyncResponseResult {
    call_response_callbacks(request, response.status(), response.headers())?;
    let mut response_builder = SyncResponse::builder()
        .status_code(response.status())
        .version(response.version())
        .headers(take(response.headers_mut()));
    if let Some(port) = response
        .url()
        .port_or_known_default()
        .and_then(NonZeroU16::new)
    {
        response_builder = response_builder.server_port(port);
    }
    if let Some(remote_addr) = response.remote_addr() {
        response_builder = response_builder.server_ip(remote_addr.ip());
        if let Some(port) = NonZeroU16::new(remote_addr.port()) {
            response_builder = response_builder.server_port(port);
        }
    }
    response_builder = response_builder.stream_as_body(Box::new(response));
    Ok(response_builder.build())
}

pub(super) fn from_reqwest_error(err: ReqwestError, request: &Request) -> ResponseError {
    if err.url().is_some() {
        ResponseError::builder(ResponseErrorKind::InvalidURL, err)
            .uri(request.url())
            .build()
    } else if err.is_redirect() {
        ResponseError::builder(ResponseErrorKind::TooManyRedirect, err)
            .uri(request.url())
            .build()
    } else if err.is_timeout() {
        ResponseError::builder(ResponseErrorKind::TimeoutError, err)
            .uri(request.url())
            .build()
    } else if err.is_request() {
        ResponseError::builder(ResponseErrorKind::InvalidRequestResponse, err)
            .uri(request.url())
            .build()
    } else if err.is_connect() {
        ResponseError::builder(ResponseErrorKind::ConnectError, err)
            .uri(request.url())
            .build()
    } else {
        ResponseError::builder(ResponseErrorKind::UnknownError, err)
            .uri(request.url())
            .build()
    }
}

#[inline]
pub(super) fn call_response_callbacks(
    request: &Request,
    status_code: StatusCode,
    headers: &HeaderMap,
) -> Result<(), ResponseError> {
    if let Some(on_receive_response_status) = request.on_receive_response_status() {
        if !on_receive_response_status(status_code) {
            return Err(ResponseError::builder(
                ResponseErrorKind::UserCanceled,
                "on_receive_response_status() returns false",
            )
            .uri(request.url())
            .build());
        }
    }
    if let Some(on_receive_response_header) = request.on_receive_response_header() {
        for (header_name, header_value) in headers.iter() {
            if !on_receive_response_header(header_name, header_value) {
                return Err(ResponseError::builder(
                    ResponseErrorKind::UserCanceled,
                    "on_receive_response_header() returns false",
                )
                .uri(request.url())
                .build());
            }
        }
    }
    Ok(())
}
