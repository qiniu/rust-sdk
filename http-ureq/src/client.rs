use anyhow::Error as AnyError;
use qiniu_http::{
    header::{CONTENT_LENGTH, USER_AGENT},
    HeaderName, HeaderValue, HttpCaller, RequestParts, ResponseError, ResponseErrorKind, StatusCode, SyncRequest,
    SyncResponse, SyncResponseBody, SyncResponseResult, TransferProgressInfo, Version,
};
use std::{
    fmt::{self, Display},
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult},
    mem::take,
};
use ureq::{Agent, Error as UreqError, ErrorKind as UreqErrorKind, Request as UreqRequest, Response as UreqResponse};

#[cfg(feature = "async")]
use {
    super::BoxFuture,
    qiniu_http::{AsyncRequest, AsyncResponseResult},
};

/// Ureq 客户端
#[derive(Debug, Clone)]
pub struct Client {
    client: Agent,
}

impl Client {
    /// 创建 Ureq 客户端
    #[inline]
    pub fn new(client: Agent) -> Self {
        Self { client }
    }
}

impl From<Agent> for Client {
    #[inline]
    fn from(agent: Agent) -> Self {
        Self::new(agent)
    }
}

impl Default for Client {
    #[inline]
    fn default() -> Self {
        Self { client: ureq::agent() }
    }
}

impl HttpCaller for Client {
    fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
        let mut user_cancelled_error: Option<ResponseError> = None;

        let ureq_request = make_ureq_request(&self.client, request)?;
        match ureq_request.send(RequestBodyWithCallbacks::new(request, &mut user_cancelled_error)) {
            Ok(response) => make_ureq_sync_response(response, request),
            Err(err) => {
                let kind = err.kind();
                match err {
                    UreqError::Status(_, response) => make_ureq_sync_response(response, request),
                    UreqError::Transport(transport) => user_cancelled_error
                        .map_or_else(|| Err(from_ureq_error(kind, AnyError::new(transport), request)), Err),
                }
            }
        }
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_call<'a>(&'a self, _request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
        unimplemented!("http_ureq::Client does not support async call")
    }
}

fn make_user_agent(request: &RequestParts) -> Result<HeaderValue, ResponseError> {
    let user_agent = format!("{}/qiniu-ureq", request.user_agent());
    HeaderValue::from_str(&user_agent).map_err(|err| build_header_value_error(request, &user_agent, &err))
}

fn make_ureq_request(agent: &Agent, request: &SyncRequest) -> Result<UreqRequest, ResponseError> {
    let mut request_builder = agent.request(request.method().as_str(), &request.url().to_string());
    for (header_name, header_value) in request.headers() {
        request_builder = set_header_for_request_builder(request_builder, request, header_name, header_value)?;
    }
    request_builder =
        set_header_for_request_builder(request_builder, request, &USER_AGENT, &make_user_agent(request)?)?;
    request_builder = request_builder.set(CONTENT_LENGTH.as_str(), &request.body().size().to_string());
    request_builder = add_extensions_to_request_builder(request, request_builder);
    Ok(request_builder)
}

fn make_ureq_sync_response(response: UreqResponse, request: &mut SyncRequest) -> SyncResponseResult {
    call_response_callbacks(request, &response)?;

    let mut response_builder = SyncResponse::builder();
    response_builder
        .status_code(status_code_of_response(&response, request)?)
        .version(parse_http_version(response.http_version(), request)?)
        .extensions(take(request.extensions_mut()));
    for header_name_str in response.headers_names().into_iter() {
        if let Some(header_value_str) = response.header(&header_name_str) {
            let header_name = HeaderName::from_bytes(header_name_str.as_bytes())
                .map_err(|err| build_header_name_error(request, &header_name_str, &err))?;
            let header_value = HeaderValue::from_bytes(header_value_str.as_bytes())
                .map_err(|err| build_header_value_error(request, header_value_str, &err))?;
            response_builder.header(header_name, header_value);
        }
    }
    response_builder.body(SyncResponseBody::from_reader(ResponseReaderWrapper(
        response.into_reader(),
    )));
    return Ok(response_builder.build());

    struct ResponseReaderWrapper<R>(R);

    impl<R: Read> Read for ResponseReaderWrapper<R> {
        #[inline]
        fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
            self.0.read(buf)
        }
    }

    impl<R> fmt::Debug for ResponseReaderWrapper<R> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_tuple("ResponseReaderWrapper").finish()
        }
    }
}

fn add_extensions_to_request_builder(request: &RequestParts, mut request_builder: UreqRequest) -> UreqRequest {
    use super::extensions::TimeoutExtension;

    if let Some(extension) = request.extensions().get::<TimeoutExtension>() {
        request_builder = request_builder.timeout(extension.get());
    }

    request_builder
}

fn call_response_callbacks(request: &RequestParts, response: &UreqResponse) -> Result<(), ResponseError> {
    if let Some(on_receive_response_status) = request.on_receive_response_status() {
        on_receive_response_status(status_code_of_response(response, request)?)
            .map_err(|err| build_on_receive_response_status_error(request, err))?;
    }
    if let Some(on_receive_response_header) = request.on_receive_response_header() {
        for header_name_str in response.headers_names().into_iter() {
            if let Some(header_value_str) = response.header(&header_name_str) {
                let header_name = HeaderName::from_bytes(header_name_str.as_bytes())
                    .map_err(|err| build_header_name_error(request, &header_name_str, &err))?;
                let header_value = HeaderValue::from_bytes(header_value_str.as_bytes())
                    .map_err(|err| build_header_value_error(request, header_value_str, &err))?;
                on_receive_response_header(&header_name, &header_value)
                    .map_err(|err| build_on_receive_response_header_error(request, err))?
            }
        }
    }
    Ok(())
}

