#[cfg(test)]
mod tests {
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
            .call(&Request::new(Method::GET, "http://up.qiniup.com", Headers::new(), None))
            .unwrap();
        assert_eq!(resp.status_code(), &405);
        assert_eq!(resp.headers().get("Content-Type").unwrap(), &"application/json");
        assert!(resp.headers().contains_key("X-Reqid"));
    }
}
