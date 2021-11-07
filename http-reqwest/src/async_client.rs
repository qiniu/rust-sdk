use super::{
    extensions::TimeoutExtension,
    sync_client::{call_response_callbacks, from_reqwest_error, make_user_agent},
    BoxFuture,
};
use bytes::Bytes;
use futures::{io::Cursor, ready, AsyncRead, Stream};
use qiniu_http::{
    AsyncResponse, AsyncResponseResult, HTTPCaller, Request, ResponseError, ResponseErrorKind,
    SyncResponseResult, TransferProgressInfo, Uri,
};
use reqwest::{
    header::USER_AGENT, Body as AsyncBody, Client as AsyncReqwestClient,
    Request as AsyncReqwestRequest, Response as AsyncReqwestResponse, Result as ReqwestResult, Url,
};
use std::{
    error::Error,
    fmt,
    io::{Error as IOError, ErrorKind as IOErrorKind, Result as IOResult},
    mem::{take, transmute},
    num::NonZeroU16,
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
    pub fn new(async_client: AsyncReqwestClient) -> Self {
        Self { async_client }
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
            let mut user_cancelled_error: Option<ResponseError> = None;
            let reqwest_request = make_async_reqwest_request(request, &mut user_cancelled_error)?;
            match self.async_client.execute(reqwest_request).await {
                Ok(reqwest_response) => from_async_response(reqwest_response, request),
                Err(err) => {
                    user_cancelled_error.map_or_else(|| Err(from_reqwest_error(err, request)), Err)
                }
            }
        })
    }
}

fn make_async_reqwest_request(
    request: &Request,
    user_cancelled_error: &mut Option<ResponseError>,
) -> Result<AsyncReqwestRequest, ResponseError> {
    let url = Url::parse(&request.url().to_string()).map_err(|err| {
        ResponseError::builder(ResponseErrorKind::InvalidURL, err)
            .uri(request.url())
            .build()
    })?;
    let mut reqwest_request = AsyncReqwestRequest::new(request.method().to_owned(), url);
    for (header_name, header_value) in request.headers() {
        reqwest_request
            .headers_mut()
            .insert(header_name, header_value.to_owned());
    }
    reqwest_request
        .headers_mut()
        .insert(USER_AGENT, make_user_agent(request, "async")?);
    *reqwest_request.body_mut() = Some(AsyncBody::wrap_stream(RequestBodyWithCallbacks::new(
        request.url(),
        request.body(),
        request.on_uploading_progress(),
        user_cancelled_error,
    )));
    if let Some(timeout) = request.extensions().get::<TimeoutExtension>() {
        *reqwest_request.timeout_mut() = Some(timeout.get());
    }
    return Ok(reqwest_request);

    type OnProgress<'r> = &'r (dyn Fn(&TransferProgressInfo) -> bool + Send + Sync);

    struct RequestBodyWithCallbacks {
        request_uri: &'static Uri,
        body: Cursor<&'static [u8]>,
        size: usize,
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
                size: body.len(),
                body: Cursor::new(unsafe { transmute(body) }),
                on_uploading_progress: on_uploading_progress
                    .map(|callback| unsafe { transmute(callback) }),
                user_cancelled_error: unsafe { transmute(user_cancelled_error) },
                request_uri: unsafe { transmute(request_uri) },
            }
        }
    }

    impl Stream for RequestBodyWithCallbacks {
        type Item = Result<Vec<u8>, Box<dyn Error + Send + Sync>>;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            const BUF_LEN: usize = 32 * 1024;
            let mut buf = [0u8; BUF_LEN];
            match ready!(Pin::new(&mut self.as_mut().body).poll_read(cx, &mut buf)) {
                Err(err) => Poll::Ready(Some(Err(Box::new(err)))),
                Ok(0) => Poll::Ready(None),
                Ok(n) => {
                    let buf = &buf[..n];
                    if let Some(on_uploading_progress) = self.on_uploading_progress {
                        if !on_uploading_progress(&TransferProgressInfo::new(
                            self.body.position(),
                            self.size as u64,
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
                            return Poll::Ready(Some(Err(Box::new(IOError::new(
                                IOErrorKind::Other,
                                ERROR_MESSAGE,
                            )))));
                        }
                    }
                    Poll::Ready(Some(Ok(buf.to_vec())))
                }
            }
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.body.position() as usize, Some(self.size))
        }
    }
}

fn from_async_response(
    mut response: AsyncReqwestResponse,
    request: &Request,
) -> AsyncResponseResult {
    call_response_callbacks(request, response.status(), response.headers())?;
    let mut response_builder = AsyncResponse::builder()
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
    response_builder = response_builder.stream_as_body(Box::new(
        AsyncReqwestResponseReadWrapper::new(response.bytes_stream()),
    ));
    return Ok(response_builder.build());

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
                match ready!(stream.poll_next(cx)) {
                    None => Poll::Ready(Ok(0)),
                    Some(Err(err)) => Poll::Ready(Err(IOError::new(IOErrorKind::Other, err))),
                    Some(Ok(data)) => {
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
}
