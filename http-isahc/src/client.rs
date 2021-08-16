use isahc::{
    config::{Configurable, Dialer},
    error::{Error as IsahcError, ErrorKind as IsahcErrorKind},
    http::{header::USER_AGENT, request::Builder as IsahcRequestBuilder, uri::Scheme},
    Body as IsahcBody, HttpClient as IsahcHttpClient, HttpClientBuilder, Metrics as IsahcMetrics,
    Response as IsahcResponse, ResponseExt,
};
use qiniu_http::{
    HTTPCaller, HeaderValue, Metrics, Request, ResponseError, ResponseErrorKind, SyncResponse,
    SyncResponseResult, UploadProgressInfo, Uri,
};
use std::{
    io::{Cursor, Error as IOError, ErrorKind as IOErrorKind, Read, Result as IOResult},
    mem::{take, transmute},
    net::SocketAddr,
    time::Duration,
};

type IsahcSyncRequest = isahc::Request<IsahcBody>;
type IsahcSyncResponse = isahc::Response<IsahcBody>;

#[cfg(feature = "async")]
use {
    futures::{io::Cursor as AsyncCursor, ready, AsyncRead},
    isahc::AsyncBody as IsahcAsyncBody,
    qiniu_http::{AsyncResponse, AsyncResponseResult},
    std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    },
};

#[cfg(feature = "async")]
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a + Send>>;

#[cfg(feature = "async")]
type IsahcAsyncRequest = isahc::Request<IsahcAsyncBody>;

#[cfg(feature = "async")]
type IsahcAsyncResponse = isahc::Response<IsahcAsyncBody>;

#[derive(Debug, Clone)]
pub struct Client {
    isahc_client: IsahcHttpClient,
}

impl Client {
    #[inline]
    pub(super) fn new(isahc_client: IsahcHttpClient) -> Self {
        Client { isahc_client }
    }

    #[inline]
    pub fn default_client() -> Result<Self, IsahcError> {
        Ok(Self::new(HttpClientBuilder::default().build()?))
    }
}

impl HTTPCaller for Client {
    fn call(&self, request: &Request) -> SyncResponseResult {
        let mut user_cancelled_error: Option<ResponseError> = None;
        let isahc_request = make_sync_isahc_request(request, &mut user_cancelled_error)?;
        match self.isahc_client.send(isahc_request) {
            Ok(isahc_response) => make_sync_response(isahc_response, request),
            Err(err) => user_cancelled_error.map_or_else(|| Err(from_isahc_error(err)), Err),
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_call<'a>(&'a self, request: &'a Request<'_>) -> BoxFuture<'a, AsyncResponseResult> {
        Box::pin(async move {
            let mut user_cancelled_error: Option<ResponseError> = None;
            let reqwest_request = make_async_reqwest_request(request, &mut user_cancelled_error)?;
            match self.isahc_client.send_async(reqwest_request).await {
                Ok(reqwest_response) => make_async_response(reqwest_response, request),
                Err(err) => user_cancelled_error.map_or_else(|| Err(from_isahc_error(err)), Err),
            }
        })
    }

