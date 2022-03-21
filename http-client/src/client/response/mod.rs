use qiniu_http::{
    HeaderName, HeaderValue, Response as HttpResponse, ResponseErrorKind as HttpResponseErrorKind,
    ResponseParts as HttpResponseParts,
};
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    io::copy as io_copy,
    ops::{Deref, DerefMut},
};

mod error;
pub(super) use error::XHeaders;
pub use error::{Error as ResponseError, ErrorKind as ResponseErrorKind};

pub type ApiResult<T> = Result<T, ResponseError>;

pub use qiniu_http::SyncResponseBody;

#[cfg(feature = "async")]
use futures::io::copy as async_io_copy;

#[cfg(feature = "async")]
pub use qiniu_http::AsyncResponseBody;

const X_REQ_ID_HEADER_NAME: &str = "x-reqid";
const X_LOG_HEADER_NAME: &str = "x-log";

#[derive(Default, Debug)]
pub struct Response<B>(HttpResponse<B>);

impl<B> Response<B> {
    pub(super) fn new(inner: HttpResponse<B>) -> Self {
        Self(inner)
    }

    /// 直接获取 HTTP 响应体
    #[inline]
    pub fn into_body(self) -> B {
        self.0.into_body()
    }

    #[inline]
    pub fn into_parts_and_body(self) -> (HttpResponseParts, B) {
        self.0.into_parts_and_body()
    }

    #[inline]
    pub fn from_parts_and_body(parts: HttpResponseParts, body: B) -> Self {
        Self(HttpResponse::from_parts_and_body(parts, body))
    }

    #[inline]
    pub fn x_req_id(&self) -> Option<&HeaderValue> {
        self.header(HeaderName::from_static(X_REQ_ID_HEADER_NAME))
    }

    #[inline]
    pub fn x_log(&self) -> Option<&HeaderValue> {
        self.header(HeaderName::from_static(X_LOG_HEADER_NAME))
    }
}

impl<B> Deref for Response<B> {
    type Target = HttpResponse<B>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<B> DerefMut for Response<B> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Response<SyncResponseBody> {
    pub fn parse_json<T: DeserializeOwned>(self) -> ApiResult<Response<T>> {
        let x_headers = XHeaders::from(self.parts());
        let json_response = self
            .fulfill()?
            .try_map_body(|body| parse_json_from_slice(&body))
            .map_err(|err| {
                ResponseError::from_http_response_error(
                    err.into_response_error(HttpResponseErrorKind::ReceiveError),
                    x_headers,
                    Some(ResponseErrorKind::ParseResponseError),
                )
            })?;
        Ok(Response::new(json_response))
    }

    pub(super) fn fulfill(self) -> ApiResult<HttpResponse<Vec<u8>>> {
        let x_headers = XHeaders::from(self.parts());
        self.0
            .try_map_body(|mut body| {
                let mut buf = Vec::new();
                io_copy(&mut body, &mut buf).map(|_| buf)
            })
            .map_err(|err| {
                ResponseError::from_http_response_error(
                    err.into_response_error(HttpResponseErrorKind::LocalIoError),
                    x_headers,
                    None,
                )
            })
    }
}

#[cfg(feature = "async")]
impl Response<AsyncResponseBody> {
    pub async fn parse_json<T: DeserializeOwned>(self) -> ApiResult<Response<T>> {
        let x_headers = XHeaders::from(self.parts());
        let json_response = self
            .fulfill()
            .await?
            .try_map_body(|body| parse_json_from_slice(&body))
            .map_err(|err| {
                ResponseError::from_http_response_error(
                    err.into_response_error(HttpResponseErrorKind::ReceiveError),
                    x_headers,
                    Some(ResponseErrorKind::ParseResponseError),
                )
            })?;
        Ok(Response::new(json_response))
    }

    pub(super) async fn fulfill(self) -> ApiResult<HttpResponse<Vec<u8>>> {
        let x_headers = XHeaders::from(self.parts());
        self.0
            .try_async_map_body(|mut body| async move {
                let mut buf = Vec::new();
                async_io_copy(&mut body, &mut buf).await.map(|_| buf)
            })
            .await
            .map_err(|err| {
                ResponseError::from_http_response_error(
                    err.into_response_error(HttpResponseErrorKind::LocalIoError),
                    x_headers,
                    None,
                )
            })
    }
}

fn parse_json_from_slice<'a, T: Deserialize<'a>>(v: &'a [u8]) -> serde_json::Result<T> {
    // Sometimes the API are supposed to response with JSON but actually Empty!
    if v.as_ref().is_empty() {
        serde_json::from_slice(&b"{}"[..])
    } else {
        serde_json::from_slice(v)
    }
}

/// 同步 HTTP 响应
pub type SyncResponse = Response<SyncResponseBody>;

/// 异步 HTTP 响应
#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub type AsyncResponse = Response<AsyncResponseBody>;
