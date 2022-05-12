use anyhow::Error as AnyError;
use isahc::{
    config::{Configurable, Dialer},
    error::{Error as IsahcError, ErrorKind as IsahcErrorKind},
    http::{header::USER_AGENT, request::Builder as IsahcRequestBuilder, uri::Scheme},
    Body as IsahcBody, HttpClient as IsahcHttpClient, Metrics as IsahcMetrics, Response as IsahcResponse, ResponseExt,
};
use qiniu_http::{
    HeaderValue, HttpCaller, Metrics, Request, RequestParts, ResponseError, ResponseErrorKind, SyncRequest,
    SyncResponse, SyncResponseBody, SyncResponseResult, TransferProgressInfo, Uri,
};
use std::{
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult},
    mem::{take, transmute},
    net::{IpAddr, SocketAddr},
    num::NonZeroU16,
};

type IsahcSyncRequest = isahc::Request<IsahcBody>;
type IsahcSyncResponse = isahc::Response<IsahcBody>;

#[cfg(feature = "async")]
use {
    futures::{future::BoxFuture, ready, AsyncRead},
    isahc::AsyncBody as IsahcAsyncBody,
    qiniu_http::{AsyncRequest, AsyncResponse, AsyncResponseBody, AsyncResponseResult},
    std::{
        pin::Pin,
        task::{Context, Poll},
    },
};

#[cfg(feature = "async")]
type IsahcAsyncRequest = isahc::Request<IsahcAsyncBody>;

#[cfg(feature = "async")]
type IsahcAsyncResponse = isahc::Response<IsahcAsyncBody>;

/// Isahc 客户端
#[derive(Debug, Clone)]
pub struct Client {
    isahc_client: IsahcHttpClient,
}

impl Client {
    /// 创建 Isahc 客户端
    #[inline]
    pub fn new(isahc_client: IsahcHttpClient) -> Self {
        Client { isahc_client }
    }

    /// 创建默认的 Isahc 客户端
    #[inline]
    pub fn default_client() -> Result<Self, IsahcError> {
        Ok(Self::new(IsahcHttpClient::new()?))
    }
}

impl From<IsahcHttpClient> for Client {
    #[inline]
    fn from(isahc_client: IsahcHttpClient) -> Self {
        Self::new(isahc_client)
    }
}

