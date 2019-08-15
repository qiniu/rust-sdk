use qiniu_http::{Error, HTTPCaller, Request, Response, ResponseBuilder, Result};
use std::{
    boxed::Box,
    default::Default,
    io::{self, Read},
    str::FromStr,
};

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
    fn call<'r>(&self, request: &Request<'r>) -> Result<Response> {
        let mut request_builder = self.inner.request(
            http::Method::from_str(request.method().as_str()).unwrap(),
            request.url(),
        );
        for (header_name, header_value) in request.headers().iter() {
            request_builder = request_builder.header(header_name, header_value);
        }
        if let Some(body) = request.body() {
            request_builder = request_builder.body(Vec::from(body));
        }
        match request_builder.build() {
            Ok(reqwest_request) => match self.inner.execute(reqwest_request) {
                Ok(reqwest_response) => {
                    let mut response_builder =
                        ResponseBuilder::default().status_code(reqwest_response.status().as_u16());
                    for (header_name, header_value) in reqwest_response.headers().iter() {
                        response_builder =
                            response_builder.header(header_name.as_str(), header_value.to_str().unwrap());
                    }
                    response_builder = response_builder.body(Box::new(reqwest_response) as Box<Read>);
                    Ok(response_builder.build())
                }
                Err(err) => {
                    let (method, url) = (Some(request.method().to_owned()), Some(request.url().to_owned()));
                    if let Some(err_ref) = err.get_ref() {
                        if err_ref.downcast_ref::<::http::Error>().is_some() {
                            return Err(Error::new_unretryable_error_from_parts(err, method, url));
                        } else if let Some(hyper_err) = err_ref.downcast_ref::<::hyper::Error>() {
                            if hyper_err.is_parse() || hyper_err.is_user() || hyper_err.is_canceled() {
                                return Err(Error::new_unretryable_error_from_parts(err, method, url));
                            } else if hyper_err.is_connect() {
                                return Err(Error::new_retryable_error_from_parts(err, true, method, url));
                            } else {
                                return Err(Error::new_retryable_error_from_parts(err, false, method, url));
                            }
                        } else if let Some(io_err) = err_ref.downcast_ref::<io::Error>() {
                            let retry_safe = match io_err.kind() {
                                io::ErrorKind::ConnectionRefused | io::ErrorKind::NotConnected => true,
                                _ => false,
                            };
                            return Err(Error::new_retryable_error_from_parts(err, retry_safe, method, url));
                        }
                    }
                    Err(Error::new_unretryable_error_from_parts(err, method, url))
                }
            },
            Err(err) => {
                let (method, url) = (request.method().to_owned(), request.url().to_owned());
                Err(Error::new_unretryable_error_from_parts(err, Some(method), Some(url)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{header, StatusCode};
    use qiniu_http::{Headers, Method};

    #[test]
    fn test_call() {
        let client = ReqwestClient::new(reqwest::Client::new());
        let resp = client
            .call(&Request::new(Method::GET, "http://up.qiniup.com", Headers::new(), None))
            .unwrap();
        assert_eq!(*resp.status_code(), StatusCode::METHOD_NOT_ALLOWED.as_u16());
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE.as_str()).unwrap(),
            &"application/json"
        );
        assert!(resp.headers().contains_key("x-reqid"));
    }
}
