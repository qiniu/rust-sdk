mod domain_or_ip_addr;
mod error;
mod ip_addrs_set;
mod utils;

use super::{
    super::{DomainWithPort, Endpoint, IpAddrWithPort},
    APIResult, Authorization, AuthorizationError, ChooserFeedback, Request,
    RequestWithoutEndpoints, ResponseError, ResponseErrorKind, ResponseInfo, RetriedStatsInfo,
    RetryResult, SimplifiedCallbackContext, SyncResponse,
};
pub use domain_or_ip_addr::DomainOrIpAddr;
use error::{ErrorResponseBody, TryError, TryErrorWithExtensions};
use ip_addrs_set::IpAddrsSet;
use qiniu_http::{
    Extensions, HeaderName, HeaderValue, Request as HTTPRequest,
    ResponseErrorKind as HTTPResponseErrorKind, StatusCode, TransferProgressInfo,
};
use serde_json::from_slice as parse_json_from_slice;
use std::{mem::take, result::Result, thread::sleep, time::Duration};
use utils::{
    call_after_request_signed_callbacks, call_after_retry_delay_callbacks,
    call_before_request_signed_callbacks, call_before_retry_delay_callbacks,
    call_domain_resolved_callbacks, call_error_callbacks, call_ips_chosen_callbacks,
    call_success_callbacks, call_to_choose_ips_callbacks, call_to_resolve_domain_callbacks,
    extract_ips_from, find_domains_with_port, find_ip_addr_with_port, make_request, make_url,
};

#[cfg(feature = "async")]
use {super::AsyncResponse, async_std::task::block_on, futures_timer::Delay as AsyncDelay};

const X_REQ_ID_HEADER_NAME: &str = "x-reqid";

