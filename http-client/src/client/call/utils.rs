use super::{
    super::{
        super::{DomainWithPort, Endpoint, IpAddrWithPort},
        CallbackContext, ChosenResult, RequestWithoutEndpoints, ResponseError, ResponseInfo,
        RetriedStatsInfo, RetryResult,
    },
    domain_or_ip_addr::DomainOrIpAddr,
    error::TryError,
};
use qiniu_http::{Request as HTTPRequest, ResponseErrorKind as HTTPResponseErrorKind};
use std::{net::IpAddr, time::Duration};
use url::{ParseError as UrlParseError, Url};

pub(super) fn make_request<'r>(
    url: &'r str,
    request: &'r RequestWithoutEndpoints,
    resolved_ips: &'r [IpAddr],
) -> HTTPRequest<'r> {
    let mut request_builder = HTTPRequest::builder();
    request_builder
        .url(url)
        .method(request.method())
        .headers(request.headers().to_owned())
        .body(request.body())
        .appended_user_agent(request.appended_user_agent())
        .follow_redirection(request.follow_redirection())
        .resolved_ip_addrs(resolved_ips);
    if let Some(timeout) = request.connect_timeout() {
        request_builder.connect_timeout(timeout);
    }
    if let Some(timeout) = request.request_timeout() {
        request_builder.request_timeout(timeout);
    }
    if let Some(timeout) = request.tcp_keepalive_idle_timeout() {
        request_builder.tcp_keepalive_idle_timeout(timeout);
    }
    if let Some(interval) = request.tcp_keepalive_probe_interval() {
        request_builder.tcp_keepalive_probe_interval(interval);
    }
    if let (Some(speed), Some(timeout)) = (
        request.low_transfer_speed(),
        request.low_transfer_speed_timeout(),
    ) {
        request_builder.low_transfer_speed(speed, timeout);
    }
    request_builder.build()
}

pub(super) fn make_url(
    domain_or_ip: &DomainOrIpAddr,
    request: &RequestWithoutEndpoints,
) -> Result<(String, Vec<IpAddr>), TryError> {
    return _make_url(domain_or_ip, request).map_err(|err| {
        TryError::new(
            ResponseError::new(HTTPResponseErrorKind::InvalidURLError.into(), err),
            RetryResult::TryNextServer,
        )
    });

    fn _make_url(
        domain_or_ip: &DomainOrIpAddr,
        request: &RequestWithoutEndpoints,
    ) -> Result<(String, Vec<IpAddr>), UrlParseError> {
        let mut url = Url::parse("https://example.org/")?;
        let mut resolved_ips = Vec::new();
        match domain_or_ip {
            DomainOrIpAddr::Domain(domain) => {
                resolved_ips = domain
                    .resolved_ips()
                    .iter()
                    .map(|resolved| resolved.ip_addr())
                    .collect();
                let domain_with_port = domain.domain_with_port();
                url.set_host(Some(domain_with_port.domain()))?;
                if let Some(port) = domain_with_port.port() {
                    url.set_port(Some(port.get())).ok();
                }
            }
            DomainOrIpAddr::IpAddr(ip_addr_with_port) => {
                url.set_ip_host(ip_addr_with_port.ip_addr()).ok();
                if let Some(port) = ip_addr_with_port.port() {
                    url.set_port(Some(port.get())).ok();
                }
            }
        }
        if request.use_https() {
            url.set_scheme("https").ok();
        } else {
            url.set_scheme("http").ok();
        };
        if !request.path().is_empty() {
            url.set_path(request.path());
        }
        if !request.query().is_empty() {
            url.set_query(Some(request.query()));
        }

        if !request.query_pairs().is_empty() {
            url.query_pairs_mut()
                .extend_pairs(request.query_pairs().iter());
        }
        Ok((url.to_string(), resolved_ips))
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
        &mut CallbackContext::new(request.request_id(), built_request, retried),
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
        &mut CallbackContext::new(request.request_id(), built_request, retried),
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
pub(super) fn call_to_choose_domain_callbacks(
    request: &RequestWithoutEndpoints,
    domain: &str,
) -> Result<(), TryError> {
    if !request.call_to_choose_domain_callbacks(domain) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_to_choose_domain_callbacks() callback returns false",
            ),
            RetryResult::DontRetry,
        ));
    }
    Ok(())
}

#[inline]
pub(super) fn call_domain_chosen_callbacks(
    request: &RequestWithoutEndpoints,
    domain: &str,
    result: &ChosenResult,
) -> Result<(), TryError> {
    if !request.call_domain_chosen_callbacks(domain, result) {
        return Err(TryError::new(
            ResponseError::new(
                HTTPResponseErrorKind::UserCanceled.into(),
                "on_domain_chosen_callbacks() callback returns false",
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
        request.request_id(),
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
        request.request_id(),
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
        &mut CallbackContext::new(request.request_id(), built_request, retried),
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
        &mut CallbackContext::new(request.request_id(), built_request, retried),
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
