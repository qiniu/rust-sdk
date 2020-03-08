#[cfg(test)]
mod tests {
    use matches::matches;
    use qiniu_http::{ErrorKind, Method, Request, Response, ResponseBody, Result, RetryKind};
    use qiniu_ng::{
        http::{HTTPAfterAction, HTTPBeforeAction},
        storage::region::Region,
        ConfigBuilder,
    };
    use qiniu_test_utils::env;
    use std::{error::Error, result::Result as StdResult};

    struct HTTPBeforeActionTester;

    impl HTTPBeforeAction for HTTPBeforeActionTester {
        fn before_call(&self, request: &mut Request) -> Result<()> {
            assert!(request.url().starts_with("https://uc.qbox.me/v3/query"));
            assert_eq!(request.method(), Method::GET);
            assert_eq!(
                request.headers().get(&"Content-Type".into()),
                Some(&"application/x-www-form-urlencoded".into())
            );
            assert_eq!(
                request.headers().get(&"Accept".into()),
                Some(&"application/json".into())
            );
            assert!(request.body().is_empty());
            Ok(())
        }
    }

    #[test]
    fn test_config_http_request_before_action_handlers() -> StdResult<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .append_http_request_before_action_handler(HTTPBeforeActionTester)
            .build();
        Region::query("z0-bucket", env::get().access_key(), config)?;
        Ok(())
    }

    struct HTTPAfterActionTester;

    impl HTTPAfterAction for HTTPAfterActionTester {
        fn after_call(&self, _request: &mut Request, response: &mut Response) -> Result<()> {
            assert_eq!(response.status_code(), 200);
            assert!(response.headers().get(&"X-Reqid".into()).is_some());
            assert!(response.server_ip().is_some());
            assert_eq!(response.server_port(), 443);
            assert!(response.body_len().unwrap() > 0);

            *response.body_mut() = Some(ResponseBody::Bytes(b"{}".to_vec()));
            response.headers_mut().insert("Content-Length".into(), "2".into());
            Ok(())
        }
    }

    #[test]
    fn test_config_http_request_after_action_handlers() -> StdResult<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .append_http_request_after_action_handler(HTTPAfterActionTester)
            .build();
        let err = Region::query("z0-bucket", env::get().access_key(), config).unwrap_err();
        assert_eq!(err.retry_kind(), RetryKind::UnretryableError);
        assert!(!err.is_retry_safe());
        assert!(matches!(err.error_kind(), ErrorKind::JSONError(..)));
        assert!(err.request_id().is_some());
        assert_eq!(err.method(), Some(Method::GET));
        assert_eq!(
            err.url().as_ref().map(|u| u.starts_with("https://uc.qbox.me/v3/query")),
            Some(true)
        );
        Ok(())
    }
}
