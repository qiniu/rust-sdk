use qiniu_http::{
    header::{CONTENT_LENGTH, USER_AGENT},
    HTTPCaller, HeaderName, HeaderValue, Request, ResponseError, ResponseErrorKind, StatusCode,
    SyncResponse, SyncResponseResult, TransferProgressInfo, Uri, Version,
};
use std::{
    error::Error,
    fmt,
    io::{Cursor, Error as IOError, ErrorKind as IOErrorKind, Read, Result as IOResult},
};
use ureq::{
    Agent, Error as UreqError, ErrorKind as UreqErrorKind, Request as UreqRequest,
    Response as UreqResponse,
};

#[cfg(feature = "async")]
use {super::BoxFuture, qiniu_http::AsyncResponseResult};

#[derive(Debug, Clone)]
pub struct Client {
    client: Agent,
}

impl Client {
    #[inline]
    pub fn new(client: Agent) -> Self {
        Self { client }
    }
}

impl Default for Client {
    #[inline]
    fn default() -> Self {
        Self {
            client: ureq::agent(),
        }
    }
}

impl HTTPCaller for Client {
    fn call(&self, request: &Request) -> SyncResponseResult {
        let mut user_cancelled_error: Option<ResponseError> = None;

        let ureq_request = make_ureq_request(&self.client, request)?;
        match ureq_request.send(RequestBodyWithCallbacks::new(
            request.url(),
            request.body(),
            request.on_uploading_progress(),
            &mut user_cancelled_error,
        )) {
            Ok(response) => make_ureq_sync_response(response, request),
            Err(err) => {
                let kind = err.kind();
                match err {
                    UreqError::Status(_, response) => make_ureq_sync_response(response, request),
                    UreqError::Transport(transport) => user_cancelled_error
                        .map_or_else(|| Err(from_ureq_error(kind, transport, request)), Err),
                }
            }
        }
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_call<'a>(&'a self, _request: &'a Request<'_>) -> BoxFuture<'a, AsyncResponseResult> {
        unimplemented!("http_ureq::Client does not support async call")
    }
}

#[inline]
fn make_user_agent(request: &Request) -> Result<HeaderValue, ResponseError> {
    let user_agent = format!("{}/qiniu-http-ureq", request.user_agent());
    HeaderValue::from_str(&user_agent)
        .map_err(|err| build_header_value_error(request, &user_agent, err))
}

fn make_ureq_request(agent: &Agent, request: &Request) -> Result<UreqRequest, ResponseError> {
    let mut request_builder = agent.request(request.method().as_str(), &request.url().to_string());
    for (header_name, header_value) in request.headers() {
        request_builder =
            set_header_for_request_builder(request_builder, request, header_name, header_value)?;
    }
    request_builder = set_header_for_request_builder(
        request_builder,
        request,
        &USER_AGENT,
        &make_user_agent(request)?,
    )?;
    request_builder =
        request_builder.set(CONTENT_LENGTH.as_str(), &request.body().len().to_string());
    request_builder = add_extensions_to_request_builder(request, request_builder);
    Ok(request_builder)
}

fn make_ureq_sync_response(response: UreqResponse, request: &Request) -> SyncResponseResult {
    call_response_callbacks(request, &response)?;

    let mut response_builder = SyncResponse::builder()
        .status_code(status_code_of_response(&response, request)?)
        .version(parse_http_version(response.http_version(), request)?);
    for header_name_str in response.headers_names().into_iter() {
        if let Some(header_value_str) = response.header(&header_name_str) {
            let header_name = HeaderName::from_bytes(header_name_str.as_bytes())
                .map_err(|err| build_header_name_error(request, &header_name_str, err))?;
            let header_value = HeaderValue::from_bytes(header_value_str.as_bytes())
                .map_err(|err| build_header_value_error(request, header_value_str, err))?;
            response_builder = response_builder.header(header_name, header_value);
        }
    }
    response_builder =
        response_builder.stream_as_body(Box::new(ResponseReaderWrapper(response.into_reader())));
    return Ok(response_builder.build());

    struct ResponseReaderWrapper<R: Read + Send>(R);

    impl<R: Read + Send> Read for ResponseReaderWrapper<R> {
        #[inline]
        fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
            self.0.read(buf)
        }
    }

    impl<R: Read + Send> fmt::Debug for ResponseReaderWrapper<R> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_tuple("ResponseReaderWrapper").finish()
        }
    }
}