fn build_on_receive_response_status_error(request: &RequestParts, err: AnyError) -> ResponseError {
    ResponseError::builder(ResponseErrorKind::CallbackError, err)
        .uri(request.url())
        .build()
}

fn build_on_receive_response_header_error(request: &RequestParts, err: AnyError) -> ResponseError {
    ResponseError::builder(ResponseErrorKind::CallbackError, err)
        .uri(request.url())
        .build()
}

fn build_status_code_error(request: &RequestParts, code: u16, err: &dyn Display) -> ResponseError {
    ResponseError::builder_with_msg(
        ResponseErrorKind::InvalidRequestResponse,
        format!("invalid status code({}): {}", code, err),
    )
    .uri(request.url())
    .build()
}

fn build_header_name_error(request: &RequestParts, header_name: &str, err: &dyn Display) -> ResponseError {
    ResponseError::builder_with_msg(
        ResponseErrorKind::InvalidHeader,
        format!("invalid header name({}): {}", header_name, err),
    )
    .uri(request.url())
    .build()
}

fn build_header_value_error(request: &RequestParts, header_value: &str, err: &dyn Display) -> ResponseError {
    ResponseError::builder_with_msg(
        ResponseErrorKind::InvalidHeader,
        format!("invalid header value({}): {}", header_value, err),
    )
    .uri(request.url())
    .build()
}

fn convert_header_value_error(request: &RequestParts, header_value: &HeaderValue, err: &dyn Display) -> ResponseError {
    ResponseError::builder_with_msg(
        ResponseErrorKind::InvalidHeader,
        format!("invalid header value({:?}): {}", header_value, err),
    )
    .uri(request.url())
    .build()
}

fn set_header_for_request_builder(
    request_builder: UreqRequest,
    request: &RequestParts,
    header_name: &HeaderName,
    header_value: &HeaderValue,
) -> Result<UreqRequest, ResponseError> {
    Ok(request_builder.set(
        header_name.as_str(),
        header_value
            .to_str()
            .map_err(|err| convert_header_value_error(request, header_value, &err))?,
    ))
}

fn status_code_of_response(response: &UreqResponse, request: &RequestParts) -> Result<StatusCode, ResponseError> {
    StatusCode::from_u16(response.status()).map_err(|err| build_status_code_error(request, response.status(), &err))
}

fn parse_http_version(version: &str, request: &RequestParts) -> Result<Version, ResponseError> {
    match version {
        "HTTP/0.9" => Ok(Version::HTTP_09),
        "HTTP/1.0" => Ok(Version::HTTP_10),
        "HTTP/1.1" => Ok(Version::HTTP_11),
        "HTTP/2.0" => Ok(Version::HTTP_2),
        "HTTP/3.0" => Ok(Version::HTTP_3),
        _ => Err(ResponseError::builder_with_msg(
            ResponseErrorKind::InvalidRequestResponse,
            format!("invalid http version: {}", version),
        )
        .uri(request.url())
        .build()),
    }
}

fn from_ureq_error(kind: UreqErrorKind, err: AnyError, request: &RequestParts) -> ResponseError {
    let response_error_kind = match kind {
        UreqErrorKind::InvalidUrl => ResponseErrorKind::InvalidUrl,
        UreqErrorKind::UnknownScheme => ResponseErrorKind::InvalidUrl,
        UreqErrorKind::Dns => ResponseErrorKind::DnsServerError,
        UreqErrorKind::ConnectionFailed => ResponseErrorKind::DnsServerError,
        UreqErrorKind::TooManyRedirects => ResponseErrorKind::TooManyRedirect,
        UreqErrorKind::BadStatus => ResponseErrorKind::InvalidRequestResponse,
        UreqErrorKind::BadHeader => ResponseErrorKind::InvalidHeader,
        UreqErrorKind::Io => ResponseErrorKind::LocalIoError,
        UreqErrorKind::InvalidProxyUrl => ResponseErrorKind::ProxyError,
        UreqErrorKind::ProxyConnect => ResponseErrorKind::ProxyError,
        UreqErrorKind::ProxyUnauthorized => ResponseErrorKind::ProxyError,
        UreqErrorKind::HTTP => ResponseErrorKind::InvalidRequestResponse,
        UreqErrorKind::InsecureRequestHttpsOnly => ResponseErrorKind::SslError,
    };
    ResponseError::builder(response_error_kind, err)
        .uri(request.url())
        .build()
}

struct RequestBodyWithCallbacks<'a, 'r> {
    request: &'a mut SyncRequest<'r>,
    have_read: u64,
    user_cancelled_error: &'a mut Option<ResponseError>,
}

impl<'a, 'r> RequestBodyWithCallbacks<'a, 'r> {
    fn new(request: &'a mut SyncRequest<'r>, user_cancelled_error: &'a mut Option<ResponseError>) -> Self {
        Self {
            request,
            user_cancelled_error,
            have_read: 0,
        }
    }
}

impl Read for RequestBodyWithCallbacks<'_, '_> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let n = self.request.body_mut().read(buf)?;
        match n {
            0 => Ok(0),
            n => {
                self.have_read += n as u64;
                let buf = &buf[..n];
                if let Some(on_uploading_progress) = self.request.on_uploading_progress() {
                    on_uploading_progress(TransferProgressInfo::new(
                        self.have_read,
                        self.request.body().size(),
                        buf,
                    ))
                    .map_err(|err| {
                        *self.user_cancelled_error = Some(
                            ResponseError::builder(ResponseErrorKind::CallbackError, err)
                                .uri(self.request.url())
                                .build(),
                        );
                        IoError::new(IoErrorKind::Other, "on_uploading_progress() callback returns error")
                    })?;
                }
                Ok(n)
            }
        }
    }
}