impl HttpCaller for Client {
    fn call<'a>(&'a self, request: &'a mut SyncRequest<'_>) -> SyncResponseResult {
        let mut user_cancelled_error: Option<ResponseError> = None;

        let isahc_result = match request.resolved_ip_addrs().map(|ips| ips.to_owned()) {
            Some(ips) if !ips.is_empty() => {
                let mut last_result = None;
                for ip in ips {
                    let isahc_request = match make_sync_isahc_request(request, Some(ip), &mut user_cancelled_error) {
                        Ok(request) => request,
                        Err(err) => {
                            last_result = Some(Err(err));
                            break;
                        }
                    };
                    match self.isahc_client.send(isahc_request) {
                        Ok(isahc_response) => {
                            last_result = Some(Ok(isahc_response));
                            break;
                        }
                        Err(err) => {
                            let should_retry = should_retry(&err);
                            last_result = Some(Err(from_isahc_error(err, request)));
                            if !should_retry {
                                break;
                            }
                        }
                    }
                }
                last_result.unwrap()
            }
            _ => {
                let isahc_request = make_sync_isahc_request(request, None, &mut user_cancelled_error)?;
                self.isahc_client
                    .send(isahc_request)
                    .map_err(|err| from_isahc_error(err, request))
            }
        };

        match isahc_result {
            Ok(isahc_response) => make_sync_response(isahc_response, request),
            Err(err) => user_cancelled_error.map_or(Err(err), Err),
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult> {
        Box::pin(async move {
            let mut user_cancelled_error: Option<ResponseError> = None;

            let isahc_result = match request.resolved_ip_addrs().map(|ips| ips.to_owned()) {
                Some(ips) if !ips.is_empty() => {
                    let mut last_result = None;
                    for ip in ips {
                        let isahc_request = match make_async_isahc_request(request, Some(ip), &mut user_cancelled_error)
                        {
                            Ok(request) => request,
                            Err(err) => {
                                last_result = Some(Err(err));
                                break;
                            }
                        };
                        match self.isahc_client.send_async(isahc_request).await {
                            Ok(isahc_response) => {
                                last_result = Some(Ok(isahc_response));
                                break;
                            }
                            Err(err) => {
                                let should_retry = should_retry(&err);
                                last_result = Some(Err(from_isahc_error(err, request)));
                                if !should_retry {
                                    break;
                                }
                            }
                        }
                    }
                    last_result.unwrap()
                }
                _ => {
                    let isahc_request = make_async_isahc_request(request, None, &mut user_cancelled_error)?;
                    self.isahc_client
                        .send_async(isahc_request)
                        .await
                        .map_err(|err| from_isahc_error(err, request))
                }
            };

            match isahc_result {
                Ok(isahc_response) => make_async_response(isahc_response, request),
                Err(err) => user_cancelled_error.map_or(Err(err), Err),
            }
        })
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

fn make_user_agent(request: &RequestParts) -> Result<HeaderValue, ResponseError> {
    HeaderValue::from_str(&format!("{}/qiniu-{}", request.user_agent(), isahc::version(),)).map_err(|err| {
        ResponseError::builder(ResponseErrorKind::InvalidHeader, err)
            .uri(request.url())
            .build()
    })
}

fn make_sync_response(mut response: IsahcSyncResponse, request: &mut SyncRequest) -> SyncResponseResult {
    call_response_callbacks(request, &response)?;

    let mut response_builder = SyncResponse::builder();
    response_builder
        .status_code(response.status())
        .version(response.version())
        .headers(take(response.headers_mut()))
        .extensions(take(request.extensions_mut()));
    if let Some(remote_addr) = response.remote_addr() {
        response_builder.server_ip(remote_addr.ip());
        if let Some(port) = NonZeroU16::new(remote_addr.port()) {
            response_builder.server_port(port);
        }
    }
    if let Some(metrics) = response.metrics() {
        response_builder.metrics(make_metrics_from_isahc(metrics));
    }
    response_builder.body(SyncResponseBody::from_reader(response.into_body()));
    Ok(response_builder.build())
}

#[cfg(feature = "async")]
fn make_async_response(mut response: IsahcAsyncResponse, request: &mut AsyncRequest) -> AsyncResponseResult {
    call_response_callbacks(request, &response)?;

    let mut response_builder = AsyncResponse::builder();
    response_builder
        .status_code(response.status())
        .version(response.version())
        .headers(take(response.headers_mut()))
        .extensions(take(request.extensions_mut()));
    if let Some(remote_addr) = response.remote_addr() {
        response_builder.server_ip(remote_addr.ip());
        if let Some(port) = NonZeroU16::new(remote_addr.port()) {
            response_builder.server_port(port);
        }
    }
    if let Some(metrics) = response.metrics() {
        response_builder.metrics(make_metrics_from_isahc(metrics));
    }
    response_builder.body(AsyncResponseBody::from_reader(response.into_body()));
    Ok(response_builder.build())
}

fn call_response_callbacks<ReqBody, RespBody>(
    request: &Request<ReqBody>,
    response: &IsahcResponse<RespBody>,
) -> Result<(), ResponseError> {
    if let Some(on_receive_response_status) = request.on_receive_response_status() {
        on_receive_response_status(response.status()).map_err(|err| make_callback_error(err, request))?;
    }
    if let Some(on_receive_response_header) = request.on_receive_response_header() {
        response.headers().iter().try_for_each(|(header_name, header_value)| {
            on_receive_response_header(header_name, header_value).map_err(|err| make_callback_error(err, request))
        })?;
    }
    Ok(())
}

fn make_callback_error(err: AnyError, request: &RequestParts) -> ResponseError {
    ResponseError::builder(ResponseErrorKind::CallbackError, err)
        .uri(request.url())
        .build()
}

fn should_retry(err: &IsahcError) -> bool {
    err.kind() == IsahcErrorKind::ConnectionFailed
}

fn make_metrics_from_isahc(metrics: &IsahcMetrics) -> Metrics {
    Metrics::builder()
        .total_duration(metrics.total_time())
        .name_lookup_duration(metrics.name_lookup_time())
        .connect_duration(metrics.connect_time())
        .secure_connect_duration(metrics.redirect_time())
        .redirect_duration(metrics.redirect_time())
        .transfer_duration(metrics.transfer_time())
        .build()
}

fn make_sync_isahc_request(
    request: &mut SyncRequest,
    ip_addr: Option<IpAddr>,
    user_cancelled_error: &mut Option<ResponseError>,
) -> Result<IsahcSyncRequest, ResponseError> {
    let mut isahc_request_builder = isahc::Request::builder().uri(request.url()).method(request.method());
    for (header_name, header_value) in request.headers() {
        isahc_request_builder = isahc_request_builder.header(header_name, header_value);
    }
    isahc_request_builder = add_extensions_to_isahc_request_builder(request, ip_addr, isahc_request_builder)?;

    isahc_request_builder = isahc_request_builder.header(USER_AGENT, make_user_agent(request)?);

    let isahc_request = isahc_request_builder
        .body(IsahcBody::from_reader_sized(
            RequestBodyWithCallbacks::new(request, user_cancelled_error),
            request.body().size(),
        ))
        .map_err(|err| {
            ResponseError::builder(ResponseErrorKind::InvalidRequestResponse, err)
                .uri(request.url())
                .build()
        })?;
    return Ok(isahc_request);

    struct RequestBodyWithCallbacks {
        request: &'static mut SyncRequest<'static>,
        user_cancelled_error: &'static mut Option<ResponseError>,
        have_read: u64,
    }

    impl RequestBodyWithCallbacks {
        fn new(request: &mut SyncRequest, user_cancelled_error: &mut Option<ResponseError>) -> Self {
            #[allow(unsafe_code)]
            Self {
                have_read: 0,
                request: unsafe { transmute(request) },
                user_cancelled_error: unsafe { transmute(user_cancelled_error) },
            }
        }
    }

    impl Read for RequestBodyWithCallbacks {
        fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
            let n = self.request.body_mut().read(buf)?;
            match n {
                0 => Ok(0),
                n => {
                    let buf = &buf[..n];
                    self.have_read += n as u64;
                    if let Some(on_uploading_progress) = self.request.on_uploading_progress() {
                        on_uploading_progress(TransferProgressInfo::new(
                            self.have_read,
                            self.request.body().size(),
                            buf,
                        ))
                        .map_err(|err| {
                            *self.user_cancelled_error = Some(make_callback_error(err, self.request));
                            IoError::new(IoErrorKind::Other, "on_uploading_progress() callback returns error")
                        })?;
                    }
                    Ok(n)
                }
            }
        }
    }
}

#[cfg(feature = "async")]
fn make_async_isahc_request(
    request: &mut AsyncRequest,
    ip_addr: Option<IpAddr>,
    user_cancelled_error: &mut Option<ResponseError>,
) -> Result<IsahcAsyncRequest, ResponseError> {
    use futures::pin_mut;

    let mut isahc_request_builder = isahc::Request::builder().uri(request.url()).method(request.method());
    for (header_name, header_value) in request.headers() {
        isahc_request_builder = isahc_request_builder.header(header_name, header_value);
    }
    isahc_request_builder = add_extensions_to_isahc_request_builder(request, ip_addr, isahc_request_builder)?;
    isahc_request_builder = isahc_request_builder.header(USER_AGENT, make_user_agent(request)?);

    let isahc_request = isahc_request_builder
        .body(IsahcAsyncBody::from_reader_sized(
            RequestBodyWithCallbacks::new(request, user_cancelled_error),
            request.body().size(),
        ))
        .map_err(|err| {
            ResponseError::builder(ResponseErrorKind::InvalidRequestResponse, err)
                .uri(request.url())
                .build()
        })?;
    return Ok(isahc_request);

    struct RequestBodyWithCallbacks {
        request: &'static mut AsyncRequest<'static>,
        user_cancelled_error: &'static mut Option<ResponseError>,
        have_read: u64,
    }

    impl RequestBodyWithCallbacks {
        fn new(request: &mut AsyncRequest, user_cancelled_error: &mut Option<ResponseError>) -> Self {
            #[allow(unsafe_code)]
            Self {
                have_read: 0,
                request: unsafe { transmute(request) },
                user_cancelled_error: unsafe { transmute(user_cancelled_error) },
            }
        }
    }

    impl AsyncRead for RequestBodyWithCallbacks {
        fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<IoResult<usize>> {
            let request_mut = &mut self.as_mut().request;
            let body = request_mut.body_mut();
            pin_mut!(body);
            match ready!(body.poll_read(cx, buf)) {
                Err(err) => Poll::Ready(Err(err)),
                Ok(0) => Poll::Ready(Ok(0)),
                Ok(n) => {
                    let buf = &buf[..n];
                    self.as_mut().have_read += n as u64;
                    if let Some(on_uploading_progress) = self.as_ref().request.on_uploading_progress() {
                        if let Err(err) = on_uploading_progress(TransferProgressInfo::new(
                            self.as_ref().have_read,
                            self.as_ref().request.body().size(),
                            buf,
                        )) {
                            *self.user_cancelled_error = Some(make_callback_error(err, self.request));
                            return Poll::Ready(Err(IoError::new(
                                IoErrorKind::Other,
                                "on_uploading_progress() callback returns error",
                            )));
                        }
                    }
                    Poll::Ready(Ok(n))
                }
            }
        }
    }
}

fn add_extensions_to_isahc_request_builder(
    request: &RequestParts,
    ip_addr: Option<IpAddr>,
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

    if let Some(extension) = request.extensions().get::<LowSpeedTimeoutRequestExtension>() {
        isahc_request_builder =
            isahc_request_builder.low_speed_timeout(extension.get().0.to_owned(), extension.get().1.to_owned());
    }

    if let Some(extension) = request.extensions().get::<VersionNegotiationRequestExtension>() {
        isahc_request_builder = isahc_request_builder.version_negotiation(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<RedirectPolicyRequestExtension>() {
        isahc_request_builder = isahc_request_builder.redirect_policy(extension.get().to_owned());
    }

    if request.extensions().get::<AutoRefererRequestExtension>().is_some() {
        isahc_request_builder = isahc_request_builder.auto_referer();
    }

    if let Some(extension) = request.extensions().get::<AutomaticDecompressionRequestExtension>() {
        isahc_request_builder = isahc_request_builder.automatic_decompression(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<TcpKeepaliveRequestExtension>() {
        isahc_request_builder = isahc_request_builder.tcp_keepalive(extension.get().to_owned());
    }

    if request.extensions().get::<TcpNodelayRequestExtension>().is_some() {
        isahc_request_builder = isahc_request_builder.tcp_nodelay();
    }

    if let Some(extension) = request.extensions().get::<NetworkInterfaceRequestExtension>() {
        isahc_request_builder = isahc_request_builder.interface(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<IpVersionRequestExtension>() {
        isahc_request_builder = isahc_request_builder.ip_version(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<DialRequestExtension>() {
        isahc_request_builder = isahc_request_builder.dial(extension.get().to_owned());
    } else if let Some(ip_addr) = ip_addr {
        isahc_request_builder = isahc_request_builder.dial(Dialer::ip_socket(SocketAddr::new(
            ip_addr,
            extract_port_for_uri(request.url())?,
        )));
    }

    if let Some(extension) = request.extensions().get::<ProxyRequestExtension>() {
        isahc_request_builder = isahc_request_builder.proxy(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<ProxyBlacklistRequestExtension>() {
        isahc_request_builder = isahc_request_builder.proxy_blacklist(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<ProxyAuthenticationRequestExtension>() {
        isahc_request_builder = isahc_request_builder.proxy_authentication(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<ProxyCredentialsRequestExtension>() {
        isahc_request_builder = isahc_request_builder.proxy_credentials(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<MaxUploadSpeedRequestExtension>() {
        isahc_request_builder = isahc_request_builder.max_upload_speed(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<MaxDownloadSpeedRequestExtension>() {
        isahc_request_builder = isahc_request_builder.max_download_speed(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<SslClientCertificateRequestExtension>() {
        isahc_request_builder = isahc_request_builder.ssl_client_certificate(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<SslCaCertificateRequestExtension>() {
        isahc_request_builder = isahc_request_builder.ssl_ca_certificate(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<SslCiphersRequestExtension>() {
        isahc_request_builder = isahc_request_builder.ssl_ciphers(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<SslOptionsRequestExtension>() {
        isahc_request_builder = isahc_request_builder.ssl_options(extension.get().to_owned());
    }

    if let Some(extension) = request.extensions().get::<TitleCaseHeadersRequestExtension>() {
        isahc_request_builder = isahc_request_builder.title_case_headers(extension.get().to_owned());
    }

    return Ok(isahc_request_builder);

    fn extract_port_for_uri(uri: &Uri) -> Result<u16, ResponseError> {
        const INVALID_URL: ResponseErrorKind = ResponseErrorKind::InvalidUrl;
        uri.port_u16().map(Ok).unwrap_or_else(|| {
            if let Some(scheme) = uri.scheme() {
                if scheme == &Scheme::HTTP {
                    Ok(80)
                } else if scheme == &Scheme::HTTPS {
                    Ok(443)
                } else {
                    Err(ResponseError::builder_with_msg(INVALID_URL, "unknown port for url").build())
                }
            } else {
                Err(ResponseError::builder_with_msg(INVALID_URL, "empty scheme for url").build())
            }
        })
    }
}

fn from_isahc_error(err: IsahcError, request: &RequestParts) -> ResponseError {
    let error_builder = match err.kind() {
        IsahcErrorKind::BadClientCertificate => ResponseError::builder(ResponseErrorKind::ClientCertError, err),
        IsahcErrorKind::BadServerCertificate => ResponseError::builder(ResponseErrorKind::ServerCertError, err),
        IsahcErrorKind::ClientInitialization => ResponseError::builder(ResponseErrorKind::LocalIoError, err),
        IsahcErrorKind::ConnectionFailed => ResponseError::builder(ResponseErrorKind::ConnectError, err),
        IsahcErrorKind::InvalidContentEncoding => ResponseError::builder(ResponseErrorKind::InvalidHeader, err),
        IsahcErrorKind::InvalidCredentials => ResponseError::builder(ResponseErrorKind::InvalidHeader, err),
        IsahcErrorKind::InvalidRequest => ResponseError::builder(ResponseErrorKind::InvalidRequestResponse, err),
        IsahcErrorKind::Io => ResponseError::builder(ResponseErrorKind::SendError, err),
        IsahcErrorKind::NameResolution => ResponseError::builder(ResponseErrorKind::UnknownHostError, err),
        IsahcErrorKind::ProtocolViolation => ResponseError::builder(ResponseErrorKind::InvalidRequestResponse, err),
        IsahcErrorKind::Timeout => ResponseError::builder(ResponseErrorKind::TimeoutError, err),
        IsahcErrorKind::TlsEngine => ResponseError::builder(ResponseErrorKind::SslError, err),
        IsahcErrorKind::TooManyRedirects => ResponseError::builder(ResponseErrorKind::TooManyRedirect, err),
        _ => ResponseError::builder(ResponseErrorKind::UnknownError, err),
    };
    error_builder.uri(request.url()).build()
}