fn add_extensions_to_request_builder(
    request: &Request,
    mut request_builder: UreqRequest,
) -> UreqRequest {
    use super::extensions::TimeoutExtension;

    if let Some(extension) = request.extensions().get::<TimeoutExtension>() {
        request_builder = request_builder.timeout(extension.get());
    }

    request_builder
}

fn call_response_callbacks(
    request: &Request,
    response: &UreqResponse,
) -> Result<(), ResponseError> {
    if let Some(on_receive_response_status) = request.on_receive_response_status() {
        if !on_receive_response_status(status_code_of_response(response, request)?) {
            return Err(build_on_receive_response_status_error(request));
        }
    }
    if let Some(on_receive_response_header) = request.on_receive_response_header() {
        for header_name_str in response.headers_names().into_iter() {
            if let Some(header_value_str) = response.header(&header_name_str) {
                let header_name = HeaderName::from_bytes(header_name_str.as_bytes())
                    .map_err(|err| build_header_name_error(request, &header_name_str, err))?;
                let header_value = HeaderValue::from_bytes(header_value_str.as_bytes())
                    .map_err(|err| build_header_value_error(request, header_value_str, err))?;
                if !on_receive_response_header(&header_name, &header_value) {
                    return Err(build_on_receive_response_header_error(request));
                }
            }
        }
    }
    Ok(())
}

#[inline]
fn build_on_receive_response_status_error(request: &Request) -> ResponseError {
    ResponseError::builder(
        ResponseErrorKind::UserCanceled,
        "on_receive_response_status() returns false",
    )
    .uri(request.url())
    .build()
}

#[inline]
fn build_on_receive_response_header_error(request: &Request) -> ResponseError {
    ResponseError::builder(
        ResponseErrorKind::UserCanceled,
        "on_receive_response_header() returns false",
    )
    .uri(request.url())
    .build()
}

#[inline]
fn build_status_code_error(request: &Request, code: u16, err: impl Error) -> ResponseError {
    ResponseError::builder(
        ResponseErrorKind::InvalidRequestResponse,
        format!("invalid status code({}): {}", code, err),
    )
    .uri(request.url())
    .build()
}

#[inline]
fn build_header_name_error(request: &Request, header_name: &str, err: impl Error) -> ResponseError {
    ResponseError::builder(
        ResponseErrorKind::InvalidHeader,
        format!("invalid header name({}): {}", header_name, err),
    )
    .uri(request.url())
    .build()
}

#[inline]
fn build_header_value_error(
    request: &Request,
    header_value: &str,
    err: impl Error,
) -> ResponseError {
    ResponseError::builder(
        ResponseErrorKind::InvalidHeader,
        format!("invalid header value({}): {}", header_value, err),
    )
    .uri(request.url())
    .build()
}

#[inline]
fn convert_header_value_error(
    request: &Request,
    header_value: &HeaderValue,
    err: impl Error,
) -> ResponseError {
    ResponseError::builder(
        ResponseErrorKind::InvalidHeader,
        format!("invalid header value({:?}): {}", header_value, err),
    )
    .uri(request.url())
    .build()
}

#[inline]
fn set_header_for_request_builder(
    request_builder: UreqRequest,
    request: &Request,
    header_name: &HeaderName,
    header_value: &HeaderValue,
) -> Result<UreqRequest, ResponseError> {
    Ok(request_builder.set(
        header_name.as_str(),
        header_value
            .to_str()
            .map_err(|err| convert_header_value_error(request, header_value, err))?,
    ))
}

