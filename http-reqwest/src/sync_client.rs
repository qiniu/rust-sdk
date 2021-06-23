use super::builder::ReqwestHTTPCallerBuilder;
use qiniu_http::{
    HTTPCaller, HeaderMap, HeaderValue, Request, ResponseError, ResponseErrorKind, StatusCode,
    SyncResponse, SyncResponseResult,
};
use reqwest::{
    blocking::{
        Client as SyncReqwestClient, Request as SyncReqwestRequest, Response as SyncReqwestResponse,
    },
    header::USER_AGENT,
    Error as ReqwestError, Result as ReqwestResult, Url,
};
use std::{any::Any, mem::take};

#[cfg(feature = "async")]
use {super::BoxFuture, qiniu_http::AsyncResponseResult};

#[derive(Debug, Default)]
pub struct SyncReqwestHTTPCaller {
    sync_client: SyncReqwestClient,
}

impl SyncReqwestHTTPCaller {
    #[inline]
    pub fn builder() -> ReqwestHTTPCallerBuilder {
        Default::default()
    }

    #[inline]
    pub(super) fn new(builder: ReqwestHTTPCallerBuilder) -> ReqwestResult<Self> {
        Ok(Self {
            sync_client: builder.build_sync_client_builder().build()?,
        })
    }
}

impl HTTPCaller for SyncReqwestHTTPCaller {
    #[inline]
    fn call(&self, request: &Request) -> SyncResponseResult {
        let reqwest_request = make_sync_reqwest_request(request)?;
        let reqwest_response = self
            .sync_client
            .execute(reqwest_request)
            .map_err(from_reqwest_error)?;
        from_sync_response(reqwest_response, request)
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

fn make_sync_reqwest_request(request: &Request) -> Result<SyncReqwestRequest, ResponseError> {
    let url = Url::parse(&request.url().to_string())
        .map_err(|err| ResponseError::new(ResponseErrorKind::InvalidURL, err))?;
    let mut reqwest_request = SyncReqwestRequest::new(request.method().to_owned(), url);
    for (header_name, header_value) in request.headers() {
        reqwest_request
            .headers_mut()
            .insert(header_name, header_value.to_owned());
    }
    reqwest_request
        .headers_mut()
        .insert(USER_AGENT, make_user_agent(request, "sync")?);
    *reqwest_request.body_mut() = Some(request.body().to_vec().into());
    Ok(reqwest_request)
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
    .map_err(|err| ResponseError::new(ResponseErrorKind::InvalidHeader, err))
}

fn from_sync_response(mut response: SyncReqwestResponse, request: &Request) -> SyncResponseResult {
    call_callbacks(request, response.status(), response.headers())?;
    let mut response_builder = SyncResponse::builder()
        .status_code(response.status())
        .headers(take(response.headers_mut()));
    if let Some(port) = response.url().port_or_known_default() {
        response_builder = response_builder.server_port(port);
    }
    if let Some(remote_addr) = response.remote_addr() {
        response_builder = response_builder
            .server_ip(remote_addr.ip())
            .server_port(remote_addr.port());
    }
    response_builder = response_builder.stream_as_body(Box::new(response));
    Ok(response_builder.build())
}

pub(super) fn from_reqwest_error(err: ReqwestError) -> ResponseError {
    if err.url().is_some() {
        ResponseError::new(ResponseErrorKind::InvalidURL, err)
    } else if err.is_redirect() {
        ResponseError::new(ResponseErrorKind::TooManyRedirect, err)
    } else if err.is_timeout() {
        ResponseError::new(ResponseErrorKind::TimeoutError, err)
    } else if err.is_request() {
        ResponseError::new(ResponseErrorKind::InvalidRequestResponse, err)
    } else if err.is_connect() {
        ResponseError::new(ResponseErrorKind::ConnectError, err)
    } else {
        ResponseError::new(ResponseErrorKind::UnknownError, err)
    }
}

#[inline]
pub(super) fn call_callbacks(
    request: &Request,
    status: StatusCode,
    headers: &HeaderMap,
) -> Result<(), ResponseError> {
    if let Some(on_receive_response_status) = request.on_receive_response_status() {
        if !on_receive_response_status(status) {
            return Err(ResponseError::new(
                ResponseErrorKind::UserCanceled,
                "on_receive_response_status() returns false",
            ));
        }
    }
    if let Some(on_receive_response_header) = request.on_receive_response_header() {
        for (header_name, header_value) in headers.iter() {
            if !on_receive_response_header(header_name, header_value) {
                return Err(ResponseError::new(
                    ResponseErrorKind::UserCanceled,
                    "on_receive_response_header() returns false",
                ));
            }
        }
    }
    Ok(())
}