    #[inline]
    fn as_http_caller(&self) -> &dyn HTTPCaller {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[inline]
    fn is_resolved_ip_addrs_supported(&self) -> bool {
        true
    }

    #[inline]
    fn is_response_metrics_supported(&self) -> bool {
        true
    }
}

#[inline]
fn make_user_agent(request: &Request) -> Result<HeaderValue, ResponseError> {
    HeaderValue::from_str(&format!(
        "{}/qiniu-http-{}",
        request.user_agent(),
        isahc::version(),
    ))
    .map_err(|err| ResponseError::new(ResponseErrorKind::InvalidHeader, err))
}

fn make_sync_response(mut response: IsahcSyncResponse, request: &Request) -> SyncResponseResult {
    call_response_callbacks(request, &response)?;

    let mut response_builder = SyncResponse::builder()
        .status_code(response.status())
        .version(response.version())
        .headers(take(response.headers_mut()))
        .extensions(take(response.extensions_mut()));
    if let Some(remote_addr) = response.remote_addr() {
        response_builder = response_builder
            .server_ip(remote_addr.ip())
            .server_port(remote_addr.port());
    }
    if let Some(metrics) = response.metrics() {
        response_builder =
            response_builder.metrics(Box::new(IsahcBasedMetrics(metrics.to_owned())));
    }
    response_builder = response_builder.stream_as_body(Box::new(response.into_body()));
    Ok(response_builder.build())
}

#[cfg(feature = "async")]
fn make_async_response(mut response: IsahcAsyncResponse, request: &Request) -> AsyncResponseResult {
    call_response_callbacks(request, &response)?;

    let mut response_builder = AsyncResponse::builder()
        .status_code(response.status())
        .version(response.version())
        .headers(take(response.headers_mut()))
        .extensions(take(response.extensions_mut()));
    if let Some(remote_addr) = response.remote_addr() {
        response_builder = response_builder
            .server_ip(remote_addr.ip())
            .server_port(remote_addr.port());
    }
    if let Some(metrics) = response.metrics() {
        response_builder =
            response_builder.metrics(Box::new(IsahcBasedMetrics(metrics.to_owned())));
    }
    response_builder = response_builder.stream_as_body(Box::new(response.into_body()));
    Ok(response_builder.build())
}

#[inline]
fn call_response_callbacks<B>(
    request: &Request,
    response: &IsahcResponse<B>,
) -> Result<(), ResponseError> {
    if let Some(on_receive_response_status) = request.on_receive_response_status() {
        if !on_receive_response_status(response.status()) {
            return Err(ResponseError::new(
                ResponseErrorKind::UserCanceled,
                "on_receive_response_status() returns false",
            ));
        }
    }
    if let Some(on_receive_response_header) = request.on_receive_response_header() {
        for (header_name, header_value) in response.headers().iter() {
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

#[derive(Debug)]
struct IsahcBasedMetrics(IsahcMetrics);

impl Metrics for IsahcBasedMetrics {
    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        Some(self.0.total_time())
    }

    #[inline]
    fn name_lookup_duration(&self) -> Option<Duration> {
        Some(self.0.name_lookup_time())
    }

    #[inline]
    fn connect_duration(&self) -> Option<Duration> {
        Some(self.0.connect_time())
    }

    #[inline]
    fn secure_connect_duration(&self) -> Option<Duration> {
        Some(self.0.secure_connect_time())
    }

    #[inline]
    fn redirect_duration(&self) -> Option<Duration> {
        Some(self.0.redirect_time())
    }

    #[inline]
    fn transfer_duration(&self) -> Option<Duration> {
        Some(self.0.transfer_time())
    }
}

fn make_sync_isahc_request(
    request: &Request,
    user_cancelled_error: &mut Option<ResponseError>,
) -> Result<IsahcSyncRequest, ResponseError> {
    let mut isahc_request_builder = isahc::Request::builder()
        .uri(request.url())
        .method(request.method());
    for (header_name, header_value) in request.headers() {
        isahc_request_builder = isahc_request_builder.header(header_name, header_value);
    }
    isahc_request_builder =
        add_extensions_to_isahc_request_builder(request, isahc_request_builder)?;
    isahc_request_builder = isahc_request_builder.header(USER_AGENT, make_user_agent(request)?);

    let isahc_request = isahc_request_builder
        .body(IsahcBody::from_reader_sized(
            RequestBodyWithCallbacks::new(
                request.body(),
                request.on_uploading_progress(),
                user_cancelled_error,
            ),
            request.body().len() as u64,
        ))
        .map_err(|err| ResponseError::new(ResponseErrorKind::InvalidRequestResponse, err))?;
    return Ok(isahc_request);

    type OnProgress<'r> = &'r (dyn Fn(&UploadProgressInfo) -> bool + Send + Sync);

    struct RequestBodyWithCallbacks {
        body: Cursor<&'static [u8]>,
        size: u64,
        on_uploading_progress: Option<OnProgress<'static>>,
        user_cancelled_error: &'static mut Option<ResponseError>,
    }

    impl RequestBodyWithCallbacks {
        fn new(
            body: &[u8],
            on_uploading_progress: Option<OnProgress>,
            user_cancelled_error: &mut Option<ResponseError>,
        ) -> Self {
            Self {
                size: body.len() as u64,
                body: Cursor::new(unsafe { transmute(body) }),
                on_uploading_progress: on_uploading_progress
                    .map(|callback| unsafe { transmute(callback) }),
                user_cancelled_error: unsafe { transmute(user_cancelled_error) },
            }
        }
    }

    impl Read for RequestBodyWithCallbacks {
        fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
            match self.body.read(buf) {
                Err(err) => Err(err),
                Ok(0) => Ok(0),
                Ok(n) => {
                    let buf = &buf[..n];
                    if let Some(on_uploading_progress) = self.on_uploading_progress {
                        if !on_uploading_progress(&UploadProgressInfo::new(
                            self.body.position(),
                            self.size,
                            buf,
                        )) {
                            const ERROR_MESSAGE: &str = "on_uploading_progress() returns false";
                            *self.user_cancelled_error = Some(ResponseError::new(
                                ResponseErrorKind::UserCanceled,
                                ERROR_MESSAGE,
                            ));
                            return Err(IOError::new(IOErrorKind::Other, ERROR_MESSAGE));
                        }
                    }
                    Ok(n)
                }
            }
        }
    }
}

#[cfg(feature = "async")]
fn make_async_reqwest_request(
    request: &Request,
    user_cancelled_error: &mut Option<ResponseError>,
) -> Result<IsahcAsyncRequest, ResponseError> {
    use futures::pin_mut;

    let mut isahc_request_builder = isahc::Request::builder()
        .uri(request.url())
        .method(request.method());
    for (header_name, header_value) in request.headers() {
        isahc_request_builder = isahc_request_builder.header(header_name, header_value);
    }
    isahc_request_builder =
        add_extensions_to_isahc_request_builder(request, isahc_request_builder)?;
    isahc_request_builder = isahc_request_builder.header(USER_AGENT, make_user_agent(request)?);

    let isahc_request = isahc_request_builder
        .body(IsahcAsyncBody::from_reader_sized(
            RequestBodyWithCallbacks::new(
                request.body(),
                request.on_uploading_progress(),
                user_cancelled_error,
            ),
            request.body().len() as u64,
        ))
        .map_err(|err| ResponseError::new(ResponseErrorKind::InvalidRequestResponse, err))?;
    return Ok(isahc_request);

    type OnProgress<'r> = &'r (dyn Fn(&UploadProgressInfo) -> bool + Send + Sync);

    struct RequestBodyWithCallbacks {
        body: AsyncCursor<&'static [u8]>,
        size: u64,
        on_uploading_progress: Option<OnProgress<'static>>,
        user_cancelled_error: &'static mut Option<ResponseError>,
    }

    impl RequestBodyWithCallbacks {
        fn new(
            body: &[u8],
            on_uploading_progress: Option<OnProgress>,
            user_cancelled_error: &mut Option<ResponseError>,
        ) -> Self {
            Self {
                size: body.len() as u64,
                body: AsyncCursor::new(unsafe { transmute(body) }),
                on_uploading_progress: on_uploading_progress
                    .map(|callback| unsafe { transmute(callback) }),
                user_cancelled_error: unsafe { transmute(user_cancelled_error) },
            }
        }
    }

    impl AsyncRead for RequestBodyWithCallbacks {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut [u8],
        ) -> Poll<IOResult<usize>> {
            let body = &mut self.as_mut().body;
            pin_mut!(body);
            match ready!(body.poll_read(cx, buf)) {
                Err(err) => Poll::Ready(Err(err)),
                Ok(0) => Poll::Ready(Ok(0)),
                Ok(n) => {
                    let buf = &buf[..n];
                    if let Some(on_uploading_progress) = self.on_uploading_progress {
                        if !on_uploading_progress(&UploadProgressInfo::new(
                            self.body.position(),
                            self.size,
                            buf,
                        )) {
                            const ERROR_MESSAGE: &str = "on_uploading_progress() returns false";
                            *self.user_cancelled_error = Some(ResponseError::new(
                                ResponseErrorKind::UserCanceled,
                                ERROR_MESSAGE,
                            ));
                            return Poll::Ready(Err(IOError::new(
                                IOErrorKind::Other,
                                ERROR_MESSAGE,
                            )));
                        }
                    }
                    Poll::Ready(Ok(n))
                }
            }
        }
    }
}

#[inline]
fn add_extensions_to_isahc_request_builder(
    request: &Request,
    mut isahc_request_builder: IsahcRequestBuilder,
) -> Result<IsahcRequestBuilder, ResponseError> {
    use super::extensions::*;

    isahc_request_builder = isahc_request_builder.metrics(true);

    if let Some(extension) = request.extensions().get::<TimeoutRequestExtension>() {
        isahc_request_builder = isahc_request_builder.timeout(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<ConnectTimeoutRequestExtension>() {
        isahc_request_builder = isahc_request_builder.connect_timeout(extension.get().to_owned());
    }

    if let Some(extension) = request
        .extensions()
        .get::<LowSpeedTimeoutRequestExtension>()
    {
        isahc_request_builder = isahc_request_builder
            .low_speed_timeout(extension.get().0.to_owned(), extension.get().1.to_owned());
    }

    if let Some(extension) = request
        .extensions()
        .get::<VersionNegotiationRequestExtension>()
    {
        isahc_request_builder =
            isahc_request_builder.version_negotiation(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<RedirectPolicyRequestExtension>() {
        isahc_request_builder = isahc_request_builder.redirect_policy(extension.get().to_owned());
    }

    if let Some(_) = request.extensions().get::<AutoRefererRequestExtension>() {
        isahc_request_builder = isahc_request_builder.auto_referer();
    }

    if let Some(extension) = request
        .extensions()
        .get::<AutomaticDecompressionRequestExtension>()
    {
        isahc_request_builder =
            isahc_request_builder.automatic_decompression(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<TcpKeepaliveRequestExtension>() {
        isahc_request_builder = isahc_request_builder.tcp_keepalive(extension.get().to_owned());
    }

    if let Some(_) = request.extensions().get::<TcpNodelayRequestExtension>() {
        isahc_request_builder = isahc_request_builder.tcp_nodelay();
    }

    if let Some(extension) = request
        .extensions()
        .get::<NetworkInterfaceRequestExtension>()
    {
        isahc_request_builder = isahc_request_builder.interface(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<IpVersionRequestExtension>() {
        isahc_request_builder = isahc_request_builder.ip_version(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<DialRequestExtension>() {
        isahc_request_builder = isahc_request_builder.dial(extension.get().to_owned());
    } else if let Some(&ip) = request.resolved_ip_addrs().and_then(|ips| ips.first()) {
        isahc_request_builder = isahc_request_builder.dial(Dialer::ip_socket(SocketAddr::new(
            ip,
            extract_port_for_uri(request.url())?,
        )));
    }

    if let Some(extension) = request.extensions().get::<ProxyRequestExtension>() {
        isahc_request_builder = isahc_request_builder.proxy(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<ProxyBlacklistRequestExtension>() {
        isahc_request_builder = isahc_request_builder.proxy_blacklist(extension.get().to_owned());
    }

    if let Some(extension) = request
        .extensions()
        .get::<ProxyAuthenticationRequestExtension>()
    {
        isahc_request_builder =
            isahc_request_builder.proxy_authentication(extension.get().to_owned());
    }

    if let Some(extension) = request
        .extensions()
        .get::<ProxyCredentialsRequestExtension>()
    {
        isahc_request_builder = isahc_request_builder.proxy_credentials(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<MaxUploadSpeedRequestExtension>() {
        isahc_request_builder = isahc_request_builder.max_upload_speed(extension.get().to_owned());
    }

    if let Some(extension) = request
        .extensions()
        .get::<MaxDownloadSpeedRequestExtension>()
    {
        isahc_request_builder =
            isahc_request_builder.max_download_speed(extension.get().to_owned());
    }

    if let Some(extension) = request
        .extensions()
        .get::<SslClientCertificateRequestExtension>()
    {
        isahc_request_builder =
            isahc_request_builder.ssl_client_certificate(extension.get().to_owned());
    }

    if let Some(extension) = request
        .extensions()
        .get::<SslCaCertificateRequestExtension>()
    {
        isahc_request_builder =
            isahc_request_builder.ssl_ca_certificate(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<SslCiphersRequestExtension>() {
        isahc_request_builder = isahc_request_builder.ssl_ciphers(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<SslOptionsRequestExtension>() {
        isahc_request_builder = isahc_request_builder.ssl_options(extension.get().to_owned());
    }

    if let Some(extension) = request
        .extensions()
        .get::<TitleCaseHeadersRequestExtension>()
    {
        isahc_request_builder =
            isahc_request_builder.title_case_headers(extension.get().to_owned());
    }

    return Ok(isahc_request_builder);

    #[inline]
    fn extract_port_for_uri(uri: &Uri) -> Result<u16, ResponseError> {
        uri.port_u16().map(Ok).unwrap_or_else(|| {
            if let Some(scheme) = uri.scheme() {
                if scheme == &Scheme::HTTP {
                    Ok(80)
                } else if scheme == &Scheme::HTTPS {
                    Ok(443)
                } else {
                    Err(ResponseError::new(
                        ResponseErrorKind::InvalidURL,
                        "unknown port for url",
                    ))
                }
            } else {
                Err(ResponseError::new(
                    ResponseErrorKind::InvalidURL,
                    "empty scheme for url",
                ))
            }
        })
    }
}

#[inline]
fn from_isahc_error(err: IsahcError) -> ResponseError {
    match err.kind() {
        IsahcErrorKind::BadClientCertificate => {
            ResponseError::new(ResponseErrorKind::SSLError, err)
        }
        IsahcErrorKind::BadServerCertificate => {
            ResponseError::new(ResponseErrorKind::SSLError, err)
        }
        IsahcErrorKind::ClientInitialization => {
            ResponseError::new(ResponseErrorKind::LocalIOError, err)
        }
        IsahcErrorKind::ConnectionFailed => {
            ResponseError::new(ResponseErrorKind::ConnectError, err)
        }
        IsahcErrorKind::InvalidContentEncoding => {
            ResponseError::new(ResponseErrorKind::InvalidHeader, err)
        }
        IsahcErrorKind::InvalidCredentials => {
            ResponseError::new(ResponseErrorKind::InvalidHeader, err)
        }
        IsahcErrorKind::InvalidRequest => {
            ResponseError::new(ResponseErrorKind::InvalidRequestResponse, err)
        }
        IsahcErrorKind::Io => ResponseError::new(ResponseErrorKind::SendError, err),
        IsahcErrorKind::NameResolution => ResponseError::new(ResponseErrorKind::LocalIOError, err),
        IsahcErrorKind::ProtocolViolation => {
            ResponseError::new(ResponseErrorKind::InvalidRequestResponse, err)
        }
        IsahcErrorKind::Timeout => ResponseError::new(ResponseErrorKind::TimeoutError, err),
        IsahcErrorKind::TlsEngine => ResponseError::new(ResponseErrorKind::SSLError, err),
        IsahcErrorKind::TooManyRedirects => {
            ResponseError::new(ResponseErrorKind::TooManyRedirect, err)
        }
        _ => ResponseError::new(ResponseErrorKind::UnknownError, err),
    }
}
