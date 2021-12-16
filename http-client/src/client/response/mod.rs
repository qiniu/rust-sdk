use qiniu_http::{
    Extensions, HeaderMap, HeaderName, HeaderValue, Metrics, Response as HttpResponse,
    ResponseErrorKind as HttpResponseErrorKind, ResponseParts as HttpResponseParts, StatusCode,
    Version,
};
use serde::{de::DeserializeOwned, Deserialize};
use std::{io::copy as io_copy, net::IpAddr, num::NonZeroU16};

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
pub struct Response<B> {
    inner: HttpResponse<B>,
}

impl<B> Response<B> {
    pub(super) fn new(inner: HttpResponse<B>) -> Self {
        Self { inner }
    }

    /// HTTP 状态码
    #[inline]
    pub fn status_code(&self) -> StatusCode {
        self.inner.status_code()
    }

    /// 修改 HTTP 状态码
    #[inline]
    pub fn status_code_mut(&mut self) -> &mut StatusCode {
        self.inner.status_code_mut()
    }

    /// HTTP Headers
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    /// 修改 HTTP Headers
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.inner.headers_mut()
    }

    /// HTTP 版本
    #[inline]
    pub fn version(&self) -> Version {
        self.inner.version()
    }

    /// 修改 HTTP 版本
    #[inline]
    pub fn version_mut(&mut self) -> &mut Version {
        self.inner.version_mut()
    }

    /// HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.inner.server_ip()
    }

    /// 修改 HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip_mut(&mut self) -> &mut Option<IpAddr> {
        self.inner.server_ip_mut()
    }

    /// HTTP 服务器端口号
    #[inline]
    pub fn server_port(&self) -> Option<NonZeroU16> {
        self.inner.server_port()
    }

    /// 修改 HTTP 服务器端口号
    #[inline]
    pub fn server_port_mut(&mut self) -> &mut Option<NonZeroU16> {
        self.inner.server_port_mut()
    }

    /// 获取 HTTP 响应 Header
    #[inline]
    pub fn header(&self, header_name: HeaderName) -> Option<&HeaderValue> {
        self.inner.header(header_name)
    }

    /// 扩展字段
    #[inline]
    pub fn extensions(&self) -> &Extensions {
        self.inner.extensions()
    }

    /// 修改扩展字段
    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        self.inner.extensions_mut()
    }

    #[inline]
    pub fn metrics(&self) -> Option<&dyn Metrics> {
        self.inner.metrics()
    }

    #[inline]
    pub fn metrics_mut(&mut self) -> &mut Option<Box<dyn Metrics>> {
        self.inner.metrics_mut()
    }

    /// HTTP 响应体
    #[inline]
    pub fn body(&self) -> &B {
        self.inner.body()
    }

    /// 直接获取 HTTP 响应体
    #[inline]
    pub fn into_body(self) -> B {
        self.inner.into_body()
    }

    /// 修改 HTTP 响应体
    #[inline]
    pub fn body_mut(&mut self) -> &mut B {
        self.inner.body_mut()
    }

    #[inline]
    pub fn parts(&self) -> &HttpResponseParts {
        self.inner.parts()
    }

    #[inline]
    pub fn into_parts(self) -> (HttpResponseParts, B) {
        self.inner.into_parts()
    }

    #[inline]
    pub fn from_parts(parts: HttpResponseParts, body: B) -> Self {
        Self {
            inner: HttpResponse::from_parts(parts, body),
        }
    }

    #[inline]
    pub fn parts_mut(&mut self) -> &mut HttpResponseParts {
        self.inner.parts_mut()
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
        self.inner
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
        self.inner
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
