use super::extensions::TimeoutExtension;
use anyhow::Error as AnyError;
use qiniu_http::{
    HeaderMap, HeaderValue, HttpCaller, RequestParts, ResponseError, ResponseErrorKind, StatusCode, SyncRequest,
    SyncResponse, SyncResponseBody, SyncResponseResult, TransferProgressInfo,
};
use reqwest::{
    blocking::{
        Body as SyncBody, Client as SyncReqwestClient, Request as SyncReqwestRequest, Response as SyncReqwestResponse,
    },
    header::USER_AGENT,
    Error as ReqwestError, Url,
};
use std::{
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult},
    mem::take,
    mem::transmute,
    num::NonZeroU16,
};

#[cfg(feature = "async")]
use {
    futures::future::BoxFuture,
    qiniu_http::{AsyncRequest, AsyncResponseResult},
};

/// Reqwest 阻塞客户端
#[derive(Debug, Default)]
pub struct SyncClient {
    sync_client: SyncReqwestClient,
}

impl SyncClient {
    /// 创建 Reqwest 阻塞客户端
    #[inline]
    pub fn new(sync_client: SyncReqwestClient) -> Self {
        Self { sync_client }
    }
}

impl From<SyncReqwestClient> for SyncClient {
    #[inline]
    fn from(sync_client: SyncReqwestClient) -> Self {
        Self::new(sync_client)
    }
}

impl HttpCaller for SyncClient {
    fn call<'a>(&'a self, request: &'a mut SyncRequest<'_>) -> SyncResponseResult {
        let mut user_cancelled_error: Option<ResponseError> = None;
        let reqwest_request = make_sync_reqwest_request(request, &mut user_cancelled_error)?;
        match self.sync_client.execute(reqwest_request) {
            Ok(reqwest_response) => from_sync_response(reqwest_response, request),
            Err(err) => user_cancelled_error.map_or_else(|| Err(from_reqwest_error(err, request)), Err),
        }
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_call<'a>(&'a self, _request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
        unimplemented!("SyncClient does not support async call")
    }
}

fn make_sync_reqwest_request(
    request: &mut SyncRequest,
    user_cancelled_error: &mut Option<ResponseError>,
) -> Result<SyncReqwestRequest, ResponseError> {
    let url = Url::parse(&request.url().to_string()).map_err(|err| {
        ResponseError::builder(ResponseErrorKind::InvalidUrl, err)
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
        RequestBodyWithCallbacks::new(request, user_cancelled_error),
        request.body().size(),
    ));

    if let Some(timeout) = request.extensions().get::<TimeoutExtension>() {
        *reqwest_request.timeout_mut() = Some(timeout.get());
    }

    return Ok(reqwest_request);

    struct RequestBodyWithCallbacks {
        request: &'static mut SyncRequest<'static>,
        user_cancelled_error: &'static mut Option<ResponseError>,
        have_read: u64,
    }

    impl RequestBodyWithCallbacks {
        fn new(request: &mut SyncRequest, user_cancelled_error: &mut Option<ResponseError>) -> Self {
            #[allow(unsafe_code)]
            Self {
                have_read: 0,
                request: unsafe { transmute(request) },
                user_cancelled_error: unsafe { transmute(user_cancelled_error) },
            }
        }
    }

    impl Read for RequestBodyWithCallbacks {
        fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
            let n = self.request.body_mut().read(buf)?;
            match n {
                0 => Ok(0),
                n => {
                    let buf = &buf[..n];
                    self.have_read += n as u64;
                    if let Some(on_uploading_progress) = self.request.on_uploading_progress() {
                        on_uploading_progress(TransferProgressInfo::new(
                            self.have_read,
                            self.request.body().size(),
                            buf,
                        ))
                        .map_err(|err| {
                            *self.user_cancelled_error = Some(make_callback_error(err, self.request));
                            IoError::new(IoErrorKind::Other, "on_uploading_progress() callback returns error")
                        })?;
                    }
                    Ok(n)
                }
            }
        }
    }
}

pub(super) fn make_user_agent(request: &RequestParts, suffix: &str) -> Result<HeaderValue, ResponseError> {
    HeaderValue::from_str(&format!(
        "{}/qiniu-reqwest-{}/{}",
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

fn from_sync_response(mut response: SyncReqwestResponse, request: &mut SyncRequest) -> SyncResponseResult {
    call_response_callbacks(request, response.status(), response.headers())?;
    let mut response_builder = SyncResponse::builder();
    response_builder
        .status_code(response.status())
        .version(response.version())
        .headers(take(response.headers_mut()))
        .extensions(take(request.extensions_mut()));
    if let Some(port) = response.url().port_or_known_default().and_then(NonZeroU16::new) {
        response_builder.server_port(port);
    }
    if let Some(remote_addr) = response.remote_addr() {
        response_builder.server_ip(remote_addr.ip());
        if let Some(port) = NonZeroU16::new(remote_addr.port()) {
            response_builder.server_port(port);
        }
    }
    response_builder.body(SyncResponseBody::from_reader(response));
    Ok(response_builder.build())
}

pub(super) fn from_reqwest_error(err: ReqwestError, request: &RequestParts) -> ResponseError {
    if err.url().is_some() {
        ResponseError::builder(ResponseErrorKind::InvalidUrl, err)
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

pub(super) fn call_response_callbacks(
    request: &RequestParts,
    status_code: StatusCode,
    headers: &HeaderMap,
) -> Result<(), ResponseError> {
    if let Some(on_receive_response_status) = request.on_receive_response_status() {
        on_receive_response_status(status_code).map_err(|err| make_callback_error(err, request))?;
    }
    if let Some(on_receive_response_header) = request.on_receive_response_header() {
        headers.iter().try_for_each(|(header_name, header_value)| {
            on_receive_response_header(header_name, header_value).map_err(|err| make_callback_error(err, request))
        })?;
    }
    Ok(())
}

pub(super) fn make_callback_error(err: AnyError, request: &RequestParts) -> ResponseError {
    ResponseError::builder(ResponseErrorKind::CallbackError, err)
        .uri(request.url())
        .build()
}
