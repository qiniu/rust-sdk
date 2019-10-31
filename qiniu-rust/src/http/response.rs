use delegate::delegate;
use getset::{CopyGetters, Getters};
use qiniu_http::{
    Error as HTTPError, ErrorKind as HTTPErrorKind, HeaderValue, Headers, Method, Response as HTTPResponse,
    ResponseBody as HTTPResponseBody, Result as HTTPResult, StatusCode,
};
use serde::de::DeserializeOwned;
use std::{fmt, io, net::IpAddr};

#[derive(CopyGetters, Getters)]
pub(crate) struct Response<'a> {
    #[get = "pub(crate)"]
    pub(super) inner: HTTPResponse,
    #[get_copy = "pub(crate)"]
    pub(super) method: Method,
    #[get_copy = "pub(crate)"]
    pub(super) base_url: &'a str,
    #[get_copy = "pub(crate)"]
    pub(super) path: &'a str,
}

impl<'a> Response<'a> {
    delegate! {
        target self.inner {
            pub(crate) fn status_code(&self) -> StatusCode;
            pub(crate) fn headers(&self) -> &Headers;
            pub(crate) fn into_body(self) -> Option<HTTPResponseBody>;
            pub(crate) fn take_body(&mut self) -> Option<HTTPResponseBody>;
            pub(crate) fn server_ip(&self) -> Option<IpAddr>;
            pub(crate) fn server_port(&self) -> u16;
        }
    }

    pub(crate) fn header<HeaderNameT: AsRef<str>>(&self, header_name: HeaderNameT) -> Option<&HeaderValue> {
        self.inner.headers().get(header_name.as_ref())
    }

    pub(crate) fn request_id(&self) -> Option<&str> {
        self.header("X-Reqid").map(|h| h.as_ref())
    }

    pub(crate) fn parse_json<T: DeserializeOwned>(&mut self) -> HTTPResult<T> {
        let body = self.take_body().unwrap();
        serde_json::from_reader(body).map_err(|err| {
            HTTPError::new_unretryable_error_from_parts(
                HTTPErrorKind::JSONError(err),
                Some(self.method),
                Some((self.base_url.to_owned() + self.path).into()),
            )
        })
    }

    pub(crate) fn ignore_body(&mut self) {
        if let Some(r) = self.take_body().as_mut() {
            io::copy(r, &mut io::sink()).ok(); // Ignore read result
        }
    }
}

impl fmt::Debug for Response<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Response")
            .field("inner", &self.inner)
            .field("method", &self.method)
            .field("base_url", &self.base_url)
            .field("path", &self.path)
            .finish()
    }
}
