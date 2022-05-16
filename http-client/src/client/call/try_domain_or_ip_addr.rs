use super::{
    super::{
        super::{DomainWithPort, IpAddrWithPort},
        ChooserFeedback, InnerRequestParts, RetriedStatsInfo, SimplifiedCallbackContext, SyncResponse,
    },
    domain_or_ip_addr::DomainOrIpAddr,
    error::{TryError, TryErrorWithExtensions},
    send_http_request::send_http_request,
    utils::{
        call_after_request_signed_callbacks, call_before_request_signed_callbacks, extract_ips_from, make_request,
        make_url, reset_request_body, sign_request,
    },
};
use qiniu_http::{
    Extensions, HeaderName, HeaderValue, Metrics, OnHeaderCallback, OnProgressCallback, OnStatusCodeCallback,
    Request as HttpRequest, RequestParts as HttpRequestParts, StatusCode, SyncRequestBody, TransferProgressInfo,
};
use std::mem::take;

pub(super) fn try_domain_or_ip_addr(
    domain_or_ip: &DomainOrIpAddr,
    parts: &InnerRequestParts<'_>,
    body: &mut SyncRequestBody<'_>,
    mut extensions: Extensions,
    retried: &mut RetriedStatsInfo,
) -> Result<SyncResponse, TryErrorWithExtensions> {
    let (url, resolved_ips) =
        make_url(domain_or_ip, parts, retried).map_err(|err| err.with_extensions(take(&mut extensions)))?;
    let on_uploading_progress = |info: TransferProgressInfo<'_>| parts.call_uploading_progress_callbacks(parts, info);
    let on_receive_response_status =
        |status_code: StatusCode| parts.call_receive_response_status_callbacks(parts, status_code);
    let on_receive_response_header =
        |name: &HeaderName, value: &HeaderValue| parts.call_receive_response_header_callbacks(parts, name, value);
    let mut http_request: HttpRequest<SyncRequestBody> =
        make_request(url, parts, body.into(), extensions, &resolved_ips);
    reset_request_body(http_request.body_mut(), retried).map_err(|err| err.with_request(&mut http_request))?;
    call_before_request_signed_callbacks(parts, &mut http_request, retried)
        .map_err(|err| err.with_request(&mut http_request))?;
    sign_request(&mut http_request, parts.authorization(), retried)
        .map_err(|err| err.with_request(&mut http_request))?;
    call_after_request_signed_callbacks(parts, &mut http_request, retried)
        .map_err(|err| err.with_request(&mut http_request))?;
    reset_request_body(http_request.body_mut(), retried).map_err(|err| err.with_request(&mut http_request))?;

    if parts.uploading_progress_callbacks_count() > 0 {
        *http_request.on_uploading_progress_mut() = Some(OnProgressCallback::reference(&on_uploading_progress));
    }
    if parts.receive_response_status_callbacks_count() > 0 {
        *http_request.on_receive_response_status_mut() =
            Some(OnStatusCodeCallback::reference(&on_receive_response_status));
    }
    if parts.receive_response_header_callbacks_count() > 0 {
        *http_request.on_receive_response_header_mut() = Some(OnHeaderCallback::reference(&on_receive_response_header));
    }

    let (extracted_ips, domain) = extract_ips_from(domain_or_ip);
    match send_http_request(&mut http_request, parts, retried) {
        Ok(response) => {
            if !extracted_ips.is_empty() {
                parts.http_client().chooser().feedback(make_positive_feedback(
                    &extracted_ips,
                    domain,
                    &mut http_request,
                    response.metrics(),
                    retried,
                ));
            }
            Ok(response)
        }
        Err(err) => {
            if !extracted_ips.is_empty() {
                parts.http_client().chooser().feedback(make_negative_feedback(
                    &extracted_ips,
                    domain,
                    &mut http_request,
                    &err,
                    retried,
                ));
            }
            Err(err.with_request(&mut http_request))
        }
    }
}

