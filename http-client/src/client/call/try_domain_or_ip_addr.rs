use super::{
    super::{
        super::IpAddrWithPort, ChooserFeedback, RequestParts, RetriedStatsInfo,
        SimplifiedCallbackContext, SyncResponse,
    },
    domain_or_ip_addr::DomainOrIpAddr,
    error::{TryError, TryErrorWithExtensions},
    send_http_request::send_http_request,
    utils::{
        call_after_request_signed_callbacks, call_before_request_signed_callbacks,
        extract_ips_from, make_request, make_url, sign_request,
    },
};
use qiniu_http::{
    Extensions, HeaderName, HeaderValue, Metrics, StatusCode, SyncRequest as SyncHttpRequest,
    SyncRequestBody, TransferProgressInfo,
};
use std::mem::take;

macro_rules! setup_callbacks {
    ($parts:ident, $http_request:ident) => {
        let on_uploading_progress = |info: &TransferProgressInfo| -> bool {
            $parts.call_uploading_progress_callbacks($parts, info)
        };
        if $parts.uploading_progress_callbacks_count() > 0 {
            *$http_request.on_uploading_progress_mut() = Some(&on_uploading_progress);
        }
        let on_receive_response_status = |status_code: StatusCode| -> bool {
            $parts.call_receive_response_status_callbacks($parts, status_code)
        };
        if $parts.receive_response_status_callbacks_count() > 0 {
            *$http_request.on_receive_response_status_mut() = Some(&on_receive_response_status);
        }
        let on_receive_response_header = |name: &HeaderName, value: &HeaderValue| -> bool {
            $parts.call_receive_response_header_callbacks($parts, name, value)
        };
        if $parts.receive_response_header_callbacks_count() > 0 {
            *$http_request.on_receive_response_header_mut() = Some(&on_receive_response_header);
        }
    };
}

pub(super) fn try_domain_or_ip_addr(
    domain_or_ip: &DomainOrIpAddr,
    parts: &RequestParts<'_>,
    body: &mut SyncRequestBody<'_>,
    mut extensions: Extensions,
    retried: &mut RetriedStatsInfo,
) -> Result<SyncResponse, TryErrorWithExtensions> {
    let (url, resolved_ips) =
        make_url(domain_or_ip, parts).map_err(|err| err.with_extensions(take(&mut extensions)))?;
    let mut http_request = make_request(
        url,
        parts,
        SyncRequestBody::from_referenced_reader(body, body.size()),
        extensions,
        &resolved_ips,
    );
    call_before_request_signed_callbacks(parts, &mut http_request, retried)
        .map_err(|err| err.with_request(&mut http_request))?;
    sign_request(&mut http_request, parts.authorization())
        .map_err(|err| err.with_request(&mut http_request))?;
    call_after_request_signed_callbacks(parts, &mut http_request, retried)
        .map_err(|err| err.with_request(&mut http_request))?;

    setup_callbacks!(parts, http_request);

    let extracted_ips = extract_ips_from(domain_or_ip);
    match send_http_request(&mut http_request, parts, retried) {
        Ok(response) => {
            if !extracted_ips.is_empty() {
                parts
                    .http_client()
                    .chooser()
                    .feedback(make_positive_feedback(
                        &extracted_ips,
                        &mut http_request,
                        response.metrics(),
                        retried,
                    ));
            }
            Ok(response)
        }
        Err(err) => {
            if !extracted_ips.is_empty() {
                parts
                    .http_client()
                    .chooser()
                    .feedback(make_negative_feedback(
                        &extracted_ips,
                        &mut http_request,
                        &err,
                        retried,
                    ));
            }
            Err(err.with_request(&mut http_request))
        }
    }
}

#[inline]
fn make_positive_feedback<'f>(
    ips: &'f [IpAddrWithPort],
    http_request: &'f mut SyncHttpRequest<'_>,
    metrics: Option<&'f dyn Metrics>,
    retried: &'f RetriedStatsInfo,
) -> ChooserFeedback<'f> {
    ChooserFeedback::new(ips, retried, http_request.extensions_mut(), metrics, None)
}

#[inline]
fn make_negative_feedback<'f>(
    ips: &'f [IpAddrWithPort],
    http_request: &'f mut SyncHttpRequest<'_>,
    err: &'f TryError,
    retried: &'f RetriedStatsInfo,
) -> ChooserFeedback<'f> {
    ChooserFeedback::new(
        ips,
        retried,
        http_request.extensions_mut(),
        err.response_error().metrics(),
        err.feedback_response_error(),
    )
}

#[cfg(feature = "async")]
mod async_try {
    use super::{
        super::{
            super::super::AsyncResponse, send_http_request::async_send_http_request,
            utils::sign_async_request,
        },
        *,
    };

    pub(in super::super) async fn async_try_domain_or_ip_addr(
        domain_or_ip: &DomainOrIpAddr,
        request_info: &RequestParts<'_>,
        mut extensions: Extensions,
        retried: &mut RetriedStatsInfo,
    ) -> Result<AsyncResponse, TryErrorWithExtensions> {
        let (url, resolved_ips) = make_url(domain_or_ip, request_info)
            .map_err(|err| err.with_extensions(take(&mut extensions)))?;
        let mut http_request = make_request(url, request_info, extensions, &resolved_ips);
        call_before_request_signed_callbacks(request_info, &mut http_request, retried)
            .map_err(|err| err.with_request(&mut http_request))?;
        sign_async_request(&mut http_request, request_info.authorization())
            .await
            .map_err(|err| err.with_request(&mut http_request))?;
        call_after_request_signed_callbacks(request_info, &mut http_request, retried)
            .map_err(|err| err.with_request(&mut http_request))?;

        setup_callbacks!(request_info, http_request);

        let extracted_ips = extract_ips_from(domain_or_ip);
        match async_send_http_request(&mut http_request, request_info, retried).await {
            Ok(response) => {
                if !extracted_ips.is_empty() {
                    request_info
                        .http_client()
                        .chooser()
                        .async_feedback(make_positive_feedback(
                            &extracted_ips,
                            &mut http_request,
                            response.metrics(),
                            retried,
                        ))
                        .await;
                }
                Ok(response)
            }
            Err(err) => {
                if !extracted_ips.is_empty() {
                    request_info
                        .http_client()
                        .chooser()
                        .async_feedback(make_negative_feedback(
                            &extracted_ips,
                            &mut http_request,
                            &err,
                            retried,
                        ))
                        .await;
                }
                Err(err.with_request(&mut http_request))
            }
        }
    }
}

#[cfg(feature = "async")]
pub(super) use async_try::*;
