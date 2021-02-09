use super::super::{
    http::{Method, Request, ResponseError, ResponseErrorKind},
    CurlHTTPCaller,
};
use curl::{
    easy::{Easy2, Handler, HttpVersion, List},
    Error as CurlError, Version,
};
use once_cell::sync::Lazy;
use std::{convert::TryInto, result::Result};
use url::Url;

pub(crate) fn set_method<H: Handler>(
    easy: &mut Easy2<H>,
    request: &Request,
) -> Result<(), ResponseError> {
    match request.method() {
        Method::GET => handle(easy.get(true)),
        Method::HEAD => handle(easy.nobody(true)),
        Method::POST => handle(easy.post(true)),
        Method::PUT => handle(easy.upload(true)),
        method => handle(easy.custom_request(method.as_str())),
    }
}

#[inline]
pub(crate) fn set_url<H: Handler>(
    easy: &mut Easy2<H>,
    request: &Request,
) -> Result<(), ResponseError> {
    handle(easy.url(request.url()))
}

pub(crate) fn set_headers<H: Handler>(
    easy: &mut Easy2<H>,
    request: &Request,
) -> Result<(), ResponseError> {
    let mut header_list = List::new();
    handle(header_list.append("Expect:"))?;
    for (header_name, header_value) in request.headers().iter() {
        let line = header_name.as_ref().to_string() + ": " + header_value;
        handle(header_list.append(&line))?;
    }
    handle(easy.http_headers(header_list))?;
    Ok(())
}

pub(crate) fn set_body<H: Handler>(
    easy: &mut Easy2<H>,
    request: &Request,
) -> Result<(), ResponseError> {
    let size: u64 = request.body().len().try_into().unwrap();
    if size > 0 {
        if request.method() == Method::PUT {
            handle(easy.in_filesize(size))?;
        } else {
            handle(easy.post_field_size(size))?;
        }
    }
    Ok(())
}

pub(crate) fn set_options<H: Handler>(
    easy: &mut Easy2<H>,
    request: &Request,
    http_client: &CurlHTTPCaller,
) -> Result<(), ResponseError> {
    set_preresolved_socket_addrs(easy, request)?;
    handle(easy.useragent(&(request.user_agent() + "/libcurl-" + Version::get().version())))?;
    handle(
        easy.accept_encoding(
            request
                .headers()
                .get(&"Accept-Encoding".into())
                .unwrap_or(&Default::default()),
        ),
    )?;
    handle(easy.http_version(HttpVersion::Any))?;
    handle(easy.show_header(false))?;
    handle(easy.signal(false))?;
    {
        let need_progress = request.on_uploading_progress().is_some()
            || request.on_downloading_progress().is_some();
        handle(easy.progress(need_progress))?;
    }
    if let Some(verify_host) = http_client.verify_host() {
        handle(easy.ssl_verify_host(verify_host))?;
    }
    if let Some(verify_peer) = http_client.verify_peer() {
        handle(easy.ssl_verify_peer(verify_peer))?;
    }
    handle(easy.transfer_encoding(true))?;
    handle(easy.follow_location(request.follow_redirection()))?;
    handle(easy.max_redirections(3))?;
    handle(easy.connect_timeout(request.connect_timeout()))?;
    handle(easy.timeout(request.request_timeout()))?;
    handle(easy.tcp_keepalive(true))?;
    handle(easy.tcp_keepidle(request.tcp_keepalive_idle_timeout()))?;
    handle(easy.tcp_keepintvl(request.tcp_keepalive_probe_interval()))?;
    {
        let (speed, timeout) = request.low_transfer_speed();
        handle(easy.low_speed_limit(speed))?;
        handle(easy.low_speed_time(timeout))?;
    }
    Ok(())
}

static IPV6_SUPPORT: Lazy<bool> = Lazy::new(|| Version::get().feature_ipv6());
static MULTI_IP_ADDRS_SUPPORT: Lazy<bool> =
    Lazy::new(|| Version::get().version_num() >= 0x07_3b_00);

fn set_preresolved_socket_addrs<H: Handler>(
    easy: &mut Easy2<H>,
    request: &Request,
) -> Result<(), ResponseError> {
    if !request.resolved_ip_addrs().is_empty() {
        let url = Url::parse(request.url()).unwrap();
        let mut addr = url.host_str().unwrap().to_owned()
            + ":"
            + &url.port_or_known_default().unwrap().to_string()
            + ":";
        for (i, ip_addr) in request.resolved_ip_addrs().iter().enumerate() {
            if !*IPV6_SUPPORT && ip_addr.is_ipv6() {
                continue;
            }
            if i > 0 {
                addr.push_str(",");
            }
            addr.push_str(&ip_addr.to_string());
            if !*MULTI_IP_ADDRS_SUPPORT {
                break;
            }
        }
        if !addr.ends_with(':') {
            let mut list = List::new();
            handle(list.append(&addr))?;
            handle(easy.resolve(list))?;
        }
    }
    Ok(())
}

pub(crate) fn handle<T>(result: Result<T, CurlError>) -> Result<T, ResponseError> {
    result.map_err(|err| {
        if err.is_unsupported_protocol()
            || err.is_bad_content_encoding()
            || err.is_filesize_exceeded()
            || err.is_http2_error()
            || err.is_http2_stream_error()
        {
            ResponseError::new(ResponseErrorKind::ProtocolError, err)
        } else if err.is_url_malformed() {
            ResponseError::new(ResponseErrorKind::InvalidURLError, err)
        } else if err.is_couldnt_resolve_proxy() || err.is_couldnt_resolve_host() {
            ResponseError::new(ResponseErrorKind::UnknownHostError, err)
        } else if err.is_couldnt_connect() {
            ResponseError::new(ResponseErrorKind::ConnectError, err)
        } else if err.is_send_error() {
            ResponseError::new(ResponseErrorKind::SendError, err)
        } else if err.is_recv_error() {
            ResponseError::new(ResponseErrorKind::ReceiveError, err)
        } else if err.is_read_error() || err.is_write_error() || err.is_send_fail_rewind() {
            ResponseError::new(ResponseErrorKind::LocalIOError, err)
        } else if err.is_aborted_by_callback() {
            ResponseError::new(ResponseErrorKind::UserCanceled, err)
        } else if err.is_operation_timedout() {
            ResponseError::new(ResponseErrorKind::TimeoutError, err)
        } else if err.is_too_many_redirects() {
            ResponseError::new(ResponseErrorKind::TooManyRedirect, err)
        } else if err.is_ssl_connect_error()
            || err.is_peer_failed_verification()
            || err.is_ssl_engine_initfailed()
            || err.is_ssl_engine_notfound()
            || err.is_ssl_engine_setfailed()
            || err.is_ssl_certproblem()
            || err.is_ssl_cipher()
            || err.is_use_ssl_failed()
            || err.is_ssl_cacert()
            || err.is_ssl_cacert_badfile()
            || err.is_ssl_crl_badfile()
            || err.is_ssl_shutdown_failed()
            || err.is_ssl_issuer_error()
        {
            ResponseError::new(ResponseErrorKind::SSLError, err)
        } else {
            ResponseError::new(ResponseErrorKind::UnknownError, err)
        }
    })
}
