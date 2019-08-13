#[cfg(test)]
mod tests {
    use http::{header, StatusCode};
    use qiniu::{
        config::Config,
        http::{Headers, Method, Request},
    };
    use std::default::Default;

    #[test]
    fn test_call() {
        let config: Config = Default::default();
        let resp = config
            .http_request_call()
            .call(Request::new(
                Method::GET,
                "http://up.qiniup.com",
                Headers::new(),
                None,
            ))
            .unwrap();
        assert_eq!(resp.status_code(), &StatusCode::METHOD_NOT_ALLOWED.as_u16());
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE.as_str()).unwrap(),
            &"application/json"
        );
        assert!(resp.headers().contains_key("x-reqid"));
    }
}
