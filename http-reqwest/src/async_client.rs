use super::{
    builder::ReqwestHTTPCallerBuilder,
    sync_client::{call_callbacks, from_reqwest_error, make_user_agent},
    BoxFuture,
};
use bytes::Bytes;
use futures_lite::{AsyncRead, Stream};
use qiniu_http::{
    AsyncResponse, AsyncResponseResult, HTTPCaller, Request, ResponseError, ResponseErrorKind,
    SyncResponseResult,
};
use reqwest::{
    header::USER_AGENT, Client as AsyncReqwestClient, Request as AsyncReqwestRequest,
    Response as AsyncReqwestResponse, Result as ReqwestResult, Url,
};
use std::{
    any::Any,
    fmt,
    io::{Error as IOError, ErrorKind as IOErrorKind, Result as IOResult},
    mem::take,
    pin::Pin,
    task::{Context, Poll},
};

#[cfg_attr(feature = "docs", doc(cfg(r#async)))]
#[derive(Debug, Default)]
pub struct AsyncReqwestHTTPCaller {
    async_client: AsyncReqwestClient,
}

impl AsyncReqwestHTTPCaller {
    #[inline]
    pub fn builder() -> ReqwestHTTPCallerBuilder {
        Default::default()
    }

    #[inline]
    pub(super) fn new(builder: ReqwestHTTPCallerBuilder) -> ReqwestResult<Self> {
        Ok(Self {
            async_client: builder.build_async_client_builder().build()?,
        })
    }
}

impl HTTPCaller for AsyncReqwestHTTPCaller {
    #[inline]
    fn call(&self, _request: &Request) -> SyncResponseResult {
        unimplemented!("AsyncReqwestHTTPCaller does not support blocking call")
    }

    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_call<'a>(&'a self, request: &'a Request) -> BoxFuture<'a, AsyncResponseResult> {
        Box::pin(async move {
            let reqwest_request = make_async_reqwest_request(request)?;
            let reqwest_response = self
                .async_client
                .execute(reqwest_request)
                .await
                .map_err(from_reqwest_error)?;
            from_async_response(reqwest_response, request)
        })
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

struct AsyncReqwestResponseReadWrapper<S: Stream<Item = ReqwestResult<Bytes>>> {
    stream: S,
    buffer: Vec<u8>,
    used: usize,
}

impl<S: Stream<Item = ReqwestResult<Bytes>>> AsyncReqwestResponseReadWrapper<S> {
    #[inline]
    fn new(stream: S) -> Self {
        AsyncReqwestResponseReadWrapper {
            stream,
            buffer: Default::default(),
            used: 0,
        }
    }
}

impl<S: Stream<Item = ReqwestResult<Bytes>>> fmt::Debug for AsyncReqwestResponseReadWrapper<S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AsyncReqwestResponseReadWrapper")
            .field("buffer_len", &self.buffer.len())
            .field("buffer_cap", &self.buffer.capacity())
            .field("used", &self.used)
            .finish()
    }
}

impl<S: Stream<Item = ReqwestResult<Bytes>>> AsyncRead for AsyncReqwestResponseReadWrapper<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IOResult<usize>> {
        let oriself = unsafe { self.get_unchecked_mut() };
        let buffer_rested = oriself.buffer.len() - oriself.used;
        if oriself.buffer.is_empty() {
            let stream = unsafe { Pin::new_unchecked(&mut oriself.stream) };
            match stream.poll_next(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(None) => Poll::Ready(Ok(0)),
                Poll::Ready(Some(Err(err))) => {
                    Poll::Ready(Err(IOError::new(IOErrorKind::Other, err)))
                }
                Poll::Ready(Some(Ok(data))) => {
                    if data.len() <= buf.len() {
                        buf[..data.len()].copy_from_slice(&data);
                        Poll::Ready(Ok(data.len()))
                    } else {
                        buf.copy_from_slice(&data[..buf.len()]);
                        oriself.buffer.extend_from_slice(&data[buf.len()..]);
                        oriself.used = 0;
                        Poll::Ready(Ok(buf.len()))
                    }
                }
            }
        } else if buf.len() >= buffer_rested {
            buf[..buffer_rested].copy_from_slice(&oriself.buffer[oriself.used..]);
            oriself.buffer.truncate(0);
            oriself.used = 0;
            Poll::Ready(Ok(buffer_rested))
        } else {
            buf.copy_from_slice(&oriself.buffer[oriself.used..(oriself.used + buf.len())]);
            oriself.used += buf.len();
            Poll::Ready(Ok(buf.len()))
        }
    }
}

fn make_async_reqwest_request(request: &Request) -> Result<AsyncReqwestRequest, ResponseError> {
    let url = Url::parse(&request.url().to_string())
        .map_err(|err| ResponseError::new(ResponseErrorKind::InvalidURL, err))?;
    let mut reqwest_request = AsyncReqwestRequest::new(request.method().to_owned(), url);
    for (header_name, header_value) in request.headers() {
        reqwest_request
            .headers_mut()
            .insert(header_name, header_value.to_owned());
    }
    reqwest_request
        .headers_mut()
        .insert(USER_AGENT, make_user_agent(request, "async")?);
    *reqwest_request.body_mut() = Some(request.body().to_vec().into());
    Ok(reqwest_request)
}

fn from_async_response(
    mut response: AsyncReqwestResponse,
    request: &Request,
) -> AsyncResponseResult {
    call_callbacks(request, response.status(), response.headers())?;
    let mut response_builder = AsyncResponse::builder()
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
    response_builder = response_builder.stream_as_body(Box::new(
        AsyncReqwestResponseReadWrapper::new(response.bytes_stream()),
    ));
    Ok(response_builder.build())
}