#[inline]
fn status_code_of_response(
    response: &UreqResponse,
    request: &Request,
) -> Result<StatusCode, ResponseError> {
    StatusCode::from_u16(response.status())
        .map_err(|err| build_status_code_error(request, response.status(), err))
}

#[inline]
fn parse_http_version(version: &str, request: &Request) -> Result<Version, ResponseError> {
    match version {
        "HTTP/0.9" => Ok(Version::HTTP_09),
        "HTTP/1.0" => Ok(Version::HTTP_10),
        "HTTP/1.1" => Ok(Version::HTTP_11),
        "HTTP/2.0" => Ok(Version::HTTP_2),
        "HTTP/3.0" => Ok(Version::HTTP_3),
        _ => Err(ResponseError::builder(
            ResponseErrorKind::InvalidRequestResponse,
            format!("invalid http version: {}", version),
        )
        .uri(request.url())
        .build()),
    }
}

#[inline]
fn from_ureq_error(
    kind: UreqErrorKind,
    err: impl Error + Send + Sync + 'static,
    request: &Request,
) -> ResponseError {
    let response_error_kind = match kind {
        UreqErrorKind::InvalidUrl => ResponseErrorKind::InvalidURL,
        UreqErrorKind::UnknownScheme => ResponseErrorKind::InvalidURL,
        UreqErrorKind::Dns => ResponseErrorKind::DNSServerError,
        UreqErrorKind::ConnectionFailed => ResponseErrorKind::DNSServerError,
        UreqErrorKind::TooManyRedirects => ResponseErrorKind::TooManyRedirect,
        UreqErrorKind::BadStatus => ResponseErrorKind::InvalidRequestResponse,
        UreqErrorKind::BadHeader => ResponseErrorKind::InvalidHeader,
        UreqErrorKind::Io => ResponseErrorKind::LocalIOError,
        UreqErrorKind::InvalidProxyUrl => ResponseErrorKind::ProxyError,
        UreqErrorKind::ProxyConnect => ResponseErrorKind::ProxyError,
        UreqErrorKind::ProxyUnauthorized => ResponseErrorKind::ProxyError,
        UreqErrorKind::HTTP => ResponseErrorKind::InvalidRequestResponse,
    };
    ResponseError::builder(response_error_kind, err)
        .uri(request.url())
        .build()
}

type OnProgress<'r> = &'r (dyn Fn(&TransferProgressInfo) -> bool + Send + Sync);

struct RequestBodyWithCallbacks<'r> {
    request_uri: &'r Uri,
    body: Cursor<&'r [u8]>,
    size: u64,
    on_uploading_progress: Option<OnProgress<'r>>,
    user_cancelled_error: &'r mut Option<ResponseError>,
}

impl<'r> RequestBodyWithCallbacks<'r> {
    fn new(
        request_uri: &'r Uri,
        body: &'r [u8],
        on_uploading_progress: Option<OnProgress<'r>>,
        user_cancelled_error: &'r mut Option<ResponseError>,
    ) -> Self {
        Self {
            size: body.len() as u64,
            body: Cursor::new(body),
            on_uploading_progress,
            user_cancelled_error,
            request_uri,
        }
    }
}

impl Read for RequestBodyWithCallbacks<'_> {
    fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
        match self.body.read(buf) {
            Err(err) => Err(err),
            Ok(0) => Ok(0),
            Ok(n) => {
                let buf = &buf[..n];
                if let Some(on_uploading_progress) = self.on_uploading_progress {
                    if !on_uploading_progress(&TransferProgressInfo::new(
                        self.body.position(),
                        self.size,
                        buf,
                    )) {
                        const ERROR_MESSAGE: &str = "on_uploading_progress() returns false";
                        *self.user_cancelled_error = Some(
                            ResponseError::builder(ResponseErrorKind::UserCanceled, ERROR_MESSAGE)
                                .uri(self.request_uri)
                                .build(),
                        );
                        return Err(IOError::new(IOErrorKind::Other, ERROR_MESSAGE));
                    }
                }
                Ok(n)
            }
        }
    }
}