macro_rules! create_request_call_fn {
    ($method_name:ident, $return_type:ident, $into_endpoints_method:ident, $sign_method:ident, $call_method:ident, $resolve_method:ident, $choose_method:ident, $feedback_method:ident, $sleep_method:path, $new_response_info:path, $block:ident, $blocking_block:ident $(, $async:ident)?) => {
        pub(super) $($async)? fn $method_name(request: Request<'_>) -> APIResult<$return_type> {
            let (request, into_endpoints, service_name, extensions) = request.split();
            let endpoints = $block!({ into_endpoints.$into_endpoints_method(service_name) })?;
            let mut tried_ips = IpAddrsSet::default();
            let mut retried = RetriedStatsInfo::default();

            return match $block!({ try_new_endpoints(endpoints.endpoints(), &request, extensions, &mut tried_ips, &mut retried) }) {
                Ok(response) => Ok(response),
                Err(err)
                    if err.retry_result() == RetryResult::TryOldEndpoints
                        && !endpoints.old_endpoints().is_empty() =>
                {
                    let (_, extensions) = err.split();
                    retried.switch_to_old_endpoints();
                    $block!({ try_old_endpoints(endpoints.old_endpoints(), &request, extensions, &mut tried_ips, &mut retried) })
                }
                Err(err) => Err(err.into_response_error()),
            };

            type TryResult = Result<$return_type, TryErrorWithExtensions>;
            type _TryResult = Result<$return_type, TryError>;

            #[inline]
            $($async)? fn try_new_endpoints(
                endpoints: &[Endpoint],
                request: &RequestWithoutEndpoints<'_>,
                extensions: Extensions,
                tried_ips: &mut IpAddrsSet,
                retried: &mut RetriedStatsInfo,
            ) -> TryResult {
                $block!({ try_endpoints(endpoints, request, extensions, tried_ips, retried, true) })
            }

            #[inline]
            $($async)? fn try_old_endpoints(
                endpoints: &[Endpoint],
                request: &RequestWithoutEndpoints<'_>,
                extensions: Extensions,
                tried_ips: &mut IpAddrsSet,
                retried: &mut RetriedStatsInfo,
            ) -> APIResult<$return_type> {
                $block!({ try_endpoints(endpoints, request, extensions, tried_ips, retried, false) }).map_err(|err| err.into_response_error())
            }

            $($async)? fn try_endpoints(
                endpoints: &[Endpoint],
                request: &RequestWithoutEndpoints<'_>,
                mut extensions: Extensions,
                tried_ips: &mut IpAddrsSet,
                retried: &mut RetriedStatsInfo,
                is_endpoints_old: bool,
            ) -> TryResult {
                let mut last_error: Option<TryError> = None;
                macro_rules! try_endpoint {
                    ($endpoint:expr) => {
                        let ext = take(&mut extensions);
                        match $block!({ try_endpoint($endpoint, request, ext, retried) }) {
                            Ok(response) => return Ok(response),
                            Err(err) => {
                                match err.retry_result() {
                                    RetryResult::TryOldEndpoints if is_endpoints_old => return Err(err),
                                    RetryResult::DontRetry => {
                                        retried.increase_abandoned_ips_of_current_endpoint();
                                        return Err(err);
                                    }
                                    _ => {
                                        retried.increase_abandoned_ips_of_current_endpoint();
                                        let (err, ext) = err.split();
                                        extensions = ext;
                                        last_error = Some(err);
                                    }
                                }
                            },
                        }
                    };
                }

                for domain_with_port in find_domains_with_port(endpoints) {
                    retried.switch_endpoint();
                    let ips = $block!({ resolve_domain(request, domain_with_port, &mut extensions) })
                        .map_err(|err| err.with_extensions(take(&mut extensions)))?;
                    if ips.is_empty() {
                        retried.increase_abandoned_endpoints();
                    } else {
                        let mut remaining_ips = {
                            let mut ips = IpAddrsSet::new(&ips);
                            ips.difference_set(tried_ips);
                            ips
                        };
                        loop {
                            let chosen_ips = match remaining_ips.remains() {
                                ips if !ips.is_empty() => {
                                    $block!({ choose(request, &ips, &mut extensions) })
                                        .map_err(|err| err.with_extensions(take(&mut extensions)))?
                                },
                                _ => vec![],
                            };
                            if chosen_ips.is_empty() {
                                retried.increase_abandoned_endpoints();
                                break;
                            } else {
                                remaining_ips.difference_slice(&chosen_ips);
                                tried_ips.union_slice(&chosen_ips);
                                retried.switch_ips();
                                let domain =
                                    DomainOrIpAddr::new_from_domain(domain_with_port.to_owned(), chosen_ips);
                                try_endpoint!(&domain);
                            }
                        }
                    }
                }

                let ips = find_ip_addr_with_port(endpoints)
                    .cloned()
                    .collect::<Vec<_>>();
                if !ips.is_empty() {
                    let mut remaining_ips = {
                        let mut ips = IpAddrsSet::new(&ips);
                        ips.difference_set(tried_ips);
                        ips
                    };
                    loop {
                        let chosen_ips = match remaining_ips.remains() {
                            ips if !ips.is_empty() => {
                                $block!({ choose(request, &remaining_ips.remains(), &mut extensions) })
                                    .map_err(|err| err.with_extensions(take(&mut extensions)))?
                            },
                            _ => vec![],
                        };
                        if chosen_ips.is_empty() {
                            retried.increase_abandoned_endpoints();
                            break;
                        } else {
                            remaining_ips.difference_slice(&chosen_ips);
                            tried_ips.union_slice(&chosen_ips);
                            for chosen_ip in chosen_ips.into_iter() {
                                retried.switch_endpoint();
                                retried.switch_ips();
                                let ip_addr = DomainOrIpAddr::from(chosen_ip);
                                try_endpoint!(&ip_addr);
                                retried.increase_abandoned_endpoints();
                            }
                        }
                    }
                }

                return Err(last_error.unwrap_or_else(no_try_error).with_extensions(extensions));
            }

            $($async)? fn try_endpoint(
                domain_or_ip: &DomainOrIpAddr,
                request: &RequestWithoutEndpoints<'_>,
                mut extensions: Extensions,
                retried: &mut RetriedStatsInfo,
            ) -> TryResult {
                let (url, resolved_ips) = make_url(domain_or_ip, request)
                    .map_err(|err| err.with_extensions(take(&mut extensions)))?;
                let mut built_request = make_request(url, request, extensions, &resolved_ips);
                call_before_request_signed_callbacks(request, &mut built_request, retried)
                    .map_err(|err| err.with_request(&mut built_request))?;
                $block!({ sign_request(&mut built_request, request.authorization()) })
                    .map_err(|err| err.with_request(&mut built_request))?;
                call_after_request_signed_callbacks(request, &mut built_request, retried)
                    .map_err(|err| err.with_request(&mut built_request))?;

                let on_uploading_progress = |info: &TransferProgressInfo| -> bool {
                    request.call_uploading_progress_callbacks(request, info)
                };
                if request.uploading_progress_callbacks_count() > 0 {
                    *built_request.on_uploading_progress_mut() = Some(&on_uploading_progress);
                }
                let on_receive_response_status = |status_code: StatusCode| -> bool {
                    request.call_receive_response_status_callbacks(request, status_code)
                };
                if request.receive_response_status_callbacks_count() > 0 {
                    *built_request.on_receive_response_status_mut() = Some(&on_receive_response_status);
                }
                let on_receive_response_header = |name: &HeaderName, value: &HeaderValue| -> bool {
                    request.call_receive_response_header_callbacks(request, name, value)
                };
                if request.receive_response_header_callbacks_count() > 0 {
                    *built_request.on_receive_response_header_mut() = Some(&on_receive_response_header);
                }

                let extracted_ips = extract_ips_from(&domain_or_ip);
                match $block!({ do_request(request, &mut built_request, retried) }) {
                    Ok(response) => {
                        let feedback = ChooserFeedback::new(&extracted_ips, retried, response.metrics(), None);
                        $block!({ request.http_client().chooser().$feedback_method(feedback) });
                        Ok(response)
                    },
                    Err(err) => {
                        let feedback = ChooserFeedback::new(&extracted_ips, retried, err.response_error().metrics(), Some(err.response_error()));
                        $block!({ request.http_client().chooser().$feedback_method(feedback) });
                        Err(err.with_request(&mut built_request))
                    }
                }
            }

            $($async)? fn do_request(
                request: &RequestWithoutEndpoints<'_>,
                built_request: &mut HTTPRequest<'_>,
                retried: &mut RetriedStatsInfo,
            ) -> _TryResult {
                loop {
                    let response = $block!({
                        request
                            .http_client()
                            .http_caller()
                            .$call_method(&built_request)
                        })
                        .map_err(ResponseError::from)
                        .map($return_type::new)
                        .and_then(|response| $blocking_block!({ judge(response) }) )
                        .map_err(|response_error| {
                            let retry_result = request.http_client().request_retrier().retry(
                                built_request,
                                request.idempotent(),
                                &response_error,
                                retried,
                            );
                            retried.increase();
                            TryError::new(response_error, retry_result)
                        });
                    match response {
                        Ok(response) => {
                            call_success_callbacks(
                                request,
                                built_request,
                                retried,
                                &$new_response_info(&response),
                            )?;
                            return Ok(response);
                        }
                        Err(err) => {
                            call_error_callbacks(request, built_request, retried, err.response_error())?;
                            match err.retry_result() {
                                retry_result @ RetryResult::RetryRequest
                                | retry_result @ RetryResult::Throttled
                                | retry_result @ RetryResult::TryNextServer => {
                                    let delay = request
                                        .http_client()
                                        .retry_delay_policy()
                                        .delay_before_next_retry(
                                            built_request,
                                            err.retry_result(),
                                            err.response_error(),
                                            retried,
                                        );
                                    call_before_retry_delay_callbacks(
                                        request,
                                        built_request,
                                        retried,
                                        delay,
                                    )?;
                                    if delay > Duration::new(0, 0) {
                                        $block!({ $sleep_method(delay) });
                                    }
                                    call_after_retry_delay_callbacks(
                                        request,
                                        built_request,
                                        retried,
                                        delay,
                                    )?;
                                    match retry_result {
                                        RetryResult::RetryRequest | RetryResult::Throttled => continue,
                                        _ => return Err(err),
                                    }
                                }
                                _ => return Err(err),
                            }
                        }
                    }
                }
            }

            $($async)? fn sign_request(
                request: &mut HTTPRequest<'_>,
                authorization: Option<&Authorization>,
            ) -> Result<(), TryError> {
                if let Some(authorization) = authorization {
                    $block!({ authorization.$sign_method(request) }).map_err(|err| match err {
                        AuthorizationError::IOError(err) => TryError::new(
                            ResponseError::new(
                                ResponseErrorKind::HTTPError(HTTPResponseErrorKind::LocalIOError),
                                err,
                            ),
                            RetryResult::DontRetry,
                        ),
                        AuthorizationError::UrlParseError(err) => TryError::new(
                            ResponseError::new(
                                ResponseErrorKind::HTTPError(HTTPResponseErrorKind::InvalidURL),
                                err,
                            ),
                            RetryResult::TryNextServer,
                        ),
                    })?;
                }
                Ok(())
            }

            $($async)? fn judge(response: $return_type) -> APIResult<$return_type> {
                if response
                    .headers()
                    .get(&HeaderName::from_static(X_REQ_ID_HEADER_NAME))
                    .is_none()
                {
                    return Err(ResponseError::new(
                        ResponseErrorKind::MaliciousResponse,
                        format!(
                            "cannot find {} header from response, might be malicious response",
                            X_REQ_ID_HEADER_NAME
                        ),
                    ));
                }
                match response.status_code().as_u16() {
                    0..=199 | 300..=399 => Err(ResponseError::new(
                        ResponseErrorKind::UnexpectedStatusCode(response.status_code()),
                        format!("status code {} is unexpected", response.status_code()),
                    )),
                    200..=299 => Ok(response),
                    _ => {
                        let status_code = response.status_code();
                        let error_response_body: Vec<u8> = $block!({ response.fulfill() })?
                            .into_body();
                        let error_response_body: ErrorResponseBody =
                            parse_json_from_slice(&error_response_body).map_err(|err| {
                                ResponseError::new(ResponseErrorKind::ParseResponseError, err)
                            })?;
                        Err(ResponseError::new(
                            ResponseErrorKind::StatusCodeError(status_code),
                            error_response_body.into_error(),
                        ))
                    }
                }
            }

            $($async)? fn resolve_domain(
                request: &RequestWithoutEndpoints<'_>,
                domain_with_port: &DomainWithPort,
                extensions: &mut Extensions,
            ) -> Result<Vec<IpAddrWithPort>, TryError> {
                call_to_resolve_domain_callbacks(request, domain_with_port.domain(), extensions)?;
                let answers = $block!({
                    request
                        .http_client()
                        .resolver()
                        .$resolve_method(domain_with_port.domain())
                }).map_err(|err| TryError::new(err, RetryResult::TryNextServer))?;
                call_domain_resolved_callbacks(request, domain_with_port.domain(), &answers, extensions)?;
                Ok(answers.into_ip_addrs().into_iter().map(|&ip| IpAddrWithPort::new(ip, domain_with_port.port())).collect())
            }

            $($async)? fn choose(
                request: &RequestWithoutEndpoints<'_>,
                ips: &[IpAddrWithPort],
                extensions: &mut Extensions,
            ) -> Result<Vec<IpAddrWithPort>, TryError> {
                call_to_choose_ips_callbacks(request, ips, extensions)?;
                let chosen_ips = $block!({
                    request
                        .http_client()
                        .chooser()
                        .$choose_method(ips)
                });
                call_ips_chosen_callbacks(request, ips, &chosen_ips, extensions)?;
                Ok(chosen_ips)
            }
        }
    };
}

