use super::{
    super::{
        super::Endpoint, request::SyncRequest, ApiResult, RequestParts, RetriedStatsInfo,
        RetryDecision, SyncRequestBody, SyncResponse,
    },
    error::TryErrorWithExtensions,
    ip_addrs_set::IpAddrsSet,
    try_endpoints::try_endpoints,
};
use qiniu_http::Extensions;

pub(in super::super) fn request_call(request: SyncRequest<'_>) -> ApiResult<SyncResponse> {
    let (parts, mut body, into_endpoints, service_name, extensions) = request.split();
    let endpoints = into_endpoints.into_endpoints(service_name)?;
    let mut tried_ips = IpAddrsSet::default();
    let mut retried = RetriedStatsInfo::default();

    return match try_preferred_endpoints(
        endpoints.preferred(),
        &parts,
        &mut body,
        extensions,
        &mut tried_ips,
        &mut retried,
    ) {
        Ok(response) => Ok(response),
        Err(err)
            if err.retry_decision() == RetryDecision::TryAlternativeEndpoints
                && !endpoints.alternative().is_empty() =>
        {
            let (_, extensions) = err.split();
            retried.switch_to_alternative_endpoints();
            try_alternative_endpoints(
                endpoints.alternative(),
                &parts,
                &mut body,
                extensions,
                &mut tried_ips,
                &mut retried,
            )
        }
        Err(err) => Err(err.into_response_error()),
    };

    #[inline]
    fn try_preferred_endpoints(
        endpoints: &[Endpoint],
        parts: &RequestParts<'_>,
        body: &mut SyncRequestBody<'_>,
        extensions: Extensions,
        tried_ips: &mut IpAddrsSet,
        retried: &mut RetriedStatsInfo,
    ) -> Result<SyncResponse, TryErrorWithExtensions> {
        try_endpoints(endpoints, parts, body, extensions, tried_ips, retried, true)
    }

    #[inline]
    fn try_alternative_endpoints(
        endpoints: &[Endpoint],
        parts: &RequestParts<'_>,
        body: &mut SyncRequestBody<'_>,
        extensions: Extensions,
        tried_ips: &mut IpAddrsSet,
        retried: &mut RetriedStatsInfo,
    ) -> ApiResult<SyncResponse> {
        try_endpoints(
            endpoints, parts, body, extensions, tried_ips, retried, false,
        )
        .map_err(|err| err.into_response_error())
    }
}

#[cfg(feature = "async")]
use super::{
    super::{request::AsyncRequest, AsyncRequestBody, AsyncResponse},
    try_endpoints::async_try_endpoints,
};

#[cfg(feature = "async")]
pub(in super::super) async fn async_request_call(
    request: AsyncRequest<'_>,
) -> ApiResult<AsyncResponse> {
    let (parts, mut body, into_endpoints, service_name, extensions) = request.split();
    let endpoints = into_endpoints.async_into_endpoints(service_name).await?;
    let mut tried_ips = IpAddrsSet::default();
    let mut retried = RetriedStatsInfo::default();

    return match try_preferred_endpoints(
        endpoints.preferred(),
        &parts,
        &mut body,
        extensions,
        &mut tried_ips,
        &mut retried,
    )
    .await
    {
        Ok(response) => Ok(response),
        Err(err)
            if err.retry_decision() == RetryDecision::TryAlternativeEndpoints
                && !endpoints.alternative().is_empty() =>
        {
            let (_, extensions) = err.split();
            retried.switch_to_alternative_endpoints();
            try_alternative_endpoints(
                endpoints.alternative(),
                &parts,
                &mut body,
                extensions,
                &mut tried_ips,
                &mut retried,
            )
            .await
        }
        Err(err) => Err(err.into_response_error()),
    };

    #[inline]
    async fn try_preferred_endpoints(
        endpoints: &[Endpoint],
        parts: &RequestParts<'_>,
        body: &mut AsyncRequestBody<'_>,
        extensions: Extensions,
        tried_ips: &mut IpAddrsSet,
        retried: &mut RetriedStatsInfo,
    ) -> Result<AsyncResponse, TryErrorWithExtensions> {
        async_try_endpoints(endpoints, parts, body, extensions, tried_ips, retried, true).await
    }

    #[inline]
    async fn try_alternative_endpoints(
        endpoints: &[Endpoint],
        parts: &RequestParts<'_>,
        body: &mut AsyncRequestBody<'_>,
        extensions: Extensions,
        tried_ips: &mut IpAddrsSet,
        retried: &mut RetriedStatsInfo,
    ) -> ApiResult<AsyncResponse> {
        async_try_endpoints(
            endpoints, parts, body, extensions, tried_ips, retried, false,
        )
        .await
        .map_err(|err| err.into_response_error())
    }
}
