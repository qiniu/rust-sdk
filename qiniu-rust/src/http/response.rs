use delegate::delegate;
use qiniu_http::{response::Body as HTTPResponseBody, HeaderValue, Headers, Response as HTTPResponse, StatusCode};
use serde::de::DeserializeOwned;
use std::fmt;

pub struct Response(pub(super) HTTPResponse);

impl Response {
    delegate! {
        target self.0 {
            pub fn status_code(&self) -> StatusCode;
            pub fn headers(&self) -> &Headers;
            pub fn into_parts(self) -> (StatusCode, Headers, Option<HTTPResponseBody>);
            pub fn into_body(self) -> Option<HTTPResponseBody>;
        }
    }
    pub fn body(&self) -> Option<&HTTPResponseBody> {
        self.0.body().as_ref()
    }

    pub fn header<HeaderNameT: AsRef<str>>(&self, header_name: HeaderNameT) -> Option<&HeaderValue> {
        self.0.headers().get(header_name.as_ref())
    }

    pub fn parse_json<T: DeserializeOwned>(self) -> Option<serde_json::Result<T>> {
        self.into_body().map(|r| serde_json::from_reader(r))
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
