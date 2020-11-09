use qiniu_http::{
    CachedResponseBody, HeaderNameOwned, HeaderValueOwned, HeadersOwned, Response as HTTPResponse,
    ResponseBody, ResponseErrorKind as HTTPResponseErrorKind, StatusCode,
};
use serde::de::DeserializeOwned;
use serde_json::from_slice as parse_json_from_slice;
use std::{io::Result as IOResult, net::IpAddr, result, time::Duration};

#[cfg(feature = "async")]
pub use qiniu_http::{AsyncCachedResponseBody, AsyncResponseBody};

mod error;
pub use error::{Error as ResponseError, ErrorKind as ResponseErrorKind};

pub type APIResult<T> = result::Result<T, ResponseError>;

#[cfg(feature = "async")]
use std::{future::Future, pin::Pin};

#[cfg(feature = "async")]
type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

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
    pub fn headers(&self) -> &HeadersOwned {
        self.inner.headers()
    }

    /// 修改 HTTP Headers
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeadersOwned {
        self.inner.headers_mut()
    }

    /// HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip(&self) -> Option<IpAddr> {
        self.inner.server_ip()
    }

    /// 修改 HTTP 服务器 IP 地址
    #[inline]
    pub fn server_ip_mut(&mut self) -> Option<&mut IpAddr> {
        self.inner.server_ip_mut()
    }

    /// HTTP 服务器端口号
    #[inline]
    pub fn server_port(&self) -> u16 {
        self.inner.server_port()
    }

    /// 修改 HTTP 服务器端口号
    #[inline]
    pub fn server_port_mut(&mut self) -> &mut u16 {
        self.inner.server_port_mut()
    }

    /// 获取 HTTP 响应 Header
    #[inline]
    pub fn header(&self, header_name: impl Into<HeaderNameOwned>) -> Option<&HeaderValueOwned> {
        self.inner.header(header_name)
    }

    #[inline]
    pub fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    #[inline]
    pub fn name_lookup_duration(&self) -> Option<Duration> {
        self.inner.name_lookup_duration()
    }

    #[inline]
    pub fn connect_duration(&self) -> Option<Duration> {
        self.inner.connect_duration()
    }

    #[inline]
    pub fn secure_connect_duration(&self) -> Option<Duration> {
        self.inner.secure_connect_duration()
    }

    #[inline]
    pub fn redirect_duration(&self) -> Option<Duration> {
        self.inner.redirect_duration()
    }

    #[inline]
    pub fn transfer_duration(&self) -> Option<Duration> {
        self.inner.transfer_duration()
    }

    #[inline]
    pub fn total_duration_mut(&mut self) -> &mut Option<Duration> {
        self.inner.total_duration_mut()
    }

    #[inline]
    pub fn name_lookup_duration_mut(&mut self) -> &mut Option<Duration> {
        self.inner.name_lookup_duration_mut()
    }

    #[inline]
    pub fn connect_duration_mut(&mut self) -> &mut Option<Duration> {
        self.inner.connect_duration_mut()
    }

    #[inline]
    pub fn secure_connect_duration_mut(&mut self) -> &mut Option<Duration> {
        self.inner.secure_connect_duration_mut()
    }

    #[inline]
    pub fn redirect_duration_mut(&mut self) -> &mut Option<Duration> {
        self.inner.redirect_duration_mut()
    }

    #[inline]
    pub fn transfer_duration_mut(&mut self) -> &mut Option<Duration> {
        self.inner.transfer_duration_mut()
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
        self.header("X-Reqid").map(|v| v.as_str())
    }

    #[inline]
    pub fn x_log(&self) -> Option<&str> {
        self.header("X-Log").map(|v| v.as_str())
    }

    #[inline]
    pub fn map_body<B2>(self, f: impl FnOnce(B) -> B2) -> Response<B2> {
        Response {
            inner: self.inner.map_body(f),
        }
    }

    #[inline]
    pub fn try_map_body<B2, E>(
        self,
        f: impl FnOnce(B) -> result::Result<B2, E>,
    ) -> result::Result<Response<B2>, E> {
        Ok(Response {
            inner: self.inner.try_map_body(f)?,
        })
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub async fn async_map_body<B2>(self, f: impl FnOnce(B) -> BoxFuture<B2>) -> Response<B2> {
        Response {
            inner: self.inner.async_map_body(f).await,
        }
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub async fn async_try_map_body<B2, E>(
        self,
        f: impl FnOnce(B) -> BoxFuture<result::Result<B2, E>>,
    ) -> result::Result<Response<B2>, E> {
        Ok(Response {
            inner: self.inner.async_try_map_body(f).await?,
        })
    }
}

impl Response<ResponseBody> {
    pub fn parse_json<T: DeserializeOwned>(self) -> APIResult<T> {
        let body = self
            .fulfill()
            .map_err(|err| ResponseError::new(HTTPResponseErrorKind::LocalIOError.into(), err))?
            .into_body()
            .into_bytes();
        let body = parse_json_from_slice(&body)
            .map_err(|err| ResponseError::new(ResponseErrorKind::ParseResponseError, err))?;
        Ok(body)
    }

    #[inline]
    pub(super) fn fulfill(self) -> IOResult<Response<CachedResponseBody>> {
        Ok(Response::new(self.inner.fulfill()?))
    }
}

#[cfg(feature = "async")]
impl Response<AsyncResponseBody> {
    pub async fn parse_json<T: DeserializeOwned>(self) -> APIResult<T> {
        let body = self
            .fulfill()
            .await
            .map_err(|err| ResponseError::new(HTTPResponseErrorKind::LocalIOError.into(), err))?
            .into_body()
            .into_bytes();
        let body = parse_json_from_slice(&body)
            .map_err(|err| ResponseError::new(ResponseErrorKind::ParseResponseError, err))?;
        Ok(body)
    }

    #[inline]
    pub(super) async fn fulfill(self) -> IOResult<Response<AsyncCachedResponseBody>> {
        Ok(Response::new(self.inner.fulfill().await?))
    }
}

/// 同步 HTTP 响应
pub type SyncResponse = Response<ResponseBody>;

/// 异步 HTTP 响应
#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(r#async)))]
pub type AsyncResponse = Response<AsyncResponseBody>;
