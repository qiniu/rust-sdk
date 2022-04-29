use super::{
    super::{
        super::{DomainWithPort, Endpoint, IpAddrWithPort},
        ApiResult, Authorization, AuthorizationError, AuthorizationProvider, CallbackContextImpl,
        ExtendedCallbackContextImpl, InnerRequestParts, ResolveAnswers, ResolveOptions, ResolveResult, ResponseError,
        ResponseErrorKind, RetriedStatsInfo, RetryDecision, SimplifiedCallbackContext, SyncResponse,
    },
    domain_or_ip_addr::DomainOrIpAddr,
    error::{ErrorResponseBody, TryError},
};
use anyhow::Error as AnyError;
use mime::APPLICATION_WWW_FORM_URLENCODED;
use qiniu_http::{
    header::CONTENT_TYPE,
    uri::{Authority, InvalidUri, PathAndQuery, Scheme, Uri},
    Extensions, HeaderValue, Request as HttpRequest, RequestParts as HttpRequestParts, Reset,
    ResponseErrorKind as HttpResponseErrorKind, ResponseParts, SyncRequest as SyncHttpRequest, SyncRequestBody,
};
use std::{borrow::Cow, io::Error as IoError, net::IpAddr, time::Duration};
use url::ParseError as UrlParseError;

#[cfg(feature = "async")]
use {
    super::super::AsyncResponse,
    qiniu_http::{AsyncRequestBody, AsyncReset},
};

pub(super) fn make_request<'r, B: Default + 'r>(
    url: Uri,
    request: &'r InnerRequestParts<'r>,
    body: B,
    extensions: Extensions,
    resolved_ips: &'r [IpAddr],
) -> HttpRequest<'r, B> {
    let mut headers = request.headers().to_owned();
    headers
        .entry(CONTENT_TYPE)
        .or_insert(HeaderValue::from_str(APPLICATION_WWW_FORM_URLENCODED.as_ref()).unwrap());
    HttpRequest::builder()
        .url(url)
        .method(request.method().to_owned())
        .version(request.version())
        .headers(headers)
        .body(body)
        .appended_user_agent(request.appended_user_agent().to_owned())
        .resolved_ip_addrs(resolved_ips)
        .extensions(extensions)
        .build()
}

pub(super) fn extract_ips_from(
    domain_or_ip: &DomainOrIpAddr,
) -> (Cow<'_, [IpAddrWithPort]>, Option<&'_ DomainWithPort>) {
    match domain_or_ip {
        DomainOrIpAddr::Domain {
            domain_with_port,
            resolved_ips,
        } => (Cow::Borrowed(resolved_ips), Some(domain_with_port)),
        &DomainOrIpAddr::IpAddr(ip_addr) => (Cow::Owned(vec![ip_addr]), None),
    }
}

pub(super) fn make_url(
    domain_or_ip: &DomainOrIpAddr,
    request: &InnerRequestParts<'_>,
    retried: &RetriedStatsInfo,
) -> Result<(Uri, Vec<IpAddr>), TryError> {
    return _make_url(domain_or_ip, request).map_err(|err| {
        TryError::new(
            ResponseError::new(HttpResponseErrorKind::InvalidUrl.into(), err).retried(retried),
            RetryDecision::TryNextServer.into(),
        )
    });

    fn _make_url(
        domain_or_ip: &DomainOrIpAddr,
        request: &InnerRequestParts<'_>,
    ) -> Result<(Uri, Vec<IpAddr>), InvalidUri> {
        let mut resolved_ip_addrs = Vec::new();
        let scheme = if request.use_https() {
            Scheme::HTTPS
        } else {
            Scheme::HTTP
        };

        let authority: Authority = match domain_or_ip {
            DomainOrIpAddr::Domain {
                domain_with_port,
                resolved_ips,
            } => {
                resolved_ip_addrs = resolved_ips.iter().map(|resolved| resolved.ip_addr()).collect();
                let mut authority = domain_with_port.domain().to_owned();
                if let Some(port) = domain_with_port.port() {
                    authority.push(':');
                    authority.push_str(&port.get().to_string());
                }
                authority.parse()?
            }
            DomainOrIpAddr::IpAddr(ip_addr_with_port) => ip_addr_with_port.to_string().parse()?,
        };
        let mut path_and_query = if request.path().starts_with('/') {
            request.path().to_owned()
        } else {
            "/".to_owned() + request.path()
        };
        if !request.query().is_empty() || !request.query_pairs().is_empty() {
            path_and_query.push('?');
            let path_len = path_and_query.len();
            if !request.query().is_empty() {
                path_and_query.push_str(request.query());
            }
            let mut serializer = form_urlencoded::Serializer::for_suffix(&mut path_and_query, path_len);
            serializer.extend_pairs(request.query_pairs().iter());
            serializer.finish();
        }
        let path_and_query: PathAndQuery = path_and_query.parse()?;

        let url = Uri::builder()
            .scheme(scheme)
            .authority(authority)
            .path_and_query(path_and_query)
            .build()
            .unwrap();
        Ok((url, resolved_ip_addrs))
    }
}

