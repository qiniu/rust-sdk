mod domain_or_ip_addr;
mod error;
mod utils;

use super::{
    super::{DomainWithPort, Endpoint},
    APIResult, Authorization, AuthorizationError, ChooserFeedback, ChosenResult, Request,
    RequestInfo, RequestWithoutEndpoints, ResponseError, ResponseErrorKind, ResponseInfo,
    RetriedStatsInfo, RetryResult, SyncResponse,
};
pub use domain_or_ip_addr::DomainOrIpAddr;
use error::{ErrorResponseBody, TryError};
use qiniu_http::{
    HeaderName, HeaderValue, Request as HTTPRequest, ResponseErrorKind as HTTPResponseErrorKind,
    StatusCode,
};
use serde_json::from_slice as parse_json_from_slice;
use std::{result::Result, thread::sleep, time::Duration};
use utils::{
    call_after_request_signed_callbacks, call_after_retry_delay_callbacks,
    call_before_request_signed_callbacks, call_before_retry_delay_callbacks,
    call_domain_chosen_callbacks, call_error_callbacks, call_success_callbacks,
    call_to_choose_domain_callbacks, find_domains_with_port, find_ip_addr_with_port, make_request,
    make_url,
};

#[cfg(feature = "async")]
use {super::AsyncResponse, async_std::task::block_on, futures_timer::Delay as AsyncDelay};

const X_REQ_ID_HEADER_NAME: &str = "X-Reqid";

macro_rules! install_callbacks {
    ($request:expr, $request_info:expr, $built_request:ident) => {
        let on_uploading_progress = |uploaded: u64, total: u64| -> bool {
            $request.call_uploading_progress_callbacks(&$request_info, uploaded, total)
        };
        *$built_request.on_uploading_progress_mut() = Some(&on_uploading_progress);
        let on_downloading_progress = |downloaded: u64, total: u64| -> bool {
            $request.call_downloading_progress_callbacks(&$request_info, downloaded, total)
        };
        *$built_request.on_downloading_progress_mut() = Some(&on_downloading_progress);
        let on_send_request_body = |chunk: &[u8]| -> bool {
            $request.call_send_request_body_callbacks(&$request_info, chunk)
        };
        *$built_request.on_send_request_body_mut() = Some(&on_send_request_body);
        let on_receive_response_status = |status_code: StatusCode| -> bool {
            $request.call_receive_response_status_callbacks(&$request_info, status_code)
        };
        *$built_request.on_receive_response_status_mut() = Some(&on_receive_response_status);
        let on_receive_response_body = |chunk: &[u8]| -> bool {
            $request.call_receive_response_body_callbacks(&$request_info, chunk)
        };
        *$built_request.on_receive_response_body_mut() = Some(&on_receive_response_body);
        let on_receive_response_header = |name: &HeaderName, value: &HeaderValue| -> bool {
            $request.call_receive_response_header_callbacks(&$request_info, name, value)
        };
        *$built_request.on_receive_response_header_mut() = Some(&on_receive_response_header);
    };
}

