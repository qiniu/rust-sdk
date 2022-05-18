use super::{
    super::{
        BackoffOptions, InnerRequestParts, RequestRetrierOptions, ResponseError, RetriedStatsInfo, RetryDecision,
        SimplifiedCallbackContext, SyncResponse,
    },
    error::TryError,
    utils::{
        call_after_backoff_callbacks, call_before_backoff_callbacks, call_error_callbacks, call_response_callbacks,
        judge,
    },
};
use qiniu_http::{RequestParts as HttpRequestParts, SyncRequest as SyncHttpRequest};
use std::{result::Result, thread::sleep, time::Duration};

pub(super) fn send_http_request(
    http_request: &mut SyncHttpRequest<'_>,
    parts: &InnerRequestParts<'_>,
    retried: &mut RetriedStatsInfo,
) -> Result<SyncResponse, TryError> {
    loop {
        let response = parts
            .http_client()
            .http_caller()
            .call(http_request)
            .map_err(ResponseError::from)
            .and_then(|response| {
                call_response_callbacks(parts, http_request, retried, response.parts()).map(|_| response)
            })
            .map(SyncResponse::from)
            .and_then(|response| judge(response, retried))
            .map_err(|err| handle_response_error(err, http_request, parts, retried));
        match response {
            Ok(response) => {
                return Ok(response);
            }
            Err(err) => {
                call_error_callbacks(parts, http_request, retried, err.response_error())?;
                if need_backoff(&err) {
                    backoff(http_request, parts, retried, &err)?;
                    if need_retry_after_backoff(&err) {
                        continue;
                    }
                }
                return Err(err);
            }
        }
    }

    fn backoff(
        http_request: &mut SyncHttpRequest<'_>,
        parts: &InnerRequestParts<'_>,
        retried: &mut RetriedStatsInfo,
        err: &TryError,
    ) -> Result<(), TryError> {
        let delay = parts
            .http_client()
            .backoff()
            .time(
                http_request,
                BackoffOptions::builder(err.response_error(), retried)
                    .retry_decision(err.retry_decision())
                    .build(),
            )
            .duration();
        call_before_backoff_callbacks(parts, http_request, retried, delay)?;
        if delay > Duration::new(0, 0) {
            sleep(delay);
        }
        call_after_backoff_callbacks(parts, http_request, retried, delay)?;
        Ok(())
    }
}

fn need_backoff(err: &TryError) -> bool {
    matches!(
        err.retry_decision(),
        RetryDecision::RetryRequest | RetryDecision::Throttled | RetryDecision::TryNextServer
    )
}

fn need_retry_after_backoff(err: &TryError) -> bool {
    matches!(
        err.retry_decision(),
        RetryDecision::RetryRequest | RetryDecision::Throttled
    )
}

fn handle_response_error(
    response_error: ResponseError,
    http_parts: &mut HttpRequestParts,
    parts: &InnerRequestParts<'_>,
    retried: &mut RetriedStatsInfo,
) -> TryError {
    let retry_result = parts.http_client().request_retrier().retry(
        http_parts,
        RequestRetrierOptions::builder(&response_error, retried)
            .idempotent(parts.idempotent())
            .build(),
    );
    retried.increase_current_endpoint();
    TryError::new(response_error, retry_result)
}

#[cfg(feature = "async")]
mod async_send {
    use super::{
        super::{super::AsyncResponse, utils::async_judge},
        *,
    };
    use futures_timer::Delay as AsyncDelay;
    use qiniu_http::AsyncRequest as AsyncHttpRequest;

    pub(in super::super) async fn async_send_http_request(
        http_request: &mut AsyncHttpRequest<'_>,
        parts: &InnerRequestParts<'_>,
        retried: &mut RetriedStatsInfo,
    ) -> Result<AsyncResponse, TryError> {
        loop {
            let mut response = parts
                .http_client()
                .http_caller()
                .async_call(http_request)
                .await
                .map_err(ResponseError::from)
                .and_then(|response| {
                    call_response_callbacks(parts, http_request, retried, response.parts()).map(|_| response)
                })
                .map(AsyncResponse::from);
            if let Ok(resp) = response {
                response = async_judge(resp, retried).await
            };
            let response = response.map_err(|err| handle_response_error(err, http_request, parts, retried));
            match response {
                Ok(response) => {
                    return Ok(response);
                }
                Err(err) => {
                    call_error_callbacks(parts, http_request, retried, err.response_error())?;
                    if need_backoff(&err) {
                        backoff(http_request, parts, retried, &err).await?;
                        if need_retry_after_backoff(&err) {
                            continue;
                        }
                    }
                    return Err(err);
                }
            }
        }

        async fn backoff(
            http_request: &mut AsyncHttpRequest<'_>,
            parts: &InnerRequestParts<'_>,
            retried: &mut RetriedStatsInfo,
            err: &TryError,
        ) -> Result<(), TryError> {
            let delay = parts
                .http_client()
                .backoff()
                .time(
                    http_request,
                    BackoffOptions::builder(err.response_error(), retried)
                        .retry_decision(err.retry_decision())
                        .build(),
                )
                .duration();
            call_before_backoff_callbacks(parts, http_request, retried, delay)?;
            if delay > Duration::new(0, 0) {
                async_sleep(delay).await;
            }
            call_after_backoff_callbacks(parts, http_request, retried, delay)?;
            Ok(())
        }

        async fn async_sleep(dur: Duration) {
            AsyncDelay::new(dur).await
        }
    }
}

#[cfg(feature = "async")]
pub(super) use async_send::*;