pub(super) fn reset_request_body(body: &mut SyncRequestBody<'_>, retried: &RetriedStatsInfo) -> Result<(), TryError> {
    body.reset().map_err(|err| {
        TryError::new(
            ResponseError::from(err).retried(retried),
            RetryDecision::DontRetry.into(),
        )
    })
}

#[cfg(feature = "async")]
pub(super) async fn reset_async_request_body(
    body: &mut AsyncRequestBody<'_>,
    retried: &RetriedStatsInfo,
) -> Result<(), TryError> {
    body.reset().await.map_err(|err| {
        TryError::new(
            ResponseError::from(err).retried(retried),
            RetryDecision::DontRetry.into(),
        )
    })
}

pub(super) fn call_before_backoff_callbacks(
    request: &InnerRequestParts<'_>,
    built: &mut HttpRequestParts<'_>,
    retried: &RetriedStatsInfo,
    delay: Duration,
) -> Result<(), TryError> {
    request
        .call_before_backoff_callbacks(&mut ExtendedCallbackContextImpl::new(request, built, retried), delay)
        .map_err(|err| make_callback_try_error(err, retried))
}

pub(super) fn call_after_backoff_callbacks(
    request: &InnerRequestParts<'_>,
    built: &mut HttpRequestParts<'_>,
    retried: &RetriedStatsInfo,
    delay: Duration,
) -> Result<(), TryError> {
    request
        .call_after_backoff_callbacks(&mut ExtendedCallbackContextImpl::new(request, built, retried), delay)
        .map_err(|err| make_callback_try_error(err, retried))
}

fn call_to_resolve_domain_callbacks(
    request: &InnerRequestParts<'_>,
    domain: &str,
    extensions: &mut Extensions,
    retried: &RetriedStatsInfo,
) -> Result<(), TryError> {
    let mut context = CallbackContextImpl::new(request, extensions);
    request
        .call_to_resolve_domain_callbacks(&mut context, domain)
        .map_err(|err| make_callback_try_error(err, retried))
}

fn call_domain_resolved_callbacks(
    request: &InnerRequestParts<'_>,
    domain: &str,
    answers: &ResolveAnswers,
    extensions: &mut Extensions,
    retried: &RetriedStatsInfo,
) -> Result<(), TryError> {
    let mut context = CallbackContextImpl::new(request, extensions);
    request
        .call_domain_resolved_callbacks(&mut context, domain, answers)
        .map_err(|err| make_callback_try_error(err, retried))
}

fn call_to_choose_ips_callbacks(
    request: &InnerRequestParts<'_>,
    ips: &[IpAddrWithPort],
    extensions: &mut Extensions,
    retried: &RetriedStatsInfo,
) -> Result<(), TryError> {
    let mut context = CallbackContextImpl::new(request, extensions);
    request
        .call_to_choose_ips_callbacks(&mut context, ips)
        .map_err(|err| make_callback_try_error(err, retried))
}