macro_rules! create_request_call_fn {
    ($method_name:ident, $return_type:ident, $into_endpoints_method:ident, $sign_method:ident, $call_method:ident, $choose_method:ident, $choose_ips_method:ident, $feedback_method:ident, $sleep_method:path, $new_response_info:path, $block:ident, $blocking_block:ident $(, $async:ident)?) => {
        pub(super) $($async)? fn $method_name(request: Request<'_>) -> APIResult<$return_type> {
            let (request, into_endpoints, service_name) = request.split();
            let endpoints = $block!({ into_endpoints.$into_endpoints_method(service_name) })?;
            let mut retried = RetriedStatsInfo::default();

            return match $block!({ try_new_endpoints(endpoints.endpoints(), &request, &mut retried) }) {
                Ok(response) => Ok(response),
                Err(err)
                    if err.retry_result() == RetryResult::TryOldEndpoints
                        && !endpoints.old_endpoints().is_empty() =>
                {
                    retried.switch_to_old_endpoints();
                    $block!({ try_old_endpoints(endpoints.old_endpoints(), &request, &mut retried) })
                }
                Err(err) => Err(err.into_response_error()),
            };

            type TryResult = Result<$return_type, TryError>;

            #[inline]
            $($async)? fn try_new_endpoints(
                endpoints: &[Endpoint],
                request: &RequestWithoutEndpoints<'_>,
                retried: &mut RetriedStatsInfo,
            ) -> TryResult {
                $block!({ try_endpoints(endpoints, request, retried, true) })
            }

            #[inline]
            $($async)? fn try_old_endpoints(
                endpoints: &[Endpoint],
                request: &RequestWithoutEndpoints<'_>,
                retried: &mut RetriedStatsInfo,
            ) -> APIResult<$return_type> {
                $block!({ try_endpoints(endpoints, request, retried, false) }).map_err(|err| err.into_response_error())
            }

            $($async)? fn try_endpoints(
                endpoints: &[Endpoint],
                request: &RequestWithoutEndpoints<'_>,
                retried: &mut RetriedStatsInfo,
                is_endpoints_old: bool,
            ) -> TryResult {
                let mut last_error: Option<TryError> = None;
                macro_rules! try_endpoint {
                    ($endpoint:expr, $request:expr, $retried:expr, $is_endpoint_old:expr) => {
                        match $block!({ try_endpoint($endpoint, $request, $retried) }) {
                            Ok(response) => return Ok(response),
                            Err(err) => match err.retry_result() {
                                RetryResult::TryOldEndpoints if $is_endpoint_old => return Err(err),
                                RetryResult::DontRetry => {
                                    retried.increase_abandoned_ips_of_current_endpoint();
                                    return Err(err);
                                }
                                _ => {
                                    retried.increase_abandoned_ips_of_current_endpoint();
                                    last_error = Some(err);
                                }
                            },
                        }
                    };
                }

                let mut tried_count = 0usize;
                for domain_with_port in find_domains_with_port(endpoints) {
                    retried.switch_endpoint();
                    'within_domain: loop {
                        retried.switch_ips();
                        match $block!({ choose_domain(request, domain_with_port, false) })? {
                            ChosenResult::IPs(ips) if !ips.is_empty() => {
                                tried_count += 1;
                                let domain =
                                    DomainOrIpAddr::new_from_domain(domain_with_port.to_owned(), ips);
                                try_endpoint!(&domain, request, retried, is_endpoints_old);
                            }
                            ChosenResult::TryAnotherDomain => {
                                retried.increase_abandoned_endpoints();
                                break 'within_domain;
                            }
                            _ /* ChosenResult::UseThisDomainDirectly or ChosenResult::IPs([]) */ => {
                                tried_count += 1;
                                let domain =
                                    DomainOrIpAddr::new_from_domain(domain_with_port.to_owned(), vec![]);
                                try_endpoint!(&domain, request, retried, is_endpoints_old);
                            }
                        }
                    }
                }

                let ips_with_port = find_ip_addr_with_port(endpoints)
                    .cloned()
                    .collect::<Vec<_>>();
                if !ips_with_port.is_empty() {
                    'within_ips: loop {
                        match $block!({ request.client().chooser().$choose_ips_method(&ips_with_port) }) {
                            ChosenResult::IPs(ips) if !ips.is_empty() => {
                                for ip in ips.into_iter() {
                                    retried.switch_endpoint();
                                    retried.switch_ips();
                                    tried_count += 1;
                                    let ip_addr = DomainOrIpAddr::from(ip);
                                    try_endpoint!(
                                        &ip_addr,
                                        request,
                                        retried,
                                        is_endpoints_old
                                    );
                                }
                            }
                            ChosenResult::UseThisDomainDirectly => {
                                panic!("choose_ips() must not returns ChosenResult::UseThisDomainDirectly")
                            }
                            _ => {
                                retried.increase_abandoned_endpoints();
                                break 'within_ips;
                            }
                        }
                    }
                }

                if tried_count == 0 {
                    'out_of_domains: for domain_with_port in find_domains_with_port(endpoints) {
                        retried.switch_endpoint();
                        'within_domain_2: loop {
                            retried.switch_ips();
                            match $block!({ choose_domain(request, domain_with_port, true) })? {
                                ChosenResult::IPs(ips) if !ips.is_empty() => {
                                    tried_count += 1;
                                    let domain =
                                        DomainOrIpAddr::new_from_domain(domain_with_port.to_owned(), ips);
                                    try_endpoint!(&domain, request, retried, is_endpoints_old);
                                    break 'out_of_domains;
                                }
                                ChosenResult::TryAnotherDomain => {
                                    retried.increase_abandoned_endpoints();
                                    break 'within_domain_2;
                                }
                                _ /* ChosenResult::UseThisDomainDirectly or ChosenResult::IPs([]) */ => {
                                    tried_count += 1;
                                    let domain =
                                        DomainOrIpAddr::new_from_domain(domain_with_port.to_owned(), vec![]);
                                    try_endpoint!(&domain, request, retried, is_endpoints_old);
                                    break 'out_of_domains;
                                }
                            }
                        }
                    }
                }

                if tried_count == 0 && !ips_with_port.is_empty() {
                    'within_ips_2: for ip in ips_with_port.into_iter() {
                        retried.switch_endpoint();
                        retried.switch_ips();
                        try_endpoint!(
                            &DomainOrIpAddr::from(ip),
                            request,
                            retried,
                            is_endpoints_old
                        );
                        break 'within_ips_2;
                    }
                }

                return Err(last_error.expect("No domains or IPs can be retried"));
            }

            $($async)? fn try_endpoint(
                domain_or_ip: &DomainOrIpAddr,
                request: &RequestWithoutEndpoints<'_>,
                retried: &mut RetriedStatsInfo,
            ) -> TryResult {
                let (url, resolved_ips) = make_url(domain_or_ip, request)?;
                let mut built_request = make_request(&url, request, &resolved_ips);
                call_before_request_signed_callbacks(request, &mut built_request, retried)?;
                $block!({ sign_request(&mut built_request, request.authorization()) })?;
                let request_info = RequestInfo::new(&built_request);
                install_callbacks!(request, request_info, built_request);
                call_after_request_signed_callbacks(request, &mut built_request, retried)?;
                match $block!({ do_request(request, &mut built_request, retried) }) {
                    Ok(response) => {
                        let feedback = ChooserFeedback::new(&domain_or_ip, retried, None);
                        $block!({ request.client().chooser().$feedback_method(feedback) });
                        Ok(response)
                    },
                    Err(err) => {
                        let feedback = ChooserFeedback::new(&domain_or_ip, retried, Some(err.response_error()));
                        $block!({ request.client().chooser().$feedback_method(feedback) });
                        Err(err)
                    }
                }
            }

            $($async)? fn do_request(
                request: &RequestWithoutEndpoints<'_>,
                built_request: &mut HTTPRequest<'_>,
                retried: &mut RetriedStatsInfo,
            ) -> TryResult {
                loop {
                    let response = $block!({
                        request
                            .client()
                            .http_caller()
                            .$call_method(&built_request)
                        })
                        .map_err(ResponseError::from)
                        .map($return_type::new)
                        .and_then(|response| $blocking_block!({ judge(response) }) )
                        .map_err(|response_error| {
                            let retry_result = request.client().request_retrier().retry(
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
                                        .client()
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
                                ResponseErrorKind::HTTPError(HTTPResponseErrorKind::InvalidURLError),
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
                    .get(&X_REQ_ID_HEADER_NAME.into())
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
                match response.status_code() {
                    0..=199 | 300..=399 => Err(ResponseError::new(
                        ResponseErrorKind::UnexpectedStatusCode(response.status_code()),
                        format!("status code {} is unexpected", response.status_code()),
                    )),
                    200..=299 => Ok(response),
            status_code => {
                let error_response_body: Vec<u8> = $block!({ response.fulfill() })
                    .map_err(|err| {
                        ResponseError::new(HTTPResponseErrorKind::LocalIOError.into(), err)
                    })?
                    .into_body()
                    .into_bytes();
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

            $($async)? fn choose_domain(
                request: &RequestWithoutEndpoints<'_>,
                domain_with_port: &DomainWithPort,
                ignore_frozen: bool,
            ) -> Result<ChosenResult, TryError> {
                call_to_choose_domain_callbacks(request, domain_with_port.domain())?;
                let result = $block!({
                    request
                        .client()
                        .chooser()
                        .$choose_method(domain_with_port, ignore_frozen)
                });
                call_domain_chosen_callbacks(request, domain_with_port.domain(), &result)?;
                Ok(result)
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
    choose,
    choose_ips,
    feedback,
    sleep,
    ResponseInfo::new_from_sync,
    sync_block,
    sync_block
);

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
    async_choose,
    async_choose_ips,
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
        credential::{CredentialProvider, StaticCredentialProvider},
        test_utils::{
            chaotic_up_domains_region, make_dumb_resolver, make_error_resolver,
            make_error_response_client_builder, make_fixed_response_client_builder,
            single_up_domain_region,
        },
        Authorization, Chooser, DefaultRetrier, ServiceName, SimpleChooser, NO_DELAY_POLICY,
    };
    use qiniu_http::HeadersOwned;
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
        let client = make_error_response_client_builder(
            HTTPResponseErrorKind::ConnectError,
            "Fake Connect Error",
        )
        .chooser(Box::new(SimpleChooser::new(
            make_dumb_resolver(),
            Duration::from_secs(10),
        )))
        .request_retrier(Box::new(DefaultRetrier::builder().retries(0).build()))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .build();

        let urls_visited = Arc::new(Mutex::new(Vec::new()));

        let err = client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_after_request_signed(Box::new({
                let urls_visited = urls_visited.to_owned();
                move |context| {
                    urls_visited
                        .lock()
                        .unwrap()
                        .push(context.request().url().to_owned());
                    true
                }
            }))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HTTPResponseErrorKind::ConnectError)
        );
        let urls_visited = Arc::try_unwrap(urls_visited).unwrap().into_inner().unwrap();
        assert_eq!(
            &urls_visited,
            &[
                "https://fakedomain.withoutport.com/".to_owned(),
                "https://fakedomain.withport.com:8080/".to_owned(),
                "https://192.168.1.1/".to_owned(),
                "https://[::ffff:c00a:2ff]/".to_owned(),
                "https://[::ffff:c00b:2ff]:8081/".to_owned(),
                "https://192.168.1.2:8080/".to_owned(),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_call_frozen_endpoints_selection() -> Result<(), Box<dyn Error>> {
        let err = ResponseError::new(
            HTTPResponseErrorKind::ConnectError.into(),
            "Fake Connect Error",
        );
        let chooser = SimpleChooser::new(make_dumb_resolver(), Duration::from_secs(10));
        chooser.feedback(ChooserFeedback::new(
            &DomainOrIpAddr::new_from_domain(
                DomainWithPort::new_with_port("fakedomain.withport.com", 8080),
                vec![],
            ),
            &RetriedStatsInfo::default(),
            Some(&err),
        ));
        chooser.feedback(ChooserFeedback::new(
            &DomainOrIpAddr::new_from_domain(
                DomainWithPort::new_with_port("fakedomain.withport.com", 8080),
                vec![
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
            ),
            &RetriedStatsInfo::default(),
            Some(&err),
        ));

        let client = make_error_response_client_builder(
            HTTPResponseErrorKind::ConnectError,
            "Fake Connect Error",
        )
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .chooser(Box::new(chooser))
        .request_retrier(Box::new(DefaultRetrier::builder().retries(0).build()))
        .build();

        let urls_visited = Arc::new(Mutex::new(Vec::new()));

        let err = client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_after_request_signed(Box::new({
                let urls_visited = urls_visited.to_owned();
                move |context| {
                    urls_visited
                        .lock()
                        .unwrap()
                        .push(context.request().url().to_owned());
                    true
                }
            }))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HTTPResponseErrorKind::ConnectError)
        );
        let urls_visited = Arc::try_unwrap(urls_visited).unwrap().into_inner().unwrap();
        assert_eq!(
            &urls_visited,
            &["https://fakedomain.withoutport.com/".to_owned(),]
        );
        Ok(())
    }

    #[test]
    fn test_call_switch_to_old_endpoints() -> Result<(), Box<dyn Error>> {
        let client =
            make_error_response_client_builder(HTTPResponseErrorKind::SSLError, "Fake SSL Error")
                .chooser(Box::new(SimpleChooser::new(
                    make_error_resolver(HTTPResponseErrorKind::SSLError.into(), "Fake SSL Error"),
                    Duration::from_secs(10),
                )))
                .retry_delay_policy(Box::new(NO_DELAY_POLICY))
                .request_retrier(Box::new(DefaultRetrier::builder().retries(0).build()))
                .build();

        let urls_visited = Arc::new(Mutex::new(Vec::new()));
        let retried = Arc::new(AtomicUsize::new(0));

        let err = client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_before_retry_delay(Box::new(|_, _| panic!("Should not retry")))
            .on_after_request_signed(Box::new({
                let urls_visited = urls_visited.to_owned();
                let retried = retried.to_owned();
                move |context| {
                    let retried = retried.fetch_add(1, Relaxed);
                    urls_visited
                        .lock()
                        .unwrap()
                        .push(context.request().url().to_owned());
                    assert_eq!(context.retried().retried_total(), retried);
                    assert_eq!(context.retried().retried_on_current_endpoint(), 0);
                    assert_eq!(context.retried().retried_on_current_ips(), 0);
                    assert_eq!(
                        context.retried().abandoned_endpoints(),
                        retried.saturating_sub(1).min(2)
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
        let urls_visited = Arc::try_unwrap(urls_visited).unwrap().into_inner().unwrap();
        assert_eq!(
            &urls_visited,
            &[
                "https://fakedomain.withoutport.com/".to_owned(),
                "https://old_fakedomain.withoutport.com/".to_owned(),
                "https://old_fakedomain.withport.com:8080/".to_owned(),
                "https://192.168.2.1/".to_owned(),
                "https://[::ffff:d00a:2ff]/".to_owned(),
                "https://[::ffff:d00b:2ff]:8081/".to_owned(),
                "https://192.168.2.2:8080/".to_owned(),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_call_single_endpoint_retry() -> Result<(), Box<dyn Error>> {
        let always_retry_client = make_error_response_client_builder(
            HTTPResponseErrorKind::TimeoutError,
            "Fake Timeout Error",
        )
        .chooser(Box::new(SimpleChooser::new(
            make_error_resolver(
                HTTPResponseErrorKind::TimeoutError.into(),
                "Fake Timeout Error",
            ),
            Duration::from_secs(10),
        )))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(DefaultRetrier::builder().retries(3).build()))
        .build();

        let retried = Arc::new(AtomicUsize::new(0));
        let err = always_retry_client
            .post(ServiceName::Up, &single_up_domain_region())
            .on_before_retry_delay({
                let retried = retried.to_owned();
                Box::new(move |context, _| {
                    assert_eq!(
                        "https://fakedomain.withport.com:8080/",
                        context.request().url()
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
            let mut headers = HeadersOwned::default();
            headers.insert(X_REQ_ID_HEADER_NAME.into(), "fake_req_id".into());
            headers
        };
        let always_throttled_client = make_fixed_response_client_builder(
            509,
            headers.to_owned(),
            b"{\"error\":\"Fake Throttled Error\"}".to_vec(),
        )
        .chooser(Box::new(SimpleChooser::new(
            make_dumb_resolver(),
            Duration::from_secs(10),
        )))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(DefaultRetrier::builder().retries(3).build()))
        .build();

        retried.store(0, Relaxed);
        let err = always_throttled_client
            .post(ServiceName::Up, &single_up_domain_region())
            .on_before_retry_delay({
                let retried = retried.to_owned();
                Box::new(move |context, _| {
                    assert_eq!(
                        "https://fakedomain.withport.com:8080/",
                        context.request().url()
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
        assert_eq!(err.kind(), ResponseErrorKind::StatusCodeError(509));
        assert_eq!(&err.to_string(), "Fake Throttled Error");

        Ok(())
    }

    #[test]
    fn test_call_retry_next() -> Result<(), Box<dyn Error>> {
        let always_try_next_client = make_error_response_client_builder(
            HTTPResponseErrorKind::UnknownHostError,
            "Test Unknown Host Error",
        )
        .chooser(Box::new(SimpleChooser::new(
            make_dumb_resolver(),
            Duration::from_secs(10),
        )))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(DefaultRetrier::builder().retries(3).build()))
        .build();

        let retried = Arc::new(AtomicUsize::new(0));
        let retry_urls = [
            "https://fakedomain.withoutport.com/".to_owned(),
            "https://fakedomain.withport.com:8080/".to_owned(),
            "https://192.168.1.1/".to_owned(),
            "https://[::ffff:c00a:2ff]/".to_owned(),
            "https://[::ffff:c00b:2ff]:8081/".to_owned(),
            "https://192.168.1.2:8080/".to_owned(),
        ];
        let err = always_try_next_client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_before_retry_delay({
                let retried = retried.to_owned();
                Box::new(move |context, _| {
                    let retried = retried.fetch_add(1, Relaxed);
                    assert_eq!(context.request().url(), retry_urls.get(retried).unwrap());
                    assert_eq!(context.retried().retried_total(), retried + 1);
                    assert_eq!(context.retried().retried_on_current_endpoint(), 1);
                    assert_eq!(context.retried().retried_on_current_ips(), 1);
                    assert_eq!(context.retried().abandoned_endpoints(), retried.min(2));
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
        let always_dont_retry_client = make_error_response_client_builder(
            HTTPResponseErrorKind::LocalIOError,
            "Test Local IO Error",
        )
        .build();

        let err = always_dont_retry_client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_before_retry_delay(Box::new(|_, _| panic!("Should never retry")))
            .on_after_request_signed(Box::new(|context| {
                assert_eq!(
                    context.request().url(),
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
        let always_retry_client =
            make_error_response_client_builder(HTTPResponseErrorKind::SendError, "Test Send Error")
                .chooser(Box::new(SimpleChooser::new(
                    make_error_resolver(HTTPResponseErrorKind::SendError.into(), "Fake Send Error"),
                    Duration::from_secs(10),
                )))
                .retry_delay_policy(Box::new(NO_DELAY_POLICY))
                .request_retrier(Box::new(DefaultRetrier::builder().retries(0).build()))
                .build();
        let credential: Arc<dyn CredentialProvider> = Arc::new(StaticCredentialProvider::new(
            "abcdefghklmnopq",
            "012345678901234567890",
        ));
        let signed_urls = Arc::new(Mutex::new(HashSet::new()));

        {
            let err = always_retry_client
                .post(ServiceName::Up, &chaotic_up_domains_region())
                .authorization(Authorization::v2(credential.to_owned()))
                .on_before_request_signed(Box::new(|context| {
                    assert!(context
                        .request()
                        .headers()
                        .get(&"Authorization".into())
                        .is_none());
                    true
                }))
                .on_after_request_signed(Box::new({
                    let signed_urls = signed_urls.to_owned();
                    move |context| {
                        signed_urls
                            .lock()
                            .unwrap()
                            .insert(context.request().url().to_owned());
                        assert!(context
                            .request()
                            .headers()
                            .get(&"Authorization".into())
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
                        .request()
                        .headers()
                        .get(&"Authorization".into())
                        .is_none());
                    true
                }))
                .on_after_request_signed(Box::new({
                    let signed_urls = signed_urls.to_owned();
                    move |context| {
                        signed_urls
                            .lock()
                            .unwrap()
                            .insert(context.request().url().to_owned());
                        assert!(context
                            .request()
                            .headers()
                            .get(&"Authorization".into())
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
        let always_malicious_client = make_fixed_response_client_builder(
            200,
            Default::default(),
            b"<p>Hello world!</p>".to_vec(),
        )
        .chooser(Box::new(SimpleChooser::new(
            make_dumb_resolver(),
            Duration::from_secs(10),
        )))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(DefaultRetrier::builder().retries(3).build()))
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
                        .entry(context.request().url().to_owned())
                        .and_modify(|t| {
                            t.fetch_add(1, Relaxed);
                        })
                        .or_insert(AtomicUsize::new(1));
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
            "https://[::ffff:c00a:2ff]/",
            "https://[::ffff:c00b:2ff]:8081/",
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
        let headers = {
            let mut headers = HeadersOwned::new();
            headers.insert(
                "Location".into(),
                "https://another-fakedomain.withoutport.com/".into(),
            );
            headers.insert(X_REQ_ID_HEADER_NAME.into(), "fake_req_id".into());
            headers
        };
        let always_redirected_client =
            make_fixed_response_client_builder(301, headers, b"<p>Hello world!</p>".to_vec())
                .chooser(Box::new(SimpleChooser::new(
                    make_dumb_resolver(),
                    Duration::from_secs(10),
                )))
                .retry_delay_policy(Box::new(NO_DELAY_POLICY))
                .request_retrier(Box::new(DefaultRetrier::builder().retries(3).build()))
                .build();

        let err = always_redirected_client
            .post(ServiceName::Up, &chaotic_up_domains_region())
            .on_before_retry_delay(Box::new(|_, _| panic!("Should never retry")))
            .call()
            .unwrap_err();
        assert_eq!(err.kind(), ResponseErrorKind::UnexpectedStatusCode(301),);

        Ok(())
    }

    #[test]
    fn test_call_callbacks() -> Result<(), Box<dyn Error>> {
        let client = make_error_response_client_builder(
            HTTPResponseErrorKind::ConnectError,
            "Fake Connect Error",
        )
        .chooser(Box::new(SimpleChooser::new(
            make_dumb_resolver(),
            Duration::from_secs(10),
        )))
        .request_retrier(Box::new(DefaultRetrier::builder().retries(0).build()))
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
        let always_retry_client = make_error_response_client_builder(
            HTTPResponseErrorKind::TimeoutError,
            "Fake Timeout Error",
        )
        .chooser(Box::new(SimpleChooser::new(
            make_error_resolver(
                HTTPResponseErrorKind::TimeoutError.into(),
                "Fake Timeout Error",
            ),
            Duration::from_secs(10),
        )))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(DefaultRetrier::builder().retries(3).build()))
        .build();

        let retried = Arc::new(AtomicUsize::new(0));
        let err = always_retry_client
            .post(ServiceName::Up, &single_up_domain_region())
            .on_before_retry_delay({
                let retried = retried.to_owned();
                Box::new(move |context, _| {
                    assert_eq!(
                        "https://fakedomain.withport.com:8080/",
                        context.request().url()
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
            let mut headers = HeadersOwned::default();
            headers.insert(X_REQ_ID_HEADER_NAME.into(), "fake_req_id".into());
            headers
        };
        let always_throttled_client = make_fixed_response_client_builder(
            509,
            headers.to_owned(),
            b"{\"error\":\"Fake Throttled Error\"}".to_vec(),
        )
        .chooser(Box::new(SimpleChooser::new(
            make_dumb_resolver(),
            Duration::from_secs(10),
        )))
        .retry_delay_policy(Box::new(NO_DELAY_POLICY))
        .request_retrier(Box::new(DefaultRetrier::builder().retries(3).build()))
        .build();

        retried.store(0, Relaxed);
        let err = always_throttled_client
            .post(ServiceName::Up, &single_up_domain_region())
            .on_before_retry_delay({
                let retried = retried.to_owned();
                Box::new(move |context, _| {
                    assert_eq!(
                        "https://fakedomain.withport.com:8080/",
                        context.request().url()
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
        assert_eq!(err.kind(), ResponseErrorKind::StatusCodeError(509));
        assert_eq!(&err.to_string(), "Fake Throttled Error");

        Ok(())
    }
}