macro_rules! sync_block {
    ($block:block) => {{
        $block
    }};
}

#[cfg(feature = "async")]
macro_rules! async_block {
    ($block:block) => {
        $block.await
    };
}

#[cfg(feature = "async")]
macro_rules! blocking_async_block {
    ($block:block) => {
        block_on(async { $block.await })
    };
}

create_request_call_fn!(
    request_call,
    SyncResponse,
    into_endpoints,
    sign,
    call,
    resolve,
    choose,
    feedback,
    sleep,
    ResponseInfo::new_from_sync,
    sync_block,
    sync_block
);

#[inline]
fn no_try_error() -> TryError {
    TryError::new(
        ResponseError::new(ResponseErrorKind::NoTry, "None resolver is tried"),
        RetryResult::DontRetry,
    )
}

#[cfg(feature = "async")]
async fn async_sleep(dur: Duration) {
    AsyncDelay::new(dur).await
}

#[cfg(feature = "async")]
create_request_call_fn!(
    async_request_call,
    AsyncResponse,
    async_into_endpoints,
    async_sign,
    async_call,
    async_resolve,
    async_choose,
    async_feedback,
    async_sleep,
    ResponseInfo::new_from_async,
    async_block,
    blocking_async_block,
    async
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        client::chooser::DirectChooser,
        credential::{Credential, CredentialProvider, StaticCredentialProvider},
        test_utils::{
            chaotic_up_domains_region, make_dumb_resolver, make_error_response_client_builder,
            make_fixed_response_client_builder, make_random_resolver, single_up_domain_region,
        },
        Authorization, Chooser, ErrorRetrier, IpChooser, LimitedRetrier, ServiceName,
        NO_DELAY_POLICY,
    };
    use qiniu_http::HeaderMap;
    use std::{
        collections::{HashMap, HashSet},
        error::Error,
        net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
        result::Result,
        sync::{
            atomic::{AtomicUsize, Ordering::Relaxed},
            Arc, Mutex,
        },
    };

    #[test]
    fn test_call_endpoints_selection() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let client = make_error_response_client_builder(
            HTTPResponseErrorKind::ConnectError,
            "Fake Connect Error",
        )
        .chooser(Box::new(DirectChooser))
        .resolver(Box::new(make_random_resolver()))
        .request_retrier(Box::new(ErrorRetrier))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .build();

        let urls_visited = Arc::new(Mutex::new(Vec::new()));
        let domain_resolved = Arc::new(Mutex::new(Vec::new()));
        let err = client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_to_resolve_domain(Box::new({
                let domain_resolved = domain_resolved.to_owned();
                move |_, domain| {
                    domain_resolved.lock().unwrap().push(domain.to_owned());
                    true
                }
            }))
            .on_after_request_signed(Box::new({
                let urls_visited = urls_visited.to_owned();
                move |context| {
                    urls_visited.lock().unwrap().push(context.url().to_string());
                    true
                }
            }))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HTTPResponseErrorKind::ConnectError)
        );
        let domain_resolved = Arc::try_unwrap(domain_resolved)
            .unwrap()
            .into_inner()
            .unwrap();
        assert_eq!(
            &domain_resolved,
            &[
                "fakedomain.withoutport.com".to_owned(),
                "fakedomain.withport.com".to_owned()
            ]
        );
        let urls_visited = Arc::try_unwrap(urls_visited).unwrap().into_inner().unwrap();
        assert_eq!(
            &urls_visited,
            &[
                "https://fakedomain.withoutport.com/".to_owned(),
                "https://fakedomain.withport.com:8080/".to_owned(),
                "https://192.168.1.1/".to_owned(),
                "https://[::ffff:192.10.2.255]/".to_owned(),
                "https://[::ffff:192.11.2.255]:8081/".to_owned(),
                "https://192.168.1.2:8080/".to_owned(),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_call_all_frozen_endpoints_selection() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let err = ResponseError::new(
            HTTPResponseErrorKind::ConnectError.into(),
            "Fake Connect Error",
        );
        let chooser = IpChooser::default();
        chooser.feedback(ChooserFeedback::new(
            &[
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into(),
                IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff)).into(),
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 2), 8080)).into(),
                SocketAddr::V6(SocketAddrV6::new(
                    Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00b, 0x2ff),
                    8081,
                    0,
                    0,
                ))
                .into(),
            ],
            &RetriedStatsInfo::default(),
            None,
            Some(&err),
        ));

        let client = make_error_response_client_builder(
            HTTPResponseErrorKind::ConnectError,
            "Fake Connect Error",
        )
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .resolver(Box::new(make_random_resolver()))
        .chooser(Box::new(chooser))
        .request_retrier(Box::new(ErrorRetrier))
        .build();

        let urls_visited = Arc::new(Mutex::new(Vec::new()));
        let domain_resolved = Arc::new(Mutex::new(Vec::new()));
        let err = client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_to_resolve_domain(Box::new({
                let domain_resolved = domain_resolved.to_owned();
                move |_, domain| {
                    domain_resolved.lock().unwrap().push(domain.to_owned());
                    true
                }
            }))
            .on_after_request_signed(Box::new({
                let urls_visited = urls_visited.to_owned();
                move |context| {
                    urls_visited.lock().unwrap().push(context.url().to_string());
                    true
                }
            }))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::HTTPError(HTTPResponseErrorKind::ConnectError)
        );
        let domain_resolved = Arc::try_unwrap(domain_resolved)
            .unwrap()
            .into_inner()
            .unwrap();
        assert_eq!(
            &domain_resolved,
            &[
                "fakedomain.withoutport.com".to_owned(),
                "fakedomain.withport.com".to_owned()
            ]
        );
        let urls_visited = Arc::try_unwrap(urls_visited).unwrap().into_inner().unwrap();
        assert_eq!(
            &urls_visited,
            &[
                "https://fakedomain.withoutport.com/".to_owned(),
                "https://fakedomain.withport.com:8080/".to_owned(),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_call_switch_to_old_endpoints() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let client =
            make_error_response_client_builder(HTTPResponseErrorKind::SSLError, "Fake SSL Error")
                .resolver(Box::new(make_random_resolver()))
                .chooser(Box::new(DirectChooser))
                .retry_delay_policy(Box::new(NO_DELAY_POLICY))
                .request_retrier(Box::new(ErrorRetrier))
                .build();

        let urls_visited = Arc::new(Mutex::new(Vec::new()));
        let domain_resolved = Arc::new(Mutex::new(Vec::new()));
        let err = client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_before_retry_delay(Box::new(|_, _| panic!("Should not retry")))
            .on_to_resolve_domain(Box::new({
                let domain_resolved = domain_resolved.to_owned();
                move |_, domain| {
                    domain_resolved.lock().unwrap().push(domain.to_owned());
                    true
                }
            }))
            .on_after_request_signed(Box::new({
                let urls_visited = urls_visited.to_owned();
                let retried = Arc::new(AtomicUsize::new(0));
                move |context| {
                    let retried = retried.fetch_add(1, Relaxed);
                    urls_visited.lock().unwrap().push(context.url().to_string());
                    assert_eq!(context.retried().retried_total(), retried);
                    assert_eq!(context.retried().retried_on_current_endpoint(), 0);
                    assert_eq!(context.retried().retried_on_current_ips(), 0);
                    assert_eq!(
                        context.retried().abandoned_endpoints(),
                        retried.saturating_sub(1)
                    );
                    if retried > 0 {
                        assert!(context.retried().switched_to_old_endpoints());
                    } else {
                        assert!(!context.retried().switched_to_old_endpoints());
                    }
                    true
                }
            }))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HTTPResponseErrorKind::SSLError)
        );
        let domain_resolved = Arc::try_unwrap(domain_resolved)
            .unwrap()
            .into_inner()
            .unwrap();
        assert_eq!(
            &domain_resolved,
            &[
                "fakedomain.withoutport.com".to_owned(),
                "old_fakedomain.withoutport.com".to_owned(),
                "old_fakedomain.withport.com".to_owned(),
            ]
        );
        let urls_visited = Arc::try_unwrap(urls_visited).unwrap().into_inner().unwrap();
        assert_eq!(
            &urls_visited,
            &[
                "https://fakedomain.withoutport.com/".to_owned(),
                "https://old_fakedomain.withoutport.com/".to_owned(),
                "https://old_fakedomain.withport.com:8080/".to_owned(),
                "https://192.168.2.1/".to_owned(),
                "https://[::ffff:208.10.2.255]/".to_owned(),
                "https://[::ffff:208.11.2.255]:8081/".to_owned(),
                "https://192.168.2.2:8080/".to_owned(),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_call_single_endpoint_retry() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let always_retry_client = make_error_response_client_builder(
            HTTPResponseErrorKind::TimeoutError,
            "Fake Timeout Error",
        )
        .resolver(Box::new(make_random_resolver()))
        .chooser(Box::new(DirectChooser))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(LimitedRetrier::new(ErrorRetrier, 3)))
        .build();

        let retried = Arc::new(AtomicUsize::new(0));
        let err = always_retry_client
            .post(ServiceName::Up, &single_up_domain_region())
            .on_before_retry_delay({
                let retried = retried.to_owned();
                Box::new(move |context, _| {
                    assert_eq!(
                        "https://fakedomain.withport.com:8080/",
                        &context.url().to_string(),
                    );
                    let retried = retried.fetch_add(1, Relaxed) + 1;
                    assert_eq!(context.retried().retried_total(), retried);
                    assert_eq!(context.retried().retried_on_current_endpoint(), retried);
                    assert_eq!(context.retried().retried_on_current_ips(), retried);
                    assert_eq!(context.retried().abandoned_endpoints(), 0);
                    true
                })
            })
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HTTPResponseErrorKind::TimeoutError)
        );

        let headers = {
            let mut headers = HeaderMap::default();
            headers.insert(
                HeaderName::from_static(X_REQ_ID_HEADER_NAME),
                HeaderValue::from_static("fake_req_id"),
            );
            headers
        };
        let always_throttled_client = make_fixed_response_client_builder(
            StatusCode::from_u16(509)?,
            headers,
            b"{\"error\":\"Fake Throttled Error\"}".to_vec(),
        )
        .resolver(Box::new(make_random_resolver()))
        .chooser(Box::new(DirectChooser))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(LimitedRetrier::new(ErrorRetrier, 3)))
        .build();

        let err = always_throttled_client
            .post(ServiceName::Up, &single_up_domain_region())
            .on_before_retry_delay({
                retried.store(0, Relaxed);
                Box::new(move |context, _| {
                    assert_eq!(
                        "https://fakedomain.withport.com:8080/",
                        &context.url().to_string(),
                    );
                    let retried = retried.fetch_add(1, Relaxed) + 1;
                    assert_eq!(context.retried().retried_total(), retried);
                    assert_eq!(context.retried().retried_on_current_endpoint(), retried);
                    assert_eq!(context.retried().retried_on_current_ips(), retried);
                    assert_eq!(context.retried().abandoned_endpoints(), 0);
                    true
                })
            })
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::StatusCodeError(StatusCode::from_u16(509)?)
        );
        assert_eq!(&err.to_string(), "Fake Throttled Error");

        Ok(())
    }

    #[test]
    fn test_call_retry_next() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let always_try_next_client = make_error_response_client_builder(
            HTTPResponseErrorKind::UnknownHostError,
            "Test Unknown Host Error",
        )
        .resolver(Box::new(make_random_resolver()))
        .chooser(Box::new(DirectChooser))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(LimitedRetrier::new(ErrorRetrier, 3)))
        .build();

        let retry_urls = [
            "https://fakedomain.withoutport.com/".to_owned(),
            "https://fakedomain.withport.com:8080/".to_owned(),
            "https://192.168.1.1/".to_owned(),
            "https://[::ffff:192.10.2.255]/".to_owned(),
            "https://[::ffff:192.11.2.255]:8081/".to_owned(),
            "https://192.168.1.2:8080/".to_owned(),
        ];
        let err = always_try_next_client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_before_retry_delay({
                let retried = Arc::new(AtomicUsize::new(0));
                Box::new(move |context, _| {
                    let retried = retried.fetch_add(1, Relaxed);
                    assert_eq!(&context.url().to_string(), retry_urls.get(retried).unwrap());
                    assert_eq!(context.retried().retried_total(), retried + 1);
                    assert_eq!(context.retried().retried_on_current_endpoint(), 1);
                    assert_eq!(context.retried().retried_on_current_ips(), 1);
                    assert_eq!(context.retried().abandoned_endpoints(), retried);
                    true
                })
            })
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HTTPResponseErrorKind::UnknownHostError)
        );

        Ok(())
    }

    #[test]
    fn test_call_dont_retry() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let always_dont_retry_client = make_error_response_client_builder(
            HTTPResponseErrorKind::LocalIOError,
            "Test Local IO Error",
        )
        .resolver(Box::new(make_random_resolver()))
        .chooser(Box::new(DirectChooser))
        .build();

        let err = always_dont_retry_client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_before_retry_delay(Box::new(|_, _| panic!("Should never retry")))
            .on_after_request_signed(Box::new(|context| {
                assert_eq!(
                    &context.url().to_string(),
                    "https://fakedomain.withoutport.com/"
                );
                assert_eq!(context.retried().retried_total(), 0);
                true
            }))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HTTPResponseErrorKind::LocalIOError)
        );

        Ok(())
    }

    #[test]
    fn test_call_request_signature() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let always_retry_client =
            make_error_response_client_builder(HTTPResponseErrorKind::SendError, "Test Send Error")
                .resolver(Box::new(make_random_resolver()))
                .chooser(Box::new(DirectChooser))
                .retry_delay_policy(Box::new(NO_DELAY_POLICY))
                .request_retrier(Box::new(LimitedRetrier::new(ErrorRetrier, 3)))
                .build();
        let credential: Arc<dyn CredentialProvider> = Arc::new(StaticCredentialProvider::new(
            Credential::new("abcdefghklmnopq", "012345678901234567890"),
        ));
        let signed_urls = Arc::new(Mutex::new(HashSet::new()));

        {
            let err = always_retry_client
                .post(ServiceName::Up, &chaotic_up_domains_region())
                .authorization(Authorization::v2(credential.to_owned()))
                .on_before_request_signed(Box::new(|context| {
                    assert!(context
                        .headers()
                        .get(&HeaderName::from_static("authorization"))
                        .is_none());
                    true
                }))
                .on_after_request_signed(Box::new({
                    let signed_urls = signed_urls.to_owned();
                    move |context| {
                        signed_urls
                            .lock()
                            .unwrap()
                            .insert(context.url().to_string());
                        assert!(context
                            .headers()
                            .get(&HeaderName::from_static("authorization"))
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .starts_with("Qiniu "));
                        true
                    }
                }))
                .call()
                .unwrap_err();
            assert_eq!(
                err.kind(),
                ResponseErrorKind::from(HTTPResponseErrorKind::SendError)
            );
        }

        {
            signed_urls.lock().unwrap().clear();
            let err = always_retry_client
                .post(ServiceName::Up, &chaotic_up_domains_region())
                .authorization(Authorization::v1(credential))
                .on_before_request_signed(Box::new(|context| {
                    assert!(context
                        .headers()
                        .get(&HeaderName::from_static("authorization"))
                        .is_none());
                    true
                }))
                .on_after_request_signed(Box::new({
                    move |context| {
                        signed_urls
                            .lock()
                            .unwrap()
                            .insert(context.url().to_string());
                        assert!(context
                            .headers()
                            .get(&HeaderName::from_static("authorization"))
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .starts_with("QBox "));
                        true
                    }
                }))
                .call()
                .unwrap_err();
            assert_eq!(
                err.kind(),
                ResponseErrorKind::from(HTTPResponseErrorKind::SendError)
            );
        }

        Ok(())
    }

    #[test]
    fn test_call_malicious_response() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let always_malicious_client = make_fixed_response_client_builder(
            StatusCode::from_u16(200)?,
            Default::default(),
            b"<p>Hello world!</p>".to_vec(),
        )
        .resolver(Box::new(make_random_resolver()))
        .chooser(Box::new(DirectChooser))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(LimitedRetrier::new(ErrorRetrier, 3)))
        .build();

        let retried_times = Arc::new(Mutex::new(HashMap::<String, AtomicUsize>::new()));
        let err = always_malicious_client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_before_retry_delay(Box::new({
                let retried_times = retried_times.to_owned();
                move |context, _| {
                    retried_times
                        .lock()
                        .unwrap()
                        .entry(context.url().to_string())
                        .and_modify(|t| {
                            t.fetch_add(1, Relaxed);
                        })
                        .or_insert_with(|| AtomicUsize::new(1));
                    true
                }
            }))
            .call()
            .unwrap_err();
        assert_eq!(err.kind(), ResponseErrorKind::MaliciousResponse);

        let urls = [
            "https://fakedomain.withoutport.com/",
            "https://fakedomain.withport.com:8080/",
            "https://192.168.1.1/",
            "https://[::ffff:192.10.2.255]/",
            "https://[::ffff:192.11.2.255]:8081/",
            "https://192.168.1.2:8080/",
        ];
        let retried_times = Arc::try_unwrap(retried_times)
            .unwrap()
            .into_inner()
            .unwrap();
        for &url in urls.iter() {
            assert_eq!(retried_times.get(url).map(|e| e.load(Relaxed)).unwrap(), 4);
        }

        Ok(())
    }

    #[test]
    fn test_call_unexpected_redirection() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let headers = {
            let mut headers = HeaderMap::new();
            headers.insert(
                HeaderName::from_static("location"),
                HeaderValue::from_static("https://another-fakedomain.withoutport.com/"),
            );
            headers.insert(
                HeaderName::from_static(X_REQ_ID_HEADER_NAME),
                HeaderValue::from_static("fake_req_id"),
            );
            headers
        };
        let always_redirected_client = make_fixed_response_client_builder(
            StatusCode::from_u16(301)?,
            headers,
            b"<p>Hello world!</p>".to_vec(),
        )
        .resolver(Box::new(make_random_resolver()))
        .chooser(Box::new(DirectChooser))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(LimitedRetrier::new(ErrorRetrier, 3)))
        .build();

        let err = always_redirected_client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_before_retry_delay(Box::new(|_, _| panic!("Should never retry")))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::UnexpectedStatusCode(StatusCode::from_u16(301)?)
        );

        Ok(())
    }

    #[test]
    fn test_call_callbacks() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let client = make_error_response_client_builder(
            HTTPResponseErrorKind::ConnectError,
            "Fake Connect Error",
        )
        .resolver(Box::new(make_dumb_resolver()))
        .chooser(Box::new(DirectChooser))
        .request_retrier(Box::new(ErrorRetrier))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .build();

        let err = client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_before_request_signed(Box::new(|_| false))
            .on_before_retry_delay(Box::new(|_, _| panic!("Should not retry")))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HTTPResponseErrorKind::UserCanceled)
        );

        let err = client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_after_request_signed(Box::new(|_| false))
            .on_before_retry_delay(Box::new(|_, _| panic!("Should not retry")))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HTTPResponseErrorKind::UserCanceled)
        );

        let err = client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_before_retry_delay(Box::new(|_, _| false))
            .on_after_retry_delay(Box::new(|_, _| panic!("Should not retry")))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HTTPResponseErrorKind::UserCanceled)
        );
        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "async")]
    async fn test_async_call_single_endpoint_retry() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let always_retry_client = make_error_response_client_builder(
            HTTPResponseErrorKind::TimeoutError,
            "Fake Timeout Error",
        )
        .resolver(Box::new(make_random_resolver()))
        .chooser(Box::new(DirectChooser))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(LimitedRetrier::new(ErrorRetrier, 3)))
        .build();

        let retried = Arc::new(AtomicUsize::new(0));
        let err = always_retry_client
            .post(ServiceName::Up, &single_up_domain_region())
            .on_before_retry_delay({
                let retried = retried.to_owned();
                Box::new(move |context, _| {
                    assert_eq!(
                        "https://fakedomain.withport.com:8080/",
                        &context.url().to_string()
                    );
                    let retried = retried.fetch_add(1, Relaxed) + 1;
                    assert_eq!(context.retried().retried_total(), retried);
                    assert_eq!(context.retried().retried_on_current_endpoint(), retried);
                    assert_eq!(context.retried().retried_on_current_ips(), retried);
                    assert_eq!(context.retried().abandoned_endpoints(), 0);
                    true
                })
            })
            .async_call()
            .await
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HTTPResponseErrorKind::TimeoutError)
        );

        let headers = {
            let mut headers = HeaderMap::default();
            headers.insert(
                HeaderName::from_static(X_REQ_ID_HEADER_NAME),
                HeaderValue::from_static("fake_req_id"),
            );
            headers
        };
        let always_throttled_client = make_fixed_response_client_builder(
            StatusCode::from_u16(509)?,
            headers.to_owned(),
            b"{\"error\":\"Fake Throttled Error\"}".to_vec(),
        )
        .resolver(Box::new(make_random_resolver()))
        .chooser(Box::new(DirectChooser))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(LimitedRetrier::new(ErrorRetrier, 3)))
        .build();

        retried.store(0, Relaxed);
        let err = always_throttled_client
            .post(ServiceName::Up, &single_up_domain_region())
            .on_before_retry_delay({
                let retried = retried.to_owned();
                Box::new(move |context, _| {
                    assert_eq!(
                        "https://fakedomain.withport.com:8080/",
                        &context.url().to_string(),
                    );
                    let retried = retried.fetch_add(1, Relaxed) + 1;
                    assert_eq!(context.retried().retried_total(), retried);
                    assert_eq!(context.retried().retried_on_current_endpoint(), retried);
                    assert_eq!(context.retried().retried_on_current_ips(), retried);
                    assert_eq!(context.retried().abandoned_endpoints(), 0);
                    true
                })
            })
            .async_call()
            .await
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::StatusCodeError(StatusCode::from_u16(509)?)
        );
        assert_eq!(&err.to_string(), "Fake Throttled Error");

        Ok(())
    }
}
