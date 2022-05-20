use assert_impl::assert_impl;
use qiniu_credential::HeaderValue;
use qiniu_http::{
    Response as HttpResponse, ResponseErrorKind as HttpResponseErrorKind, ResponseParts as HttpResponseParts,
};
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    io::copy as io_copy,
    mem::take,
    ops::{Deref, DerefMut},
};
use tap::TapFallible;

mod error;
pub(super) use error::XHeaders;
pub use error::{Error as ResponseError, ErrorKind as ResponseErrorKind};

/// API 响应结果
pub type ApiResult<T> = Result<T, ResponseError>;

use qiniu_http::SyncResponseBody;

#[cfg(feature = "async")]
use futures::io::copy as async_io_copy;

#[cfg(feature = "async")]
use qiniu_http::AsyncResponseBody;

const X_REQ_ID_HEADER_NAME: &str = "x-reqid";
const X_LOG_HEADER_NAME: &str = "x-log";

/// HTTP 响应
#[derive(Default, Debug)]
pub struct Response<B>(HttpResponse<B>);

impl<B> Response<B> {
    /// 转换为 HTTP 响应体
    #[inline]
    pub fn into_body(self) -> B {
        self.0.into_body()
    }

    /// 转换为 HTTP 响应信息和响应体
    #[inline]
    pub fn into_parts_and_body(self) -> (HttpResponseParts, B) {
        self.0.into_parts_and_body()
    }

    /// 根据 HTTP 响应信息和响应体创建 HTTP 响应
    #[inline]
    pub fn from_parts_and_body(parts: HttpResponseParts, body: B) -> Self {
        Self(HttpResponse::from_parts_and_body(parts, body))
    }

    /// 获取 HTTP 响应的 X-ReqId 信息
    #[inline]
    pub fn x_reqid(&self) -> Option<&HeaderValue> {
        self.header(X_REQ_ID_HEADER_NAME)
    }

    /// 获取 HTTP 响应的 X-Log 信息
    #[inline]
    pub fn x_log(&self) -> Option<&HeaderValue> {
        self.header(X_LOG_HEADER_NAME)
    }
}

impl<B> From<HttpResponse<B>> for Response<B> {
    #[inline]
    fn from(response: HttpResponse<B>) -> Self {
        Self(response)
    }
}

impl<B> From<Response<B>> for HttpResponse<B> {
    #[inline]
    fn from(response: Response<B>) -> Self {
        response.0
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

impl<B: Sync + Send> Response<B> {
    #[allow(dead_code)]
    fn assert() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl Response<SyncResponseBody> {
    /// 解析 JSON 响应体
    pub fn parse_json<T: DeserializeOwned>(self) -> ApiResult<Response<T>> {
        let x_headers = XHeaders::from(self.parts());
        let mut got_body = Vec::new();
        let json_response = self
            .fulfill()?
            .try_map_body(|mut body| parse_json_from_slice(&body).tap_err(|_| got_body = take(&mut body)))
            .map_err(|err| {
                ResponseError::from_http_response_error(
                    err.into_response_error(HttpResponseErrorKind::ReceiveError),
                    x_headers,
                    Some(ResponseErrorKind::ParseResponseError),
                )
                .set_response_body_sample(got_body)
            })?;
        Ok(Response::from(json_response))
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
                    err.into_response_error(HttpResponseErrorKind::ReceiveError),
                    x_headers,
                    None,
                )
            })
    }
}

#[cfg(feature = "async")]
impl Response<AsyncResponseBody> {
    /// 异步解析 JSON 响应体
    pub async fn parse_json<T: DeserializeOwned>(self) -> ApiResult<Response<T>> {
        let x_headers = XHeaders::from(self.parts());
        let mut got_body = Vec::new();
        let json_response = self
            .fulfill()
            .await?
            .try_map_body(|mut body| parse_json_from_slice(&body).tap_err(|_| got_body = take(&mut body)))
            .map_err(|err| {
                ResponseError::from_http_response_error(
                    err.into_response_error(HttpResponseErrorKind::ReceiveError),
                    x_headers,
                    Some(ResponseErrorKind::ParseResponseError),
                )
                .set_response_body_sample(got_body)
            })?;
        Ok(Response::from(json_response))
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
                    err.into_response_error(HttpResponseErrorKind::ReceiveError),
                    x_headers,
                    None,
                )
            })
    }
}

fn parse_json_from_slice<'a, T: Deserialize<'a>>(v: &'a [u8]) -> serde_json::Result<T> {
    // Sometimes the API are supposed to response with JSON but actually Empty!
    if v.as_ref().is_empty() {
        serde_json::from_slice(b"{}".as_slice())
    } else {
        serde_json::from_slice(v)
    }
}

/// 阻塞 HTTP 响应
pub type SyncResponse = Response<SyncResponseBody>;

/// 异步 HTTP 响应
#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub type AsyncResponse = Response<AsyncResponseBody>;