fn call_ips_chosen_callbacks(
    request: &InnerRequestParts<'_>,
    ips: &[IpAddrWithPort],
    chosen: &[IpAddrWithPort],
    extensions: &mut Extensions,
    retried: &RetriedStatsInfo,
) -> Result<(), TryError> {
    let mut context = CallbackContextImpl::new(request, extensions);
    request
        .call_ips_chosen_callbacks(&mut context, ips, chosen)
        .map_err(|err| make_callback_try_error(err, retried))
}

pub(super) fn call_before_request_signed_callbacks(
    request: &InnerRequestParts<'_>,
    built: &mut HttpRequestParts<'_>,
    retried: &RetriedStatsInfo,
) -> Result<(), TryError> {
    let mut context = ExtendedCallbackContextImpl::new(request, built, retried);
    request
        .call_before_request_signed_callbacks(&mut context)
        .map_err(|err| make_callback_try_error(err, retried))
}

pub(super) fn call_after_request_signed_callbacks(
    request: &InnerRequestParts<'_>,
    built: &mut HttpRequestParts<'_>,
    retried: &RetriedStatsInfo,
) -> Result<(), TryError> {
    let mut context = ExtendedCallbackContextImpl::new(request, built, retried);
    request
        .call_after_request_signed_callbacks(&mut context)
        .map_err(|err| make_callback_try_error(err, retried))
}

pub(super) fn call_response_callbacks(
    request: &InnerRequestParts<'_>,
    built: &mut HttpRequestParts<'_>,
    retried: &RetriedStatsInfo,
    response: &ResponseParts,
) -> Result<(), ResponseError> {
    let mut context = ExtendedCallbackContextImpl::new(request, built, retried);
    request
        .call_response_callbacks(&mut context, response)
        .map_err(|err| make_callback_response_error(err, retried))
}

pub(super) fn call_error_callbacks(
    request: &InnerRequestParts<'_>,
    built: &mut HttpRequestParts<'_>,
    retried: &RetriedStatsInfo,
    response_error: &ResponseError,
) -> Result<(), TryError> {
    let mut context = ExtendedCallbackContextImpl::new(request, built, retried);
    request
        .call_error_callbacks(&mut context, response_error)
        .map_err(|err| make_callback_try_error(err, retried))
}

pub(super) fn find_domains_with_port(endpoints: &[Endpoint]) -> impl Iterator<Item = &DomainWithPort> {
    endpoints.iter().filter_map(|endpoint| match endpoint {
        Endpoint::DomainWithPort(domain_with_port) => Some(domain_with_port),
        _ => None,
    })
}

pub(super) fn find_ip_addr_with_port(endpoints: &[Endpoint]) -> impl Iterator<Item = &IpAddrWithPort> {
    endpoints.iter().filter_map(|endpoint| match endpoint {
        Endpoint::IpAddrWithPort(ip_addr_with_port) => Some(ip_addr_with_port),
        _ => None,
    })
}

pub(super) fn sign_request(
    request: &mut SyncHttpRequest<'_>,
    authorization: Option<&Authorization<'_>>,
    retried: &RetriedStatsInfo,
) -> Result<(), TryError> {
    if let Some(authorization) = authorization {
        authorization
            .sign(request)
            .map_err(|err| handle_sign_request_error(err, retried))?;
    }
    Ok(())
}

fn handle_sign_request_error(err: AuthorizationError, retried: &RetriedStatsInfo) -> TryError {
    match err {
        AuthorizationError::IoError(err) => make_local_io_try_error(err, retried),
        AuthorizationError::UrlParseError(err) => make_parse_try_error(err, retried),
        AuthorizationError::CallbackError(err) => make_callback_try_error(err, retried),
    }
}

fn make_local_io_try_error(err: IoError, retried: &RetriedStatsInfo) -> TryError {
    TryError::new(
        ResponseError::new(ResponseErrorKind::HttpError(HttpResponseErrorKind::LocalIoError), err).retried(retried),
        RetryDecision::DontRetry.into(),
    )
}

