use super::{
    super::{
        super::{DomainWithPort, Endpoint, IpAddrWithPort},
        InnerRequestParts, ResponseError, ResponseErrorKind, RetriedStatsInfo, RetryDecision, SyncResponse,
    },
    domain_or_ip_addr::DomainOrIpAddr,
    error::{TryError, TryErrorWithExtensions},
    ip_addrs::IpAddrs,
    ip_addrs_set::IpAddrsSet,
    try_domain_or_ip_addr::try_domain_or_ip_addr,
    utils::{choose, find_domains_with_port, find_ip_addr_with_port, resolve},
};
use log::debug;
use qiniu_http::{Extensions, SyncRequestBody};
use std::mem::take;
use tap::TapFallible;

pub(super) fn try_endpoints(
    endpoints: &[Endpoint],
    parts: &InnerRequestParts<'_>,
    body: &mut SyncRequestBody<'_>,
    mut extensions: Extensions,
    tried_ips: &mut IpAddrsSet,
    retried: &mut RetriedStatsInfo,
    is_endpoints_alternative: bool,
) -> Result<SyncResponse, TryErrorWithExtensions> {
    let mut last_error: Option<TryError> = None;

    for domain_with_port in find_domains_with_port(endpoints) {
        debug!("Try domain with port: {}", domain_with_port);
        match try_domain_with_port(
            domain_with_port,
            tried_ips,
            parts,
            body,
            &mut extensions,
            retried,
            is_endpoints_alternative,
        ) {
            Ok(response) => return Ok(response),
            Err(ControlFlow::TryNext(Some(err))) => {
                let (err, ext) = err.split();
                extensions = ext;
                last_error = Some(err);
            }
            Err(ControlFlow::TryNext(None)) => {}
            Err(ControlFlow::DontRetry(err)) => {
                return Err(err);
            }
        }
    }

    let ips = find_ip_addr_with_port(endpoints).copied().collect::<Vec<_>>();
    if !ips.is_empty() {
        debug!("Try IPs with port: {:?}", ips);
        match try_ips(
            &ips,
            tried_ips,
            parts,
            body,
            &mut extensions,
            retried,
            is_endpoints_alternative,
        ) {
            Ok(response) => return Ok(response),
            Err(ControlFlow::TryNext(Some(err))) => {
                let (err, ext) = err.split();
                extensions = ext;
                last_error = Some(err);
            }
            Err(ControlFlow::TryNext(None)) => {}
            Err(ControlFlow::DontRetry(err)) => {
                return Err(err);
            }
        }
    }

    return Err(last_error
        .unwrap_or_else(|| no_try_error(retried))
        .with_extensions(extensions));

    fn try_domain_with_port(
        domain_with_port: &DomainWithPort,
        tried_ips: &mut IpAddrsSet,
        parts: &InnerRequestParts<'_>,
        body: &mut SyncRequestBody<'_>,
        extensions: &mut Extensions,
        retried: &mut RetriedStatsInfo,
        is_endpoints_alternative: bool,
    ) -> Result<SyncResponse, ControlFlow<TryErrorWithExtensions>> {
        retried.switch_endpoint();
        return if parts.http_client().http_caller().is_resolved_ip_addrs_supported() {
            debug!("Try domain with resolver: {}", domain_with_port);
            with_resolver(
                domain_with_port,
                tried_ips,
                parts,
                body,
                extensions,
                retried,
                is_endpoints_alternative,
            )
        } else {
            debug!("Try domain without resolver: {}", domain_with_port);
            without_resolver(
                domain_with_port,
                parts,
                body,
                extensions,
                retried,
                is_endpoints_alternative,
            )
        }
        .tap_err(|_| retried.increase_abandoned_endpoints());

        fn with_resolver(
            domain_with_port: &DomainWithPort,
            tried_ips: &mut IpAddrsSet,
            parts: &InnerRequestParts<'_>,
            body: &mut SyncRequestBody<'_>,
            extensions: &mut Extensions,
            retried: &mut RetriedStatsInfo,
            is_endpoints_alternative: bool,
        ) -> Result<SyncResponse, ControlFlow<TryErrorWithExtensions>> {
            let mut last_error: Option<TryError> = None;
            let ips = resolve(parts, domain_with_port, extensions, retried)
                .map_err(|err| err.with_extensions(take(extensions)))
                .map_err(Some)
                .map_err(ControlFlow::TryNext)?;
            if !ips.is_empty() {
                let mut remaining_ips = {
                    let mut ips = IpAddrsSet::new(&ips);
                    ips.difference_set(tried_ips);
                    ips
                };
                loop {
                    match try_domain_with_ips(
                        domain_with_port,
                        &mut remaining_ips,
                        tried_ips,
                        parts,
                        body,
                        extensions,
                        retried,
                        is_endpoints_alternative,
                    ) {
                        Ok(response) => return Ok(response),
                        Err(TryFlow::TryNext(None)) => {
                            break;
                        }
                        Err(TryFlow::TryAgain(err)) => {
                            let (err, ext) = err.split();
                            *extensions = ext;
                            last_error = Some(err);
                        }
                        Err(TryFlow::DontRetry(err)) => {
                            return Err(ControlFlow::DontRetry(err));
                        }
                        Err(TryFlow::TryNext(Some(err))) => {
                            return Err(ControlFlow::TryNext(Some(err)));
                        }
                    }
                }
            }
            Err(ControlFlow::TryNext(
                last_error.map(|err| err.with_extensions(take(extensions))),
            ))
        }

        fn without_resolver(
            domain_with_port: &DomainWithPort,
            parts: &InnerRequestParts<'_>,
            body: &mut SyncRequestBody<'_>,
            extensions: &mut Extensions,
            retried: &mut RetriedStatsInfo,
            is_endpoints_alternative: bool,
        ) -> Result<SyncResponse, ControlFlow<TryErrorWithExtensions>> {
            let domain = DomainOrIpAddr::new_from_domain(domain_with_port.to_owned(), vec![]);
            match try_domain_or_ip_addr(&domain, parts, body, take(extensions), retried) {
                Ok(response) => Ok(response),
                Err(err) => match err.retry_decision() {
                    RetryDecision::TryAlternativeEndpoints if is_endpoints_alternative => {
                        Err(ControlFlow::DontRetry(err))
                    }
                    RetryDecision::DontRetry => {
                        retried.increase_abandoned_ips_of_current_endpoint();
                        Err(ControlFlow::DontRetry(err))
                    }
                    _ => {
                        retried.increase_abandoned_ips_of_current_endpoint();
                        Err(ControlFlow::TryNext(Some(err)))
                    }
                },
            }
        }

        #[allow(clippy::too_many_arguments)]
        fn try_domain_with_ips(
            domain_with_port: &DomainWithPort,
            remaining_ips: &mut IpAddrsSet,
            tried_ips: &mut IpAddrsSet,
            parts: &InnerRequestParts<'_>,
            body: &mut SyncRequestBody<'_>,
            extensions: &mut Extensions,
            retried: &mut RetriedStatsInfo,
            is_endpoints_alternative: bool,
        ) -> Result<SyncResponse, TryFlow<TryErrorWithExtensions>> {
            let chosen_ips = match remaining_ips.remains() {
                ips if !ips.is_empty() => choose(parts, &ips, extensions, retried)
                    .map_err(|err| err.with_extensions(take(extensions)))
                    .map_err(TryFlow::TryAgain)?,
                _ => vec![],
            };
            if chosen_ips.is_empty() {
                Err(TryFlow::TryNext(None))
            } else {
                remaining_ips.difference_slice(&chosen_ips);
                tried_ips.union_slice(&chosen_ips);
                retried.switch_ips();
                let chosen_ips = IpAddrs::from(chosen_ips);
                debug!("Try domain with IPs: {}({})", domain_with_port, chosen_ips);
                match try_domain_or_single_ip(
                    &DomainOrIpAddr::new_from_domain(domain_with_port.to_owned(), chosen_ips.into()),
                    parts,
                    body,
                    take(extensions),
                    retried,
                    is_endpoints_alternative,
                ) {
                    Ok(response) => Ok(response),
                    Err(SingleTryFlow::TryAgain(err)) => Err(TryFlow::TryAgain(err)),
                    Err(SingleTryFlow::DontRetry(err)) => Err(TryFlow::DontRetry(err)),
                }
            }
        }
    }

    fn try_ips(
        ips: &[IpAddrWithPort],
        tried_ips: &mut IpAddrsSet,
        parts: &InnerRequestParts<'_>,
        body: &mut SyncRequestBody<'_>,
        extensions: &mut Extensions,
        retried: &mut RetriedStatsInfo,
        is_endpoints_alternative: bool,
    ) -> Result<SyncResponse, ControlFlow<TryErrorWithExtensions>> {
        let mut last_error: Option<TryError> = None;

        let mut remaining_ips = {
            let mut ips = IpAddrsSet::new(ips);
            ips.difference_set(tried_ips);
            ips
        };
        loop {
            debug!("Try IPs: {}", remaining_ips);
            match try_remaining_ips(
                &mut remaining_ips,
                tried_ips,
                parts,
                body,
                extensions,
                retried,
                is_endpoints_alternative,
            ) {
                Ok(response) => return Ok(response),
                Err(ControlFlow::TryNext(Some(err))) => {
                    let (err, ext) = err.split();
                    *extensions = ext;
                    last_error = Some(err);
                }
                Err(ControlFlow::TryNext(None)) => {
                    break;
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        return Err(ControlFlow::TryNext(
            last_error.map(|err| err.with_extensions(take(extensions))),
        ));

        fn try_remaining_ips(
            remaining_ips: &mut IpAddrsSet,
            tried_ips: &mut IpAddrsSet,
            parts: &InnerRequestParts<'_>,
            body: &mut SyncRequestBody<'_>,
            extensions: &mut Extensions,
            retried: &mut RetriedStatsInfo,
            is_endpoints_alternative: bool,
        ) -> Result<SyncResponse, ControlFlow<TryErrorWithExtensions>> {
            let mut last_error: Option<TryError> = None;
            let chosen_ips = match remaining_ips.remains() {
                ips if !ips.is_empty() => choose(parts, &ips, extensions, retried)
                    .map_err(|err| err.with_extensions(take(extensions)))
                    .map_err(Some)
                    .map_err(ControlFlow::TryNext)?,
                _ => vec![],
            };
            if !chosen_ips.is_empty() {
                remaining_ips.difference_slice(&chosen_ips);
                tried_ips.union_slice(&chosen_ips);
                for chosen_ip in chosen_ips.into_iter() {
                    retried.switch_endpoint();
                    debug!("Try single IP: {}", chosen_ip);
                    match try_single_ip(chosen_ip, parts, body, extensions, retried, is_endpoints_alternative) {
                        Ok(response) => return Ok(response),
                        Err(SingleTryFlow::TryAgain(err)) => {
                            let (err, ext) = err.split();
                            *extensions = ext;
                            last_error = Some(err);
                            retried.increase_abandoned_endpoints();
                        }
                        Err(SingleTryFlow::DontRetry(err)) => {
                            retried.increase_abandoned_endpoints();
                            return Err(ControlFlow::DontRetry(err));
                        }
                    }
                }
            }
            Err(ControlFlow::TryNext(
                last_error.map(|err| err.with_extensions(take(extensions))),
            ))
        }

        fn try_single_ip(
            ip: IpAddrWithPort,
            parts: &InnerRequestParts<'_>,
            body: &mut SyncRequestBody,
            extensions: &mut Extensions,
            retried: &mut RetriedStatsInfo,
            is_endpoints_alternative: bool,
        ) -> Result<SyncResponse, SingleTryFlow<TryErrorWithExtensions>> {
            try_domain_or_single_ip(
                &DomainOrIpAddr::from(ip),
                parts,
                body,
                take(extensions),
                retried,
                is_endpoints_alternative,
            )
        }
    }

    fn try_domain_or_single_ip(
        domain: &DomainOrIpAddr,
        parts: &InnerRequestParts<'_>,
        body: &mut SyncRequestBody<'_>,
        extensions: Extensions,
        retried: &mut RetriedStatsInfo,
        is_endpoints_alternative: bool,
    ) -> Result<SyncResponse, SingleTryFlow<TryErrorWithExtensions>> {
        match try_domain_or_ip_addr(domain, parts, body, extensions, retried) {
            Ok(response) => Ok(response),
            Err(err) => match err.retry_decision() {
                RetryDecision::TryAlternativeEndpoints if is_endpoints_alternative => {
                    Err(SingleTryFlow::DontRetry(err))
                }
                RetryDecision::DontRetry => {
                    retried.increase_abandoned_ips_of_current_endpoint();
                    Err(SingleTryFlow::DontRetry(err))
                }
                _ => {
                    retried.increase_abandoned_ips_of_current_endpoint();
                    Err(SingleTryFlow::TryAgain(err))
                }
            },
        }
    }
}

#[cfg(feature = "async")]
use super::{
    super::{AsyncRequestBody, AsyncResponse},
    try_domain_or_ip_addr::async_try_domain_or_ip_addr,
    utils::{async_choose, async_resolve},
};

#[cfg(feature = "async")]
pub(super) async fn async_try_endpoints(
    endpoints: &[Endpoint],
    parts: &InnerRequestParts<'_>,
    body: &mut AsyncRequestBody<'_>,
    mut extensions: Extensions,
    tried_ips: &mut IpAddrsSet,
    retried: &mut RetriedStatsInfo,
    is_endpoints_alternative: bool,
) -> Result<AsyncResponse, TryErrorWithExtensions> {
    let mut last_error: Option<TryError> = None;

    for domain_with_port in find_domains_with_port(endpoints) {
        debug!("Try domain with port: {}", domain_with_port);
        match try_domain_with_port(
            domain_with_port,
            tried_ips,
            parts,
            body,
            &mut extensions,
            retried,
            is_endpoints_alternative,
        )
        .await
        {
            Ok(response) => return Ok(response),
            Err(ControlFlow::TryNext(Some(err))) => {
                let (err, ext) = err.split();
                extensions = ext;
                last_error = Some(err);
            }
            Err(ControlFlow::TryNext(None)) => {}
            Err(ControlFlow::DontRetry(err)) => {
                return Err(err);
            }
        }
    }

    let ips = find_ip_addr_with_port(endpoints).copied().collect::<Vec<_>>();
    if !ips.is_empty() {
        debug!("Try IPs with port: {:?}", ips);
        match try_ips(
            &ips,
            tried_ips,
            parts,
            body,
            &mut extensions,
            retried,
            is_endpoints_alternative,
        )
        .await
        {
            Ok(response) => return Ok(response),
            Err(ControlFlow::TryNext(Some(err))) => {
                let (err, ext) = err.split();
                extensions = ext;
                last_error = Some(err);
            }
            Err(ControlFlow::TryNext(None)) => {}
            Err(ControlFlow::DontRetry(err)) => {
                return Err(err);
            }
        }
    }

    return Err(last_error
        .unwrap_or_else(|| no_try_error(retried))
        .with_extensions(extensions));

    async fn try_domain_with_port(
        domain_with_port: &DomainWithPort,
        tried_ips: &mut IpAddrsSet,
        parts: &InnerRequestParts<'_>,
        body: &mut AsyncRequestBody<'_>,
        extensions: &mut Extensions,
        retried: &mut RetriedStatsInfo,
        is_endpoints_alternative: bool,
    ) -> Result<AsyncResponse, ControlFlow<TryErrorWithExtensions>> {
        retried.switch_endpoint();
        return if parts.http_client().http_caller().is_resolved_ip_addrs_supported() {
            debug!("Try domain with resolver: {}", domain_with_port);
            with_resolver(
                domain_with_port,
                tried_ips,
                parts,
                body,
                extensions,
                retried,
                is_endpoints_alternative,
            )
            .await
        } else {
            debug!("Try domain without resolver: {}", domain_with_port);
            without_resolver(
                domain_with_port,
                parts,
                body,
                extensions,
                retried,
                is_endpoints_alternative,
            )
            .await
        }
        .tap_err(|_| retried.increase_abandoned_endpoints());

        async fn with_resolver(
            domain_with_port: &DomainWithPort,
            tried_ips: &mut IpAddrsSet,
            parts: &InnerRequestParts<'_>,
            body: &mut AsyncRequestBody<'_>,
            extensions: &mut Extensions,
            retried: &mut RetriedStatsInfo,
            is_endpoints_alternative: bool,
        ) -> Result<AsyncResponse, ControlFlow<TryErrorWithExtensions>> {
            let mut last_error: Option<TryError> = None;
            let ips = async_resolve(parts, domain_with_port, extensions, retried)
                .await
                .map_err(|err| err.with_extensions(take(extensions)))
                .map_err(Some)
                .map_err(ControlFlow::TryNext)?;
            if !ips.is_empty() {
                let mut remaining_ips = {
                    let mut ips = IpAddrsSet::new(&ips);
                    ips.difference_set(tried_ips);
                    ips
                };
                loop {
                    match try_domain_with_ips(
                        domain_with_port,
                        &mut remaining_ips,
                        tried_ips,
                        parts,
                        body,
                        extensions,
                        retried,
                        is_endpoints_alternative,
                    )
                    .await
                    {
                        Ok(response) => return Ok(response),
                        Err(TryFlow::TryNext(None)) => {
                            break;
                        }
                        Err(TryFlow::TryAgain(err)) => {
                            let (err, ext) = err.split();
                            *extensions = ext;
                            last_error = Some(err);
                        }
                        Err(TryFlow::DontRetry(err)) => {
                            return Err(ControlFlow::DontRetry(err));
                        }
                        Err(TryFlow::TryNext(Some(err))) => {
                            return Err(ControlFlow::TryNext(Some(err)));
                        }
                    }
                }
            }
            Err(ControlFlow::TryNext(
                last_error.map(|err| err.with_extensions(take(extensions))),
            ))
        }

        async fn without_resolver(
            domain_with_port: &DomainWithPort,
            parts: &InnerRequestParts<'_>,
            body: &mut AsyncRequestBody<'_>,
            extensions: &mut Extensions,
            retried: &mut RetriedStatsInfo,
            is_endpoints_alternative: bool,
        ) -> Result<AsyncResponse, ControlFlow<TryErrorWithExtensions>> {
            let domain = DomainOrIpAddr::new_from_domain(domain_with_port.to_owned(), vec![]);
            match async_try_domain_or_ip_addr(&domain, parts, body, take(extensions), retried).await {
                Ok(response) => Ok(response),
                Err(err) => match err.retry_decision() {
                    RetryDecision::TryAlternativeEndpoints if is_endpoints_alternative => {
                        Err(ControlFlow::DontRetry(err))
                    }
                    RetryDecision::DontRetry => {
                        retried.increase_abandoned_ips_of_current_endpoint();
                        Err(ControlFlow::DontRetry(err))
                    }
                    _ => {
                        retried.increase_abandoned_ips_of_current_endpoint();
                        Err(ControlFlow::TryNext(Some(err)))
                    }
                },
            }
        }

        #[allow(clippy::too_many_arguments)]
        async fn try_domain_with_ips(
            domain_with_port: &DomainWithPort,
            remaining_ips: &mut IpAddrsSet,
            tried_ips: &mut IpAddrsSet,
            parts: &InnerRequestParts<'_>,
            body: &mut AsyncRequestBody<'_>,
            extensions: &mut Extensions,
            retried: &mut RetriedStatsInfo,
            is_endpoints_alternative: bool,
        ) -> Result<AsyncResponse, TryFlow<TryErrorWithExtensions>> {
            let chosen_ips = match remaining_ips.remains() {
                ips if !ips.is_empty() => async_choose(parts, &ips, extensions, retried)
                    .await
                    .map_err(|err| err.with_extensions(take(extensions)))
                    .map_err(TryFlow::TryAgain)?,
                _ => vec![],
            };
            if chosen_ips.is_empty() {
                Err(TryFlow::TryNext(None))
            } else {
                remaining_ips.difference_slice(&chosen_ips);
                tried_ips.union_slice(&chosen_ips);
                retried.switch_ips();
                let chosen_ips = IpAddrs::from(chosen_ips);
                debug!("Try domain with IPs: {}({})", domain_with_port, chosen_ips);
                match try_domain_or_single_ip(
                    &DomainOrIpAddr::new_from_domain(domain_with_port.to_owned(), chosen_ips.into()),
                    parts,
                    body,
                    take(extensions),
                    retried,
                    is_endpoints_alternative,
                )
                .await
                {
                    Ok(response) => Ok(response),
                    Err(SingleTryFlow::TryAgain(err)) => Err(TryFlow::TryAgain(err)),
                    Err(SingleTryFlow::DontRetry(err)) => Err(TryFlow::DontRetry(err)),
                }
            }
        }
    }

    async fn try_ips(
        ips: &[IpAddrWithPort],
        tried_ips: &mut IpAddrsSet,
        parts: &InnerRequestParts<'_>,
        body: &mut AsyncRequestBody<'_>,
        extensions: &mut Extensions,
        retried: &mut RetriedStatsInfo,
        is_endpoints_alternative: bool,
    ) -> Result<AsyncResponse, ControlFlow<TryErrorWithExtensions>> {
        let mut last_error: Option<TryError> = None;

        let mut remaining_ips = {
            let mut ips = IpAddrsSet::new(ips);
            ips.difference_set(tried_ips);
            ips
        };
        loop {
            debug!("Try IPs: {}", remaining_ips);
            match try_remaining_ips(
                &mut remaining_ips,
                tried_ips,
                parts,
                body,
                extensions,
                retried,
                is_endpoints_alternative,
            )
            .await
            {
                Ok(response) => return Ok(response),
                Err(ControlFlow::TryNext(Some(err))) => {
                    let (err, ext) = err.split();
                    *extensions = ext;
                    last_error = Some(err);
                }
                Err(ControlFlow::TryNext(None)) => {
                    break;
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        return Err(ControlFlow::TryNext(
            last_error.map(|err| err.with_extensions(take(extensions))),
        ));

        async fn try_remaining_ips(
            remaining_ips: &mut IpAddrsSet,
            tried_ips: &mut IpAddrsSet,
            parts: &InnerRequestParts<'_>,
            body: &mut AsyncRequestBody<'_>,
            extensions: &mut Extensions,
            retried: &mut RetriedStatsInfo,
            is_endpoints_alternative: bool,
        ) -> Result<AsyncResponse, ControlFlow<TryErrorWithExtensions>> {
            let mut last_error: Option<TryError> = None;
            let chosen_ips = match remaining_ips.remains() {
                ips if !ips.is_empty() => async_choose(parts, &ips, extensions, retried)
                    .await
                    .map_err(|err| err.with_extensions(take(extensions)))
                    .map_err(Some)
                    .map_err(ControlFlow::TryNext)?,
                _ => vec![],
            };
            if !chosen_ips.is_empty() {
                remaining_ips.difference_slice(&chosen_ips);
                tried_ips.union_slice(&chosen_ips);
                for chosen_ip in chosen_ips.into_iter() {
                    retried.switch_endpoint();
                    debug!("Try single IP: {}", chosen_ip);
                    match try_single_ip(chosen_ip, parts, body, extensions, retried, is_endpoints_alternative).await {
                        Ok(response) => return Ok(response),
                        Err(SingleTryFlow::TryAgain(err)) => {
                            let (err, ext) = err.split();
                            *extensions = ext;
                            last_error = Some(err);
                            retried.increase_abandoned_endpoints();
                        }
                        Err(SingleTryFlow::DontRetry(err)) => {
                            retried.increase_abandoned_endpoints();
                            return Err(ControlFlow::DontRetry(err));
                        }
                    }
                }
            }
            Err(ControlFlow::TryNext(
                last_error.map(|err| err.with_extensions(take(extensions))),
            ))
        }

        async fn try_single_ip(
            ip: IpAddrWithPort,
            parts: &InnerRequestParts<'_>,
            body: &mut AsyncRequestBody<'_>,
            extensions: &mut Extensions,
            retried: &mut RetriedStatsInfo,
            is_endpoints_alternative: bool,
        ) -> Result<AsyncResponse, SingleTryFlow<TryErrorWithExtensions>> {
            try_domain_or_single_ip(
                &DomainOrIpAddr::from(ip),
                parts,
                body,
                take(extensions),
                retried,
                is_endpoints_alternative,
            )
            .await
        }
    }

    async fn try_domain_or_single_ip(
        domain: &DomainOrIpAddr,
        parts: &InnerRequestParts<'_>,
        body: &mut AsyncRequestBody<'_>,
        extensions: Extensions,
        retried: &mut RetriedStatsInfo,
        is_endpoints_alternative: bool,
    ) -> Result<AsyncResponse, SingleTryFlow<TryErrorWithExtensions>> {
        match async_try_domain_or_ip_addr(domain, parts, body, extensions, retried).await {
            Ok(response) => Ok(response),
            Err(err) => match err.retry_decision() {
                RetryDecision::TryAlternativeEndpoints if is_endpoints_alternative => {
                    Err(SingleTryFlow::DontRetry(err))
                }
                RetryDecision::DontRetry => {
                    retried.increase_abandoned_ips_of_current_endpoint();
                    Err(SingleTryFlow::DontRetry(err))
                }
                _ => {
                    retried.increase_abandoned_ips_of_current_endpoint();
                    Err(SingleTryFlow::TryAgain(err))
                }
            },
        }
    }
}

enum ControlFlow<E> {
    TryNext(Option<E>),
    DontRetry(E),
}

enum TryFlow<E> {
    TryNext(Option<E>),
    TryAgain(E),
    DontRetry(E),
}

enum SingleTryFlow<E> {
    TryAgain(E),
    DontRetry(E),
}

fn no_try_error(retried: &RetriedStatsInfo) -> TryError {
    TryError::new(
        ResponseError::new_with_msg(ResponseErrorKind::NoTry, "None endpoint is tried").retried(retried),
        RetryDecision::DontRetry.into(),
    )
}
