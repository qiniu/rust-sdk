use qiniu_http::{
    Extensions, HeaderMap, HeaderName, HeaderValue, Metrics, Response as HTTPResponse,
    ResponseErrorKind as HTTPResponseErrorKind, ResponseResult as HTTPResponseResult, StatusCode,
    SyncResponseBody, Version,
};
use serde::de::DeserializeOwned;
use serde_json::from_slice as parse_json_from_slice;
use std::{io::copy as io_copy, net::IpAddr, num::NonZeroU16};

mod error;
pub use error::{Error as ResponseError, ErrorKind as ResponseErrorKind};

pub type APIResult<T> = Result<T, ResponseError>;

#[cfg(feature = "async")]
use {futures::io::copy as async_io_copy, qiniu_http::AsyncResponseBody};

#[derive(Default, Debug)]
pub struct Response<B> {
    inner: HTTPResponse<B>,
}

impl<B> Response<B> {
    #[inline]
    pub(super) fn new(inner: HTTPResponse<B>) -> Self {
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
    pub fn x_req_id(&self) -> Option<&str> {
        self.header(HeaderName::from_static("x-reqid"))
            .and_then(|v| v.to_str().ok())
    }

    #[inline]
    pub fn x_log(&self) -> Option<&str> {
        self.header(HeaderName::from_static("x-log"))
            .and_then(|v| v.to_str().ok())
    }
}

impl Response<SyncResponseBody> {
    pub fn parse_json<T: DeserializeOwned>(self) -> APIResult<Response<T>> {
        let json_response = self
            .fulfill()?
            .try_map_body(|body| parse_json_from_slice(&body))
            .map_err(|err| {
                ResponseError::from_http_response_error(
                    ResponseErrorKind::ParseResponseError,
                    err.into_response_error(HTTPResponseErrorKind::ReceiveError),
                )
            })?;
        Ok(Response::new(json_response))
    }

    #[inline]
    pub(super) fn fulfill(self) -> HTTPResponseResult<Vec<u8>> {
        self.inner
            .try_map_body(|mut body| {
                let mut buf = Vec::new();
                io_copy(&mut body, &mut buf).map(|_| buf)
            })
            .map_err(|err| err.into_response_error(HTTPResponseErrorKind::LocalIOError))
    }
}

#[cfg(feature = "async")]
impl Response<AsyncResponseBody> {
    pub async fn parse_json<T: DeserializeOwned>(self) -> APIResult<Response<T>> {
        let json_response = self
            .fulfill()
            .await?
            .try_map_body(|body| parse_json_from_slice(&body))
            .map_err(|err| {
                ResponseError::from_http_response_error(
                    ResponseErrorKind::ParseResponseError,
                    err.into_response_error(HTTPResponseErrorKind::ReceiveError),
                )
            })?;
        Ok(Response::new(json_response))
    }

    #[inline]
    pub(super) async fn fulfill(self) -> HTTPResponseResult<Vec<u8>> {
        self.inner
            .try_async_map_body(|mut body| async move {
                let mut buf = Vec::new();
                async_io_copy(&mut body, &mut buf).await.map(|_| buf)
            })
            .await
            .map_err(|err| err.into_response_error(HTTPResponseErrorKind::LocalIOError))
    }
}

/// 同步 HTTP 响应
pub type SyncResponse = Response<SyncResponseBody>;

/// 异步 HTTP 响应
#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(r#async)))]
pub type AsyncResponse = Response<AsyncResponseBody>;