fn make_parse_try_error(err: UrlParseError, retried: &RetriedStatsInfo) -> TryError {
    TryError::new(
        ResponseError::new(ResponseErrorKind::HttpError(HttpResponseErrorKind::InvalidUrl), err).retried(retried),
        RetryDecision::TryNextServer.into(),
    )
}

fn make_callback_try_error(err: AnyError, retried: &RetriedStatsInfo) -> TryError {
    TryError::new(
        make_callback_response_error(err, retried),
        RetryDecision::DontRetry.into(),
    )
}

fn make_callback_response_error(err: AnyError, retried: &RetriedStatsInfo) -> ResponseError {
    ResponseError::new(HttpResponseErrorKind::CallbackError.into(), err).retried(retried)
}

pub(super) fn resolve(
    request: &InnerRequestParts<'_>,
    domain_with_port: &DomainWithPort,
    extensions: &mut Extensions,
    retried: &RetriedStatsInfo,
) -> Result<Vec<IpAddrWithPort>, TryError> {
    let answers = with_resolve_domain(request, domain_with_port.domain(), extensions, retried, || {
        request.http_client().resolver().resolve(
            domain_with_port.domain(),
            ResolveOptions::builder().retried(retried).build(),
        )
    })?;
    return Ok(answers
        .into_ip_addrs()
        .iter()
        .map(|&ip| IpAddrWithPort::new(ip, domain_with_port.port()))
        .collect());

    fn with_resolve_domain(
        request: &InnerRequestParts<'_>,
        domain: &str,
        extensions: &mut Extensions,
        retried: &RetriedStatsInfo,
        f: impl FnOnce() -> ResolveResult,
    ) -> Result<ResolveAnswers, TryError> {
        call_to_resolve_domain_callbacks(request, domain, extensions, retried)?;
        let answers = f().map_err(|err| TryError::new(err, RetryDecision::TryNextServer.into()))?;
        call_domain_resolved_callbacks(request, domain, &answers, extensions, retried)?;
        Ok(answers)
    }
}

pub(super) fn choose(
    request: &InnerRequestParts<'_>,
    ips: &[IpAddrWithPort],
    extensions: &mut Extensions,
    retried: &RetriedStatsInfo,
) -> Result<Vec<IpAddrWithPort>, TryError> {
    call_to_choose_ips_callbacks(request, ips, extensions, retried)?;
    let chosen_ips = request
        .http_client()
        .chooser()
        .choose(ips, Default::default())
        .into_ip_addrs();
    call_ips_chosen_callbacks(request, ips, &chosen_ips, extensions, retried)?;
    Ok(chosen_ips)
}

pub(super) fn judge(mut response: SyncResponse, retried: &RetriedStatsInfo) -> ApiResult<SyncResponse> {
    return match response.status_code().as_u16() {
        0..=199 | 300..=399 => Err(make_unexpected_status_code_error(response.parts(), retried)),
        200..=299 => {
            check_x_req_id(&mut response, retried)?;
            Ok(response)
        }
        _ => to_status_code_error(response, retried),
    };

    fn to_status_code_error(response: SyncResponse, retried: &RetriedStatsInfo) -> ApiResult<SyncResponse> {
        let status_code = response.status_code();
        let (parts, body) = response.parse_json::<ErrorResponseBody>()?.into_parts_and_body();
        Err(
            ResponseError::new_with_msg(ResponseErrorKind::StatusCodeError(status_code), body.into_error())
                .response_parts(&parts)
                .retried(retried),
        )
    }
}

fn check_x_req_id(response: &mut SyncResponse, retried: &RetriedStatsInfo) -> ApiResult<()> {
    if response.x_reqid().is_some() {
        Ok(())
    } else {
        Err(make_malicious_response(response.parts(), retried).read_response_body_sample(response.body_mut())?)
    }
}

#[cfg(feature = "async")]
async fn async_check_x_req_id(response: &mut AsyncResponse, retried: &RetriedStatsInfo) -> ApiResult<()> {
    if response.x_reqid().is_some() {
        Ok(())
    } else {
        Err(make_malicious_response(response.parts(), retried)
            .async_read_response_body_sample(response.body_mut())
            .await?)
    }
}

