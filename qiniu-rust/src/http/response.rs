use super::{
    Error as HTTPError, ErrorKind as HTTPErrorKind, HeaderName, HeaderValue, Headers, Method, Result as HTTPResult,
    StatusCode,
};
use delegate::delegate;
use getset::{CopyGetters, Getters};
use qiniu_http::{Response as HTTPResponse, ResponseBody as HTTPResponseBody};
use serde::de::DeserializeOwned;
use std::{
    borrow::Cow,
    fmt,
    io::{copy as io_copy, sink as io_sink, Read, Result as IOResult},
    net::IpAddr,
    result::Result,
};
use tap::{TapOps, TapOptionOps};

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
            #[allow(dead_code)]
            pub(crate) fn status_code(&self) -> StatusCode;
            #[allow(dead_code)]
            pub(crate) fn headers(&self) -> &Headers;
            #[allow(dead_code)]
            pub(crate) fn into_body(self) -> Option<HTTPResponseBody>;
            #[allow(dead_code)]
            pub(crate) fn take_body(&mut self) -> Option<HTTPResponseBody>;
            #[allow(dead_code)]
            pub(crate) fn clone_body(&mut self) -> IOResult<Option<HTTPResponseBody>>;
            #[allow(dead_code)]
            pub(crate) fn body_len(&mut self) -> IOResult<u64>;
            #[allow(dead_code)]
            pub(crate) fn server_ip(&self) -> Option<IpAddr>;
            #[allow(dead_code)]
            pub(crate) fn server_port(&self) -> u16;
        }
    }

    pub(crate) fn header<HeaderNameT: Into<HeaderName<'static>>>(
        &self,
        header_name: HeaderNameT,
    ) -> Option<&HeaderValue> {
        self.inner.headers().get(&header_name.into())
    }

    pub(crate) fn request_id(&self) -> Option<&str> {
        self.header("X-Reqid").map(|h| h.as_ref())
    }

    pub(crate) fn parse_json<T: DeserializeOwned>(&mut self) -> HTTPResult<T> {
        match self.take_body().unwrap() {
            HTTPResponseBody::Reader(reader) => serde_json::from_reader(reader),
            HTTPResponseBody::File(file) => serde_json::from_reader(file),
            HTTPResponseBody::Bytes(bytes) => serde_json::from_slice(&bytes),
        }
        .map_err(|err| {
            HTTPError::new_unretryable_error_from_parts(
                HTTPErrorKind::JSONError(err),
                Some(self.method),
                Some((self.base_url.to_owned() + self.path).into()),
            )
        })
    }

    pub(crate) fn try_parse_json<T: DeserializeOwned>(&mut self) -> Result<T, Vec<u8>> {
        let body = match self.take_body().unwrap() {
            HTTPResponseBody::Reader(mut reader) => Vec::new().tap(|buf| {
                let _ = reader.read(buf);
            }),
            HTTPResponseBody::File(mut file) => Vec::new().tap(|buf| {
                let _ = file.read(buf);
            }),
            HTTPResponseBody::Bytes(bytes) => bytes,
        };
        if self.header("Content-Type") != Some(&Cow::Borrowed("application/json")) {
            return Err(body);
        }
        serde_json::from_slice(&body).map_err(|_| body)
    }

    pub(crate) fn ignore_body(&mut self) {
        self.take_body().as_mut().tap_some(|r| match r {
            HTTPResponseBody::Reader(reader) => {
                let _ = io_copy(reader, &mut io_sink());
            }
            HTTPResponseBody::File(file) => {
                let _ = io_copy(file, &mut io_sink());
            }
            _ => {}
        });
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
