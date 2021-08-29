use super::{
    super::{
        super::{DomainWithPort, Endpoint, IpAddrWithPort},
        CallbackContext, RequestWithoutEndpoints, ResolveAnswers, ResponseError, ResponseInfo,
        RetriedStatsInfo, RetryResult,
    },
    domain_or_ip_addr::DomainOrIpAddr,
    error::TryError,
};
use qiniu_http::{
    uri::{Authority, InvalidUri, PathAndQuery, Scheme, Uri},
    Extensions, Request as HTTPRequest, ResponseErrorKind as HTTPResponseErrorKind,
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
        .appended_user_agent(request.appended_user_agent())
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
            RetryResult::TryNextServer,
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
                    authority.push_str(":");
                    authority.push_str(&port.get().to_string());
                }
                authority.parse()?
            }
            DomainOrIpAddr::IpAddr(ip_addr_with_port) => {
                let mut authority = ip_addr_with_port.ip_addr().to_string();
                if let Some(port) = ip_addr_with_port.port() {
                    authority.push_str(":");
                    authority.push_str(&port.get().to_string());
                }
                authority.parse()?
            }
        };
        let mut path_and_query = request.path().to_owned();
        if !request.query().is_empty() || !request.query_pairs().is_empty() {
            let path_len = path_and_query.len();
            path_and_query.push_str("?");
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
pub(super) fn call_before_retry_delay_callbacks(
    request: &RequestWithoutEndpoints,
    built_request: &mut HTTPRequest,
    retried: &RetriedStatsInfo,
    delay: Duration,
) -> Result<(), TryError> {
    if !request.call_before_retry_delay_callbacks(
        &mut CallbackContext::new(request, built_request, retried),
        delay,
    ) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_before_retry_delay() callback returns false",
            ),
            RetryResult::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
pub(super) fn call_after_retry_delay_callbacks(
    request: &RequestWithoutEndpoints,
    built_request: &mut HTTPRequest,
    retried: &RetriedStatsInfo,
    delay: Duration,
) -> Result<(), TryError> {
    if !request.call_after_retry_delay_callbacks(
        &mut CallbackContext::new(request, built_request, retried),
        delay,
    ) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_after_retry_delay() callback returns false",
            ),
            RetryResult::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
pub(super) fn call_to_resolve_domain_callbacks(
    request: &RequestWithoutEndpoints,
    domain: &str,
) -> Result<(), TryError> {
    if !request.call_to_resolve_domain_callbacks(domain) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_to_resolve_domain_callbacks() callback returns false",
            ),
            RetryResult::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
pub(super) fn call_domain_resolved_callbacks(
    request: &RequestWithoutEndpoints,
    domain: &str,
    answers: &ResolveAnswers,
) -> Result<(), TryError> {
    if !request.call_domain_resolved_callbacks(domain, answers) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_domain_resolved_callbacks() callback returns false",
            ),
            RetryResult::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
pub(super) fn call_to_choose_ips_callbacks(
    request: &RequestWithoutEndpoints,
    ips: &[IpAddrWithPort],
) -> Result<(), TryError> {
    if !request.call_to_choose_ips_callbacks(ips) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_to_choose_ips_callbacks() callback returns false",
            ),
            RetryResult::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
pub(super) fn call_ips_chosen_callbacks(
    request: &RequestWithoutEndpoints,
    ips: &[IpAddrWithPort],
    chosen: &[IpAddrWithPort],
) -> Result<(), TryError> {
    if !request.call_ips_chosen_callbacks(ips, chosen) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_ips_chosen_callbacks() callback returns false",
            ),
            RetryResult::DontRetry,
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
    if !request.call_before_request_signed_callbacks(&mut CallbackContext::new(
        request,
        built_request,
        retried,
    )) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_before_request_signed() callback returns false",
            ),
            RetryResult::DontRetry,
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
    if !request.call_after_request_signed_callbacks(&mut CallbackContext::new(
        request,
        built_request,
        retried,
    )) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_after_request_signed() callback returns false",
            ),
            RetryResult::DontRetry,
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
    if !request.call_success_callbacks(
        &mut CallbackContext::new(request, built_request, retried),
        response,
    ) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_success() callback returns false",
            ),
            RetryResult::DontRetry,
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
    if !request.call_error_callbacks(
        &mut CallbackContext::new(request, built_request, retried),
        response_error,
    ) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_error() callback returns false",
            ),
            RetryResult::DontRetry,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_utils::make_dumb_client_builder, ServiceName};
    use std::{
        error::Error,
        net::{Ipv4Addr, Ipv6Addr, SocketAddr},
        result::Result,
    };

    #[test]
    fn test_call_utils_make_url() -> Result<(), Box<dyn Error>> {
        let default_client = make_dumb_client_builder().build();
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com"), vec![]),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(url.as_str(), "https://fakedomain.com/");
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("/fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com"), vec![]),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(url.as_str(), "https://fakedomain.com/fake/path");
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com"), vec![]),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(url.as_str(), "https://fakedomain.com/fake/path");
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com"), vec![]),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(url.as_str(), "https://fakedomain.com/fake/path");
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com"), vec![]),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(url.as_str(), "https://fakedomain.com/fake/path");
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new_with_port("fakedomain.com", 8080),
                    vec![],
                ),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(url.as_str(), "https://fakedomain.com:8080/fake/path");
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new_with_port("fakedomain.com", 8080),
                    vec![],
                ),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(url.as_str(), "https://fakedomain.com:8080/fake/path");
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into()).into(),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(url.as_str(), "https://192.168.1.4/fake/path");
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
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
            assert_eq!(url.as_str(), "https://192.168.1.4:8080/fake/path");
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff).into())
                    .into(),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(url.as_str(), "https://[::ffff:c00a:2ff]/fake/path");
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
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
            assert_eq!(url.as_str(), "https://[::ffff:c00a:2ff]:8080/fake/path");
        }
        {
            let (request, _, _) = default_client
                .get(
                    ServiceName::Up,
                    vec![SocketAddr::new(
                        Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff).into(),
                        8080,
                    )],
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
            assert_eq!(url.as_str(), "https://[::ffff:c00b:2ff]:8080/fake/path");
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("fake/path")
                .append_query_pair("sign", "155d24fea16df8c77e9b9eec08a895f7")
                .append_query_pair("t", "5f99714f")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into()).into(),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(
                url.as_str(),
                "https://192.168.1.4/fake/path?sign=155d24fea16df8c77e9b9eec08a895f7&t=5f99714f"
            );
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("fake/path")
                .query("avthumb/mp4")
                .append_query_pair("sign", "155d24fea16df8c77e9b9eec08a895f7")
                .append_query_pair("t", "5f99714f")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &IpAddrWithPort::new(Ipv4Addr::new(192, 168, 1, 4).into()).into(),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(
                url.as_str(),
                "https://192.168.1.4/fake/path?avthumb/mp4&sign=155d24fea16df8c77e9b9eec08a895f7&t=5f99714f"
            );
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("fake/path")
                .build()
                .split();
            let err = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com/"), vec![]),
                &request,
            )
            .unwrap_err();
            assert_eq!(
                err.response_error().kind(),
                HTTPResponseErrorKind::InvalidURLError.into(),
            );
        }
        {
            let (request, _, _) = default_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("fake/path")
                .build()
                .split();
            let err = make_url(
                &DomainOrIpAddr::new_from_domain(
                    DomainWithPort::new_with_port("fakedomain.com/", 8080),
                    vec![],
                ),
                &request,
            )
            .unwrap_err();
            assert_eq!(
                err.response_error().kind(),
                HTTPResponseErrorKind::InvalidURLError.into(),
            );
        }

        let http_client = make_dumb_client_builder().use_https(false).build();
        {
            let (request, _, _) = http_client
                .get(ServiceName::Up, vec!["fakedomain.com".to_owned()])
                .path("fake/path")
                .build()
                .split();
            let (url, resolved_ips) = make_url(
                &DomainOrIpAddr::new_from_domain(DomainWithPort::new("fakedomain.com"), vec![]),
                &request,
            )
            .unwrap();
            assert!(resolved_ips.is_empty());
            assert_eq!(url.as_str(), "http://fakedomain.com/fake/path");
        }

        Ok(())
    }
}