fn make_malicious_response(parts: &ResponseParts, retried: &RetriedStatsInfo) -> ResponseError {
    ResponseError::new_with_msg(
        ResponseErrorKind::MaliciousResponse,
        "cannot find X-ReqId header from response, might be malicious response",
    )
    .response_parts(parts)
    .retried(retried)
}

fn make_unexpected_status_code_error(parts: &ResponseParts, retried: &RetriedStatsInfo) -> ResponseError {
    ResponseError::new_with_msg(
        ResponseErrorKind::UnexpectedStatusCode(parts.status_code()),
        format!("status code {} is unexpected", parts.status_code()),
    )
    .response_parts(parts)
    .retried(retried)
}

#[cfg(feature = "async")]
mod async_utils {
    use super::{
        super::super::{AsyncResponse, InnerRequestParts},
        *,
    };
    use qiniu_http::AsyncRequest as AsyncHttpRequest;
    use std::future::Future;

    pub(in super::super) async fn sign_async_request(
        request: &mut AsyncHttpRequest<'_>,
        authorization: Option<&Authorization<'_>>,
        retried: &RetriedStatsInfo,
    ) -> Result<(), TryError> {
        if let Some(authorization) = authorization {
            authorization
                .async_sign(request)
                .await
                .map_err(|err| handle_sign_request_error(err, retried))?;
        }
        Ok(())
    }

    pub(in super::super) async fn async_resolve(
        parts: &InnerRequestParts<'_>,
        domain_with_port: &DomainWithPort,
        extensions: &mut Extensions,
        retried: &RetriedStatsInfo,
    ) -> Result<Vec<IpAddrWithPort>, TryError> {
        let answers = with_resolve_domain(parts, domain_with_port.domain(), extensions, retried, || async {
            parts
                .http_client()
                .resolver()
                .async_resolve(
                    domain_with_port.domain(),
                    ResolveOptions::builder().retried(retried).build(),
                )
                .await
        });
        return Ok(answers
            .await?
            .into_ip_addrs()
            .iter()
            .map(|&ip| IpAddrWithPort::new(ip, domain_with_port.port()))
            .collect());

        async fn with_resolve_domain<F: FnOnce() -> Fu, Fu: Future<Output = ResolveResult>>(
            parts: &InnerRequestParts<'_>,
            domain: &str,
            extensions: &mut Extensions,
            retried: &RetriedStatsInfo,
            f: F,
        ) -> Result<ResolveAnswers, TryError> {
            call_to_resolve_domain_callbacks(parts, domain, extensions, retried)?;
            let answers = f()
                .await
                .map_err(|err| TryError::new(err, RetryDecision::TryNextServer.into()))?;
            call_domain_resolved_callbacks(parts, domain, &answers, extensions, retried)?;
            Ok(answers)
        }
    }

    pub(in super::super) async fn async_choose(
        parts: &InnerRequestParts<'_>,
        ips: &[IpAddrWithPort],
        extensions: &mut Extensions,
        retried: &RetriedStatsInfo,
    ) -> Result<Vec<IpAddrWithPort>, TryError> {
        call_to_choose_ips_callbacks(parts, ips, extensions, retried)?;
        let chosen_ips = parts
            .http_client()
            .chooser()
            .async_choose(ips, Default::default())
            .await
            .into_ip_addrs();
        call_ips_chosen_callbacks(parts, ips, &chosen_ips, extensions, retried)?;
        Ok(chosen_ips)
    }

    pub(in super::super) async fn async_judge(
        mut response: AsyncResponse,
        retried: &RetriedStatsInfo,
    ) -> ApiResult<AsyncResponse> {
        return match response.status_code().as_u16() {
            0..=199 | 300..=399 => Err(make_unexpected_status_code_error(response.parts(), retried)),
            200..=299 => {
                async_check_x_req_id(&mut response, retried).await?;
                Ok(response)
            }
            _ => to_status_code_error(response, retried).await,
        };

        async fn to_status_code_error(response: AsyncResponse, retried: &RetriedStatsInfo) -> ApiResult<AsyncResponse> {
            let status_code = response.status_code();
            let (parts, body) = response.parse_json::<ErrorResponseBody>().await?.into_parts_and_body();
            Err(
                ResponseError::new_with_msg(ResponseErrorKind::StatusCodeError(status_code), body.into_error())
                    .response_parts(&parts)
                    .retried(retried),
            )
        }
    }
}

