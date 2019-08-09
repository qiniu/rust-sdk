#[cfg(test)]
mod tests {
    use http::{header, Request, StatusCode};
    use qiniu::config::Config;
    use std::default::Default;

    #[test]
    fn test_call() {
        let config: Config = Default::default();
        let resp = config
            .http_request_call()
            .call(
                Request::get("http://up.qiniup.com")
                    .body(Vec::new())
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            &"application/json"
        );
        assert!(resp.headers().contains_key("X-Reqid"));
    }
}