fn make_positive_feedback<'f>(
    ips: &'f [IpAddrWithPort],
    domain: Option<&'f DomainWithPort>,
    parts: &'f mut HttpRequestParts,
    metrics: Option<&'f Metrics>,
    retried: &'f RetriedStatsInfo,
) -> ChooserFeedback<'f> {
    let mut builder = ChooserFeedback::builder(ips);
    builder.retried(retried).extensions(parts.extensions_mut());
    if let Some(domain) = domain {
        builder.domain(domain);
    }
    if let Some(metrics) = metrics {
        builder.metrics(metrics);
    }
    builder.build()
}

fn make_negative_feedback<'f>(
    ips: &'f [IpAddrWithPort],
    domain: Option<&'f DomainWithPort>,
    parts: &'f mut HttpRequestParts,
    err: &'f TryError,
    retried: &'f RetriedStatsInfo,
) -> ChooserFeedback<'f> {
    let mut builder = ChooserFeedback::builder(ips);
    builder.retried(retried).extensions(parts.extensions_mut());
    if let Some(domain) = domain {
        builder.domain(domain);
    }
    if let Some(metrics) = err.response_error().metrics() {
        builder.metrics(metrics);
    }
    if let Some(error) = err.feedback_response_error() {
        builder.error(error);
    }
    builder.build()
}

#[cfg(feature = "async")]
use super::{
    super::super::{AsyncRequestBody, AsyncResponse},
    send_http_request::async_send_http_request,
    utils::{reset_async_request_body, sign_async_request},
};

#[cfg(feature = "async")]
pub(super) async fn async_try_domain_or_ip_addr(
    domain_or_ip: &DomainOrIpAddr,
    parts: &InnerRequestParts<'_>,
    body: &mut AsyncRequestBody<'_>,
    mut extensions: Extensions,
    retried: &mut RetriedStatsInfo,
) -> Result<AsyncResponse, TryErrorWithExtensions> {
    let (url, resolved_ips) =
        make_url(domain_or_ip, parts, retried).map_err(|err| err.with_extensions(take(&mut extensions)))?;
    let on_uploading_progress = |info: TransferProgressInfo<'_>| parts.call_uploading_progress_callbacks(parts, info);
    let on_receive_response_status =
        |status_code: StatusCode| parts.call_receive_response_status_callbacks(parts, status_code);
    let on_receive_response_header =
        |name: &HeaderName, value: &HeaderValue| parts.call_receive_response_header_callbacks(parts, name, value);
    let mut http_request: HttpRequest<AsyncRequestBody> =
        make_request(url, parts, body.into(), extensions, &resolved_ips);
    reset_async_request_body(http_request.body_mut(), retried)
        .await
        .map_err(|err| err.with_request(&mut http_request))?;
    call_before_request_signed_callbacks(parts, &mut http_request, retried)
        .map_err(|err| err.with_request(&mut http_request))?;
    sign_async_request(&mut http_request, parts.authorization(), retried)
        .await
        .map_err(|err| err.with_request(&mut http_request))?;
    call_after_request_signed_callbacks(parts, &mut http_request, retried)
        .map_err(|err| err.with_request(&mut http_request))?;
    reset_async_request_body(http_request.body_mut(), retried)
        .await
        .map_err(|err| err.with_request(&mut http_request))?;

    if parts.uploading_progress_callbacks_count() > 0 {
        *http_request.on_uploading_progress_mut() = Some(OnProgressCallback::reference(&on_uploading_progress));
    }
    if parts.receive_response_status_callbacks_count() > 0 {
        *http_request.on_receive_response_status_mut() =
            Some(OnStatusCodeCallback::reference(&on_receive_response_status));
    }
    if parts.receive_response_header_callbacks_count() > 0 {
        *http_request.on_receive_response_header_mut() = Some(OnHeaderCallback::reference(&on_receive_response_header));
    }

    let (extracted_ips, domain) = extract_ips_from(domain_or_ip);
    match async_send_http_request(&mut http_request, parts, retried).await {
        Ok(response) => {
            if !extracted_ips.is_empty() {
                parts
                    .http_client()
                    .chooser()
                    .async_feedback(make_positive_feedback(
                        &extracted_ips,
                        domain,
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
                parts
                    .http_client()
                    .chooser()
                    .async_feedback(make_negative_feedback(
                        &extracted_ips,
                        domain,
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
