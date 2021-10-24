use super::{
    super::{
        super::{DomainWithPort, Endpoint, IpAddrWithPort},
        APIResult, Authorization, AuthorizationError, CallbackContextImpl,
        ExtendedCallbackContextImpl, RequestWithoutEndpoints, ResolveAnswers, ResolveResult,
        ResponseError, ResponseErrorKind, ResponseInfo, RetriedStatsInfo, RetryDecision,
        SimplifiedCallbackContext, SyncResponse,
    },
    domain_or_ip_addr::DomainOrIpAddr,
    error::{ErrorResponseBody, TryError},
};
use qiniu_http::{
    uri::{Authority, InvalidUri, PathAndQuery, Scheme, Uri},
    Extensions, Request as HTTPRequest, ResponseErrorKind as HTTPResponseErrorKind, StatusCode,
};
use std::{borrow::Cow, net::IpAddr, time::Duration};

pub(super) fn make_request<'r>(
    url: Uri,
    request: &'r RequestWithoutEndpoints,
    extensions: Extensions,
    resolved_ips: &'r [IpAddr],
) -> HTTPRequest<'r> {
    let mut request_builder = HTTPRequest::builder();
    request_builder
        .url(url)
        .method(request.method().to_owned())
        .version(request.version())
        .headers(request.headers().to_owned())
        .body(request.body())
        .appended_user_agent(request.appended_user_agent().to_owned())
        .resolved_ip_addrs(resolved_ips)
        .extensions(extensions)
        .build()
}

pub(super) fn extract_ips_from(domain_or_ip: &DomainOrIpAddr) -> Cow<[IpAddrWithPort]> {
    match domain_or_ip {
        DomainOrIpAddr::Domain { resolved_ips, .. } => Cow::Borrowed(resolved_ips),
        &DomainOrIpAddr::IpAddr(ip_addr) => Cow::Owned(vec![ip_addr]),
    }
}

