use delegate::delegate;
use qiniu_http::{
    response::Body as HTTPResponseBody, Error as HTTPError, HeaderValue, Headers, Method, Response as HTTPResponse,
    Result as HTTPResult, StatusCode,
};
use serde::de::DeserializeOwned;
use std::fmt;

pub struct Response<'a> {
    pub(super) inner: HTTPResponse,
    pub(super) method: Method,
    pub(super) host: &'a str,
    pub(super) path: &'a str,
}

impl<'a> Response<'a> {
    delegate! {
        target self.inner {
            pub fn status_code(&self) -> StatusCode;
            pub fn headers(&self) -> &Headers;
            pub fn into_parts(self) -> (StatusCode, Headers, Option<HTTPResponseBody>);
            pub fn into_body(self) -> Option<HTTPResponseBody>;
            pub fn take_body(&mut self) -> Option<HTTPResponseBody>;
        }
    }

    pub fn body(&self) -> Option<&HTTPResponseBody> {
        self.inner.body().as_ref()
    }

    pub fn header<HeaderNameT: AsRef<str>>(&self, header_name: HeaderNameT) -> Option<&HeaderValue> {
        self.inner.headers().get(header_name.as_ref())
    }

    pub fn parse_json<T: DeserializeOwned>(mut self) -> Option<HTTPResult<T>> {
        self.take_body().map(|r| {
            serde_json::from_reader(r).map_err(|err| {
                HTTPError::new_unretryable_error_from_parts(
                    err,
                    Some(self.method),
                    Some(self.host.to_owned() + self.path),
                )
            })
        })
    }
}

impl fmt::Debug for Response<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Response")
            .field("inner", &self.inner)
            .field("method", &self.method)
            .field("host", &self.host)
            .field("path", &self.path)
            .finish()
    }
}
