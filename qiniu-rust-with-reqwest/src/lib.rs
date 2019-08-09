use http::{Request, Response};
use qiniu_http::HTTPCaller;
use std::{boxed::Box, default::Default, error::Error, io::Read, result::Result};

pub struct ReqwestClient {
    inner: reqwest::Client,
}

impl ReqwestClient {
    fn new(client: reqwest::Client) -> ReqwestClient {
        ReqwestClient { inner: client }
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        ReqwestClient::new(reqwest::Client::new())
    }
}

impl HTTPCaller for ReqwestClient {
    fn call(&self, request: Request<Vec<u8>>) -> Result<Response<Box<Read>>, Box<Error>> {
        let resp = self
            .inner
            .request(request.method().to_owned(), &request.uri().to_string())
            .headers(request.headers().to_owned())
            .body(request.into_body())
            .send()?;
        let mut result_builder = Response::builder();
        result_builder.status(resp.status());
        result_builder.version(resp.version());
        for (header_name, header_value) in resp.headers().iter() {
            result_builder.header(header_name, header_value);
        }
        result_builder
            .body(Box::new(resp) as Box<Read>)
            .map_err(|err| err.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{header, StatusCode};

    #[test]
    fn test_call() {
        let client = ReqwestClient::new(reqwest::Client::new());
        let resp = client
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