pub(super) fn make_url(
    domain_or_ip: &DomainOrIpAddr,
    request: &RequestWithoutEndpoints,
) -> Result<(Uri, Vec<IpAddr>), TryError> {
    return _make_url(domain_or_ip, request).map_err(|err| {
        TryError::new(
            ResponseError::new(HTTPResponseErrorKind::InvalidURL.into(), err),
            RetryDecision::TryNextServer,
        )
    });

    fn _make_url(
        domain_or_ip: &DomainOrIpAddr,
        request: &RequestWithoutEndpoints,
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
                resolved_ip_addrs = resolved_ips
                    .iter()
                    .map(|resolved| resolved.ip_addr())
                    .collect();
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
            let mut serializer =
                form_urlencoded::Serializer::for_suffix(&mut path_and_query, path_len);
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

#[inline]
pub(super) fn call_before_backoff_callbacks(
    request: &RequestWithoutEndpoints,
    built_request: &mut HTTPRequest,
    retried: &RetriedStatsInfo,
    delay: Duration,
) -> Result<(), TryError> {
    if !request.call_before_backoff_callbacks(
        &mut ExtendedCallbackContextImpl::new(request, built_request, retried),
        delay,
    ) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_before_backoff() callback returns false",
            ),
            RetryDecision::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
pub(super) fn call_after_backoff_callbacks(
    request: &RequestWithoutEndpoints,
    built_request: &mut HTTPRequest,
    retried: &RetriedStatsInfo,
    delay: Duration,
) -> Result<(), TryError> {
    if !request.call_after_backoff_callbacks(
        &mut ExtendedCallbackContextImpl::new(request, built_request, retried),
        delay,
    ) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_after_backoff() callback returns false",
            ),
            RetryDecision::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
fn call_to_resolve_domain_callbacks(
    request: &RequestWithoutEndpoints,
    domain: &str,
    extensions: &mut Extensions,
) -> Result<(), TryError> {
    let mut context = CallbackContextImpl::new(request, extensions);
    if !request.call_to_resolve_domain_callbacks(&mut context, domain) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_to_resolve_domain_callbacks() callback returns false",
            ),
            RetryDecision::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
fn call_domain_resolved_callbacks(
    request: &RequestWithoutEndpoints,
    domain: &str,
    answers: &ResolveAnswers,
    extensions: &mut Extensions,
) -> Result<(), TryError> {
    let mut context = CallbackContextImpl::new(request, extensions);
    if !request.call_domain_resolved_callbacks(&mut context, domain, answers) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_domain_resolved_callbacks() callback returns false",
            ),
            RetryDecision::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
fn call_to_choose_ips_callbacks(
    request: &RequestWithoutEndpoints,
    ips: &[IpAddrWithPort],
    extensions: &mut Extensions,
) -> Result<(), TryError> {
    let mut context = CallbackContextImpl::new(request, extensions);
    if !request.call_to_choose_ips_callbacks(&mut context, ips) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_to_choose_ips_callbacks() callback returns false",
            ),
            RetryDecision::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
fn call_ips_chosen_callbacks(
    request: &RequestWithoutEndpoints,
    ips: &[IpAddrWithPort],
    chosen: &[IpAddrWithPort],
    extensions: &mut Extensions,
) -> Result<(), TryError> {
    let mut context = CallbackContextImpl::new(request, extensions);
    if !request.call_ips_chosen_callbacks(&mut context, ips, chosen) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_ips_chosen_callbacks() callback returns false",
            ),
            RetryDecision::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
pub(super) fn call_before_request_signed_callbacks(
    request: &RequestWithoutEndpoints,
    built_request: &mut HTTPRequest,
    retried: &mut RetriedStatsInfo,
) -> Result<(), TryError> {
    let mut context = ExtendedCallbackContextImpl::new(request, built_request, retried);
    if !request.call_before_request_signed_callbacks(&mut context) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_before_request_signed() callback returns false",
            ),
            RetryDecision::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
pub(super) fn call_after_request_signed_callbacks(
    request: &RequestWithoutEndpoints,
    built_request: &mut HTTPRequest,
    retried: &mut RetriedStatsInfo,
) -> Result<(), TryError> {
    let mut context = ExtendedCallbackContextImpl::new(request, built_request, retried);
    if !request.call_after_request_signed_callbacks(&mut context) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_after_request_signed() callback returns false",
            ),
            RetryDecision::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
pub(super) fn call_success_callbacks(
    request: &RequestWithoutEndpoints,
    built_request: &mut HTTPRequest,
    retried: &RetriedStatsInfo,
    response: &ResponseInfo,
) -> Result<(), TryError> {
    let mut context = ExtendedCallbackContextImpl::new(request, built_request, retried);
    if !request.call_success_callbacks(&mut context, response) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_success() callback returns false",
            ),
            RetryDecision::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
pub(super) fn call_error_callbacks(
    request: &RequestWithoutEndpoints,
    built_request: &mut HTTPRequest,
    retried: &RetriedStatsInfo,
    response_error: &ResponseError,
) -> Result<(), TryError> {
    let mut context = ExtendedCallbackContextImpl::new(request, built_request, retried);
    if !request.call_error_callbacks(&mut context, response_error) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_error() callback returns false",
            ),
            RetryDecision::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
pub(super) fn find_domains_with_port(
    endpoints: &[Endpoint],
) -> impl Iterator<Item = &DomainWithPort> {
    endpoints.iter().filter_map(|endpoint| match endpoint {
        Endpoint::DomainWithPort(domain_with_port) => Some(domain_with_port),
        _ => None,
    })
}

#[inline]
pub(super) fn find_ip_addr_with_port(
    endpoints: &[Endpoint],
) -> impl Iterator<Item = &IpAddrWithPort> {
    endpoints.iter().filter_map(|endpoint| match endpoint {
        Endpoint::IpAddrWithPort(ip_addr_with_port) => Some(ip_addr_with_port),
        _ => None,
    })
}

#[inline]
pub(super) fn sign_request(
    request: &mut HTTPRequest<'_>,
    authorization: Option<&Authorization>,
) -> Result<(), TryError> {
    if let Some(authorization) = authorization {
        authorization
            .sign(request)
            .map_err(handle_sign_request_error)?;
    }
    Ok(())
}

#[inline]
fn handle_sign_request_error(err: AuthorizationError) -> TryError {
    match err {
        AuthorizationError::IOError(err) => TryError::new(
            ResponseError::new(
                ResponseErrorKind::HTTPError(HTTPResponseErrorKind::LocalIOError),
                err,
            ),
            RetryDecision::DontRetry,
        ),
        AuthorizationError::UrlParseError(err) => TryError::new(
            ResponseError::new(
                ResponseErrorKind::HTTPError(HTTPResponseErrorKind::InvalidURL),
                err,
            ),
            RetryDecision::TryNextServer,
        ),
    }
}

#[inline]
pub(super) fn resolve(
    request: &RequestWithoutEndpoints<'_>,
    domain_with_port: &DomainWithPort,
    extensions: &mut Extensions,
) -> Result<Vec<IpAddrWithPort>, TryError> {
    let answers = with_resolve_domain(request, domain_with_port.domain(), extensions, || {
        request
            .http_client()
            .resolver()
            .resolve(domain_with_port.domain(), &Default::default())
    })?;
    return Ok(answers
        .into_ip_addrs()
        .iter()
        .map(|&ip| IpAddrWithPort::new(ip, domain_with_port.port()))
        .collect());

    #[inline]
    fn with_resolve_domain(
        request: &RequestWithoutEndpoints<'_>,
        domain: &str,
        extensions: &mut Extensions,
        f: impl FnOnce() -> ResolveResult,
    ) -> Result<ResolveAnswers, TryError> {
        call_to_resolve_domain_callbacks(request, domain, extensions)?;
        let answers = f().map_err(|err| TryError::new(err, RetryDecision::TryNextServer))?;
        call_domain_resolved_callbacks(request, domain, &answers, extensions)?;
        Ok(answers)
    }
}

pub(super) fn choose(
    request: &RequestWithoutEndpoints<'_>,
    ips: &[IpAddrWithPort],
    extensions: &mut Extensions,
) -> Result<Vec<IpAddrWithPort>, TryError> {
    call_to_choose_ips_callbacks(request, ips, extensions)?;
    let chosen_ips = request
        .http_client()
        .chooser()
        .choose(ips, &Default::default())
        .into_ip_addrs();
    call_ips_chosen_callbacks(request, ips, &chosen_ips, extensions)?;
    Ok(chosen_ips)
}

#[inline]
pub(super) fn judge(response: SyncResponse) -> APIResult<SyncResponse> {
    check_x_req_id(response.x_req_id())?;
    return match response.status_code().as_u16() {
        0..=199 | 300..=399 => Err(make_unexpected_status_code_error(response.status_code())),
        200..=299 => Ok(response),
        _ => to_status_code_error(response),
    };

    #[inline]
    fn to_status_code_error(response: SyncResponse) -> APIResult<SyncResponse> {
        let status_code = response.status_code();
        let error_response_body: ErrorResponseBody = response.parse_json()?.into_body();
        Err(ResponseError::new(
            ResponseErrorKind::StatusCodeError(status_code),
            error_response_body.into_error(),
        ))
    }
}

#[inline]
fn check_x_req_id(req_id: Option<&str>) -> APIResult<()> {
    req_id.map_or_else(
        || {
            Err(ResponseError::new(
                ResponseErrorKind::MaliciousResponse,
                "cannot find X-ReqId header from response, might be malicious response",
            ))
        },
        |_| Ok(()),
    )
}

#[inline]
fn make_unexpected_status_code_error(status_code: StatusCode) -> ResponseError {
    ResponseError::new(
        ResponseErrorKind::UnexpectedStatusCode(status_code),
        format!("status code {} is unexpected", status_code),
    )
}

#[cfg(feature = "async")]
mod async_utils {
    use super::{super::super::AsyncResponse, *};
    use std::future::Future;

    #[inline]
    pub(in super::super) async fn sign_async_request(
        request: &mut HTTPRequest<'_>,
        authorization: Option<&Authorization>,
    ) -> Result<(), TryError> {
        if let Some(authorization) = authorization {
            authorization
                .async_sign(request)
                .await
                .map_err(handle_sign_request_error)?;
        }
        Ok(())
    }

    #[inline]
    pub(in super::super) async fn async_resolve(
        request: &RequestWithoutEndpoints<'_>,
        domain_with_port: &DomainWithPort,
        extensions: &mut Extensions,
    ) -> Result<Vec<IpAddrWithPort>, TryError> {
        let answers =
            with_resolve_domain(request, domain_with_port.domain(), extensions, || async {
                request
                    .http_client()
                    .resolver()
                    .async_resolve(domain_with_port.domain(), &Default::default())
                    .await
            });
        return Ok(answers
            .await?
            .into_ip_addrs()
            .iter()
            .map(|&ip| IpAddrWithPort::new(ip, domain_with_port.port()))
            .collect());

        #[inline]
        async fn with_resolve_domain<F: FnOnce() -> Fu, Fu: Future<Output = ResolveResult>>(
            request: &RequestWithoutEndpoints<'_>,
            domain: &str,
            extensions: &mut Extensions,
            f: F,
        ) -> Result<ResolveAnswers, TryError> {
            call_to_resolve_domain_callbacks(request, domain, extensions)?;
            let answers = f()
                .await
                .map_err(|err| TryError::new(err, RetryDecision::TryNextServer))?;
            call_domain_resolved_callbacks(request, domain, &answers, extensions)?;
            Ok(answers)
        }
    }

    pub(in super::super) async fn async_choose(
        request: &RequestWithoutEndpoints<'_>,
        ips: &[IpAddrWithPort],
        extensions: &mut Extensions,
    ) -> Result<Vec<IpAddrWithPort>, TryError> {
        call_to_choose_ips_callbacks(request, ips, extensions)?;
        let chosen_ips = request
            .http_client()
            .chooser()
            .async_choose(ips, &Default::default())
            .await
            .into_ip_addrs();
        call_ips_chosen_callbacks(request, ips, &chosen_ips, extensions)?;
        Ok(chosen_ips)
    }

    #[inline]
    pub(in super::super) async fn async_judge(response: AsyncResponse) -> APIResult<AsyncResponse> {
        check_x_req_id(response.x_req_id())?;
        return match response.status_code().as_u16() {
            0..=199 | 300..=399 => Err(make_unexpected_status_code_error(response.status_code())),
            200..=299 => Ok(response),
            _ => to_status_code_error(response).await,
        };

        #[inline]
        async fn to_status_code_error(response: AsyncResponse) -> APIResult<AsyncResponse> {
            let status_code = response.status_code();
            let error_response_body: ErrorResponseBody = response.parse_json().await?.into_body();
            Err(ResponseError::new(
                ResponseErrorKind::StatusCodeError(status_code),
                error_response_body.into_error(),
            ))
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
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new("fakedomain.com", None),
                    vec![],
                ),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com/");
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("/fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new("fakedomain.com", None),
                    vec![],
                ),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com/fake/path");
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new("fakedomain.com", None),
                    vec![],
                ),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com/fake/path");
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new("fakedomain.com", None),
                    vec![],
                ),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com/fake/path");
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new("fakedomain.com", None),
                    vec![],
                ),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com/fake/path");
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new("fakedomain.com", NonZeroU16::new(8080)),
                    vec![],
                ),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com:8080/fake/path");
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new("fakedomain.com", NonZeroU16::new(8080)),
                    vec![],
                ),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://fakedomain.com:8080/fake/path");
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into(), None).into(),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://192.168.1.4/fake/path");
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::from(SocketAddr::new(Ipv4Addr::new(192, 168, 1, 4).into(), 8080))
                    .into(),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://192.168.1.4:8080/fake/path");
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(
                    Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff).into(),
                    None,
                )
                .into(),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "https://[::ffff:192.10.2.255]/fake/path");
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::from(SocketAddr::new(
                    Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff).into(),
                    8080,
                ))
                .into(),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(
                &url.to_string(),
                "https://[::ffff:192.10.2.255]:8080/fake/path"
            );
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new(SocketAddr::new(
                        Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff).into(),
                        8080,
                    )),
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
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(
                &url.to_string(),
                "https://[::ffff:192.11.2.255]:8080/fake/path"
            );
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .append_query_pair("sign", "155d24fea16df8c77e9b9eec08a895f7")
                .append_query_pair("t", "5f99714f")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into(), None).into(),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(
                &url.to_string(),
                "https://192.168.1.4/fake/path?sign=155d24fea16df8c77e9b9eec08a895f7&t=5f99714f"
            );
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .query("avthumb/mp4")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into(), None).into(),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(
                &url.to_string(),
                "https://192.168.1.4/fake/path?avthumb/mp4"
            );
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .query("avthumb/mp4")
                .append_query_pair("sign", "155d24fea16df8c77e9b9eec08a895f7")
                .append_query_pair("t", "5f99714f")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into(), None).into(),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(
                &url.to_string(),
                "https://192.168.1.4/fake/path?avthumb/mp4&sign=155d24fea16df8c77e9b9eec08a895f7&t=5f99714f"
            );
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .query("avthumb/mp4")
                .append_query_pair("sign", "155d24fea16df8c77e9b9eec08a895f7")
                .append_query_pair("t", "5f99714f")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into(), None).into(),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(
                &url.to_string(),
                "https://192.168.1.4/fake/path?avthumb/mp4&sign=155d24fea16df8c77e9b9eec08a895f7&t=5f99714f"
            );
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .build()
                .split();
            let err = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new("fakedomain.com/", None),
                    vec![],
                ),
                &request,
            )
            .unwrap_err();
            assert_eq!(
                err.response_error().kind(),
                HTTPResponseErrorKind::InvalidURL.into(),
            );
        }
        {
            let (request, _, _, _) = default_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .build()
                .split();
            let err = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new("fakedomain.com/", NonZeroU16::new(8080)),
                    vec![],
                ),
                &request,
            )
            .unwrap_err();
            assert_eq!(
                err.response_error().kind(),
                HTTPResponseErrorKind::InvalidURL.into(),
            );
        }

        let http_client = make_dumb_client_builder().use_https(false).build();
        {
            let (request, _, _, _) = http_client
                .get(
                    &[ServiceName::Up],
                    Endpoints::new("fakedomain.com".to_owned()),
                )
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new("fakedomain.com", None),
                    vec![],
                ),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(&url.to_string(), "http://fakedomain.com/fake/path");
        }

        Ok(())
    }
}
