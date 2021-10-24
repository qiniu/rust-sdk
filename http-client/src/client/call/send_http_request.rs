use super::{
    super::{
        BackoffOptions, RequestRetrierOptions, RequestWithoutEndpoints, ResponseError,
        ResponseInfo, RetriedStatsInfo, RetryDecision, SimplifiedCallbackContext, SyncResponse,
    },
    error::TryError,
    utils::{
        call_after_backoff_callbacks, call_before_backoff_callbacks, call_error_callbacks,
        call_success_callbacks, judge,
    },
};
use qiniu_http::Request as HTTPRequest;
use std::{result::Result, thread::sleep, time::Duration};

pub(super) fn send_http_request(
    http_request: &mut HTTPRequest<'_>,
    request_info: &RequestWithoutEndpoints<'_>,
    retried: &mut RetriedStatsInfo,
) -> Result<SyncResponse, TryError> {
    loop {
        let response = request_info
            .http_client()
            .http_caller()
            .call(http_request)
            .map_err(ResponseError::from)
            .map(SyncResponse::new)
            .and_then(judge)
            .map_err(|err| handle_response_error(err, http_request, request_info, retried));
        match response {
            Ok(response) => {
                call_success_callbacks(
                    request_info,
                    http_request,
                    retried,
                    &ResponseInfo::new_from_sync(&response),
                )?;
                return Ok(response);
            }
            Err(err) => {
                call_error_callbacks(request_info, http_request, retried, err.response_error())?;
                if need_backoff(&err) {
                    backoff(http_request, request_info, retried, &err)?;
                    if need_retry_after_backoff(&err) {
                        continue;
                    }
                }
                return Err(err);
            }
        }
    }

    #[inline]
    fn backoff(
        http_request: &mut HTTPRequest<'_>,
        request_info: &RequestWithoutEndpoints<'_>,
        retried: &mut RetriedStatsInfo,
        err: &TryError,
    ) -> Result<(), TryError> {
        let delay = request_info
            .http_client()
            .backoff()
            .time(
                http_request,
                &BackoffOptions::new(err.retry_decision(), err.response_error(), retried),
            )
            .duration();
        call_before_backoff_callbacks(request_info, http_request, retried, delay)?;
        if delay > Duration::new(0, 0) {
            sleep(delay);
        }
        call_after_backoff_callbacks(request_info, http_request, retried, delay)?;
        Ok(())
    }
}

#[inline]
fn need_backoff(err: &TryError) -> bool {
    matches!(
        err.retry_decision(),
        RetryDecision::RetryRequest | RetryDecision::Throttled | RetryDecision::TryNextServer
    )
}

#[inline]
fn need_retry_after_backoff(err: &TryError) -> bool {
    matches!(
        err.retry_decision(),
        RetryDecision::RetryRequest | RetryDecision::Throttled
    )
}

#[inline]
fn handle_response_error(
    response_error: ResponseError,
    http_request: &mut HTTPRequest,
    request_info: &RequestWithoutEndpoints<'_>,
    retried: &mut RetriedStatsInfo,
) -> TryError {
    let retry_decision = request_info
        .http_client()
        .request_retrier()
        .retry(
            http_request,
            &RequestRetrierOptions::new(request_info.idempotent(), &response_error, retried),
        )
        .decision();
    retried.increase();
    TryError::new(response_error, retry_decision)
}

#[cfg(feature = "async")]
mod async_send {
    use super::{
        super::{super::AsyncResponse, utils::async_judge},
        *,
    };
    use async_std::task::block_on;
    use futures_timer::Delay as AsyncDelay;

    pub(in super::super) async fn async_send_http_request(
        http_request: &mut HTTPRequest<'_>,
        request_info: &RequestWithoutEndpoints<'_>,
        retried: &mut RetriedStatsInfo,
    ) -> Result<AsyncResponse, TryError> {
        loop {
            let response = request_info
                .http_client()
                .http_caller()
                .async_call(http_request)
                .await
                .map_err(ResponseError::from)
                .map(AsyncResponse::new)
                .and_then(|err| block_on(async { async_judge(err).await }))
                .map_err(|err| handle_response_error(err, http_request, request_info, retried));
            match response {
                Ok(response) => {
                    call_success_callbacks(
                        request_info,
                        http_request,
                        retried,
                        &ResponseInfo::new_from_async(&response),
                    )?;
                    return Ok(response);
                }
                Err(err) => {
                    call_error_callbacks(
                        request_info,
                        http_request,
                        retried,
                        err.response_error(),
                    )?;
                    if need_backoff(&err) {
                        backoff(http_request, request_info, retried, &err).await?;
                        if need_retry_after_backoff(&err) {
                            continue;
                        }
                    }
                    return Err(err);
                }
            }
        }

        #[inline]
        async fn backoff(
            http_request: &mut HTTPRequest<'_>,
            request_info: &RequestWithoutEndpoints<'_>,
            retried: &mut RetriedStatsInfo,
            err: &TryError,
        ) -> Result<(), TryError> {
            let delay = request_info
                .http_client()
                .backoff()
                .time(
                    http_request,
                    &BackoffOptions::new(err.retry_decision(), err.response_error(), retried),
                )
                .duration();
            call_before_backoff_callbacks(request_info, http_request, retried, delay)?;
            if delay > Duration::new(0, 0) {
                async_sleep(delay).await;
            }
            call_after_backoff_callbacks(request_info, http_request, retried, delay)?;
            Ok(())
        }

        #[inline]
        async fn async_sleep(dur: Duration) {
            AsyncDelay::new(dur).await
        }
    }
}

#[cfg(feature = "async")]
pub(super) use async_send::*;
