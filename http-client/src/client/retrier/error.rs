use super::{
    super::{Idempotent, ResponseErrorKind},
    RequestRetrier, RequestRetrierOptions, RetryDecision, RetryResult,
};
use qiniu_http::{RequestParts as HttpRequestParts, ResponseErrorKind as HttpResponseErrorKind};

/// 根据七牛 API 返回的状态码作出重试决定
#[derive(Copy, Clone, Debug, Default)]
pub struct ErrorRetrier;

impl RequestRetrier for ErrorRetrier {
    fn retry(&self, request: &mut HttpRequestParts, opts: RequestRetrierOptions) -> RetryResult {
        return match opts.response_error().kind() {
            ResponseErrorKind::HttpError(http_err_kind) => match http_err_kind {
                HttpResponseErrorKind::ProtocolError => RetryDecision::RetryRequest,
                HttpResponseErrorKind::InvalidUrl => RetryDecision::TryNextServer,
                HttpResponseErrorKind::ConnectError => RetryDecision::TryNextServer,
                HttpResponseErrorKind::ProxyError => RetryDecision::RetryRequest,
                HttpResponseErrorKind::DnsServerError => RetryDecision::RetryRequest,
                HttpResponseErrorKind::UnknownHostError => RetryDecision::TryNextServer,
                HttpResponseErrorKind::SendError => RetryDecision::RetryRequest,
                HttpResponseErrorKind::ReceiveError | HttpResponseErrorKind::UnknownError => {
                    if is_idempotent(request, opts.idempotent()) {
                        RetryDecision::RetryRequest
                    } else {
                        RetryDecision::DontRetry
                    }
                }
                HttpResponseErrorKind::LocalIoError => RetryDecision::DontRetry,
                HttpResponseErrorKind::TimeoutError => RetryDecision::RetryRequest,
                HttpResponseErrorKind::ServerCertError => RetryDecision::TryAlternativeEndpoints,
                HttpResponseErrorKind::ClientCertError => RetryDecision::DontRetry,
                HttpResponseErrorKind::TooManyRedirect => RetryDecision::DontRetry,
                HttpResponseErrorKind::CallbackError => RetryDecision::DontRetry,
                _ => RetryDecision::RetryRequest,
            },
            ResponseErrorKind::UnexpectedStatusCode(_) => RetryDecision::DontRetry,
            ResponseErrorKind::StatusCodeError(status_code) => match status_code.as_u16() {
                0..=399 => panic!("Should not arrive here"),
                400..=501 | 579 | 599 | 608 | 612 | 614 | 616 | 618 | 630 | 631 | 632 | 640 | 701 => {
                    RetryDecision::DontRetry
                }
                509 | 573 => RetryDecision::Throttled,
                _ => RetryDecision::TryNextServer,
            },
            ResponseErrorKind::ParseResponseError | ResponseErrorKind::UnexpectedEof => {
                if is_idempotent(request, opts.idempotent()) {
                    RetryDecision::RetryRequest
                } else {
                    RetryDecision::DontRetry
                }
            }
            ResponseErrorKind::MaliciousResponse => RetryDecision::RetryRequest,
            ResponseErrorKind::NoTry | ResponseErrorKind::SystemCallError => RetryDecision::DontRetry,
        }
        .into();

        fn is_idempotent(request: &HttpRequestParts, idempotent: Idempotent) -> bool {
            match idempotent {
                Idempotent::Always => true,
                Idempotent::Default => request.method().is_safe(),
                Idempotent::Never => false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::super::{super::RetriedStatsInfo, ResponseError},
        *,
    };
    use qiniu_http::{Method as HttpMethod, Request as HttpRequest, Uri as HttpUri};
    use std::{convert::TryFrom, error::Error, result::Result};

    #[test]
    fn test_error_retrier_idempotent() -> Result<(), Box<dyn Error>> {
        let uri = HttpUri::try_from("http://localhost/abc")?;

        let retrier = ErrorRetrier;
        let (mut parts, _) = HttpRequest::builder()
            .url(uri.to_owned())
            .method(HttpMethod::GET)
            .body(())
            .build()
            .into_parts_and_body();
        let result = retrier.retry(
            &mut parts,
            RequestRetrierOptions::builder(
                &ResponseError::new_with_msg(HttpResponseErrorKind::ReceiveError.into(), "Test Error"),
                &RetriedStatsInfo::default(),
            )
            .build(),
        );
        assert_eq!(result.decision(), RetryDecision::RetryRequest);

        let result = retrier.retry(
            &mut parts,
            RequestRetrierOptions::builder(
                &ResponseError::new_with_msg(HttpResponseErrorKind::ReceiveError.into(), "Test Error"),
                &RetriedStatsInfo::default(),
            )
            .idempotent(Idempotent::Never)
            .build(),
        );
        assert_eq!(result.decision(), RetryDecision::DontRetry);

        let (mut parts, _) = HttpRequest::builder()
            .url(uri)
            .method(HttpMethod::POST)
            .body(())
            .build()
            .into_parts_and_body();
        let result = retrier.retry(
            &mut parts,
            RequestRetrierOptions::builder(
                &ResponseError::new_with_msg(HttpResponseErrorKind::ReceiveError.into(), "Test Error"),
                &RetriedStatsInfo::default(),
            )
            .build(),
        );
        assert_eq!(result.decision(), RetryDecision::DontRetry);

        let result = retrier.retry(
            &mut parts,
            RequestRetrierOptions::builder(
                &ResponseError::new_with_msg(HttpResponseErrorKind::ReceiveError.into(), "Test Error"),
                &RetriedStatsInfo::default(),
            )
            .idempotent(Idempotent::Always)
            .build(),
        );
        assert_eq!(result.decision(), RetryDecision::RetryRequest);

        let result = retrier.retry(
            &mut parts,
            RequestRetrierOptions::builder(
                &ResponseError::new_with_msg(HttpResponseErrorKind::InvalidUrl.into(), "Test Error"),
                &RetriedStatsInfo::default(),
            )
            .idempotent(Idempotent::Always)
            .build(),
        );
        assert_eq!(result.decision(), RetryDecision::TryNextServer);

        Ok(())
    }

    #[test]
    fn test_error_retrier_retries() -> Result<(), Box<dyn Error>> {
        let uri = HttpUri::try_from("http://localhost/abc")?;

        let retrier = ErrorRetrier;
        let mut retried = RetriedStatsInfo::default();
        retried.increase_current_endpoint();
        retried.increase_current_endpoint();

        let (mut parts, _) = HttpRequest::builder()
            .url(uri)
            .method(HttpMethod::GET)
            .body(())
            .build()
            .into_parts_and_body();
        let result = retrier.retry(
            &mut parts,
            RequestRetrierOptions::builder(
                &ResponseError::new_with_msg(HttpResponseErrorKind::ReceiveError.into(), "Test Error"),
                &retried,
            )
            .build(),
        );
        assert_eq!(result.decision(), RetryDecision::RetryRequest);

        retried.switch_endpoint();

        let result = retrier.retry(
            &mut parts,
            RequestRetrierOptions::builder(
                &ResponseError::new_with_msg(HttpResponseErrorKind::ReceiveError.into(), "Test Error"),
                &retried,
            )
            .build(),
        );
        assert_eq!(result.decision(), RetryDecision::RetryRequest);

        Ok(())
    }
}