#[cfg(feature = "async")]
pub(super) use async_utils::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_utils::make_dumb_client_builder, Endpoints, ServiceName};
    use std::{
        error::Error,
        net::{Ipv4Addr, Ipv6Addr, SocketAddr},
        num::NonZeroU16,
        result::Result,
    };

    #[test]
    fn test_call_utils_make_url() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let default_client = make_dumb_client_builder().build();
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com", None), vec![]),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com/");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("/fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com", None), vec![]),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com/fake/path");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com", None), vec![]),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com/fake/path");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com", None), vec![]),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com/fake/path");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com", None), vec![]),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com/fake/path");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com", NonZeroU16::new(8080)), vec![]),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com:8080/fake/path");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com", NonZeroU16::new(8080)), vec![]),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com:8080/fake/path");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into(), None).into(),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://192.168.1.4/fake/path");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::from(SocketAddr::new(Ipv4Addr::new(192, 168, 1, 4).into(), 8080)).into(),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://192.168.1.4:8080/fake/path");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff).into(), None).into(),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://[::ffff:192.10.2.255]/fake/path");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::from(SocketAddr::new(
                    Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff).into(),
                    8080,
                ))
                .into(),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://[::ffff:192.10.2.255]:8080/fake/path");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new(
                        SocketAddr::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff).into(), 8080).into(),
                    ),
                )
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::from(SocketAddr::new(
                    Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00b, 0x2ff).into(),
                    8080,
                ))
                .into(),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://[::ffff:192.11.2.255]:8080/fake/path");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .append_query_pair("sign", "155d24fea16df8c77e9b9eec08a895f7")
                .append_query_pair("t", "5f99714f")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into(), None).into(),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(
                &url.to_string(),
                "https://192.168.1.4/fake/path?sign=155d24fea16df8c77e9b9eec08a895f7&t=5f99714f"
            );
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .query("avthumb/mp4")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into(), None).into(),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://192.168.1.4/fake/path?avthumb/mp4");
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .query("avthumb/mp4")
                .append_query_pair("sign", "155d24fea16df8c77e9b9eec08a895f7")
                .append_query_pair("t", "5f99714f")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into(), None).into(),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(
                &url.to_string(),
                "https://192.168.1.4/fake/path?avthumb/mp4&sign=155d24fea16df8c77e9b9eec08a895f7&t=5f99714f"
            );
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .query("avthumb/mp4")
                .append_query_pair("sign", "155d24fea16df8c77e9b9eec08a895f7")
                .append_query_pair("t", "5f99714f")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into(), None).into(),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(
                &url.to_string(),
                "https://192.168.1.4/fake/path?avthumb/mp4&sign=155d24fea16df8c77e9b9eec08a895f7&t=5f99714f"
            );
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .build()
                .split();
            let err = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com/", None), vec![]),
                &parts,
                &Default::default(),
            )
            .unwrap_err();
            assert_eq!(err.response_error().kind(), HttpResponseErrorKind::InvalidUrl.into(),);
        }
        {
            let (parts, _, _, _, _) = default_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .build()
                .split();
            let err = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com/", NonZeroU16::new(8080)), vec![]),
                &parts,
                &Default::default(),
            )
            .unwrap_err();
            assert_eq!(err.response_error().kind(), HttpResponseErrorKind::InvalidUrl.into(),);
        }

        let http_client = make_dumb_client_builder().use_https(false).build();
        {
            let (parts, _, _, _, _) = http_client
                .get(&[ServiceName::Up], Endpoints::new("fakedomain.com".parse().unwrap()))
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com", None), vec![]),
                &parts,
                &Default::default(),
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "http://fakedomain.com/fake/path");
        }

        Ok(())
    }
}
